use rand::Rng;
use std::collections::VecDeque;

use pathfinding::kuhn_munkres::kuhn_munkres_min;
use pathfinding::matrix::Matrix;

use command::Command;
use grid::droplet::{Blob, SimpleBlob};
use grid::Electrode;
use plan::Path;
use process::ProcessId;
use util::collections::{Map, Set};

#[cfg(feature = "pi")]
use pi::RaspberryPi;

use super::{Droplet, DropletId, DropletInfo, Grid, Location};

pub struct GridView {
    pub grid: Grid,
    completed: Vec<Snapshot>,
    planned: VecDeque<Snapshot>,
    pub done: bool,
    pub bad_edges: Set<(Location, Location)>,
    #[cfg(feature = "pi")]
    pub pi: Option<RaspberryPi>,
}

#[must_use]
#[derive(Debug, Default)]
pub struct Snapshot {
    pub droplets: Map<DropletId, Droplet>,
    pub commands_to_finalize: Vec<Box<dyn Command>>,
}

#[derive(Debug, PartialEq)]
pub enum DropletDiff {
    Disappeared,
    DidNotMove,
    Moved { from: Location, to: Location },
}

impl Snapshot {
    pub fn new_with_same_droplets(&self) -> Snapshot {
        let mut new_snapshot = Snapshot::default();
        new_snapshot.droplets = self.droplets.clone();

        // clear out the destination because we're doing to replan
        for d in new_snapshot.droplets.values_mut() {
            d.destination = None;
        }

        new_snapshot
    }

    #[cfg(not(feature = "pi"))]
    fn finalize(&mut self) {
        // we need to drain this so we can mutate the command without mutating
        // self, as we need to pass self into cmd.finalize
        // this feels pretty ugly....
        let mut x: Vec<_> = self.commands_to_finalize.drain(..).collect();
        for cmd in &mut x {
            debug!("Finalizing command: {:#?}", cmd);
            cmd.finalize(self)
        }
        self.commands_to_finalize = x;
    }

    #[cfg(feature = "pi")]
    fn finalize(&mut self, pi: Option<&mut RaspberryPi>) {
        // we need to drain this so we can mutate the command without mutating
        // self, as we need to pass self into cmd.finalize
        // this feels pretty ugly....
        let mut x: Vec<_> = self.commands_to_finalize.drain(..).collect();
        if let Some(pi) = pi {
            for cmd in &mut x {
                debug!("Finalizing command: {:#?}", cmd);
                cmd.finalize(self, Some(pi))
            }
        } else {
            for cmd in &mut x {
                debug!("Finalizing command: {:#?}", cmd);
                cmd.finalize(self, None)
            }
        }
        self.commands_to_finalize = x;
    }

    pub fn abort(mut self, gridview: &mut GridView) {
        for mut cmd in self.commands_to_finalize.drain(..) {
            debug!("Sending command back for replanning: {:#?}", cmd);
            if let Err((mut cmd, err)) = gridview.plan(cmd) {
                cmd.abort(err);
            }
        }
    }

    pub fn droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        self.droplets
            .values()
            .filter(|&d| pid_option.map_or(true, |pid| d.id.process_id == pid))
            .map(|d| d.info())
            .collect()
    }

    /// Returns an invalid droplet, if any.
    fn get_collision(&self) -> Option<(i32, Droplet, Droplet)> {
        for (id1, droplet1) in &self.droplets {
            for (id2, droplet2) in &self.droplets {
                if id1 == id2 {
                    continue;
                }
                if droplet1.collision_group == droplet2.collision_group {
                    continue;
                }
                let distance = droplet1.collision_distance(droplet2);
                if distance <= 0 {
                    return Some((distance, droplet1.clone(), droplet2.clone()));
                }
            }
        }
        None
    }

    pub fn to_blobs(&self) -> Vec<SimpleBlob> {
        self.droplets.values().map(|d| d.to_blob()).collect()
    }

    /// Takes a map of droplet ids to droplets (as in that
    /// of the planner/executor view) and a vector of blobs
    /// (as in that of the chip view) and returns a matching
    /// of droplet ids to closest matching blobs.
    ///
    /// Can currently only handle where both views contain
    /// the same number of 'droplets'
    fn match_with_blobs<B: Blob>(&self, blobs: &[B]) -> Option<Map<DropletId, B>> {
        // Ensure lengths are the same
        if self.droplets.len() != blobs.len() {
            error!("Expected and actual droplets are of different lengths");
            return None;
        }
        let mut result = Map::new(); // to be returned
        let mut ids = vec![]; // store corresponding ids to indeces
        let mut matches = vec![]; // store similarity between blobs/droplets
        let n = blobs.len();

        // store the id of each droplet in its corresponding
        // index in 'ids', then store the similarity of each
        // droplet to each blob in 'matches'
        for (&id, droplet) in &self.droplets {
            ids.push(id);
            for blob in blobs {
                let similarity = blob.get_similarity(&droplet);
                // must be non-negative for the algorithm to work
                assert!(similarity >= 0);
                matches.push(similarity);
            }
        }

        // convert the matches vector to a matrix
        // input should be [1,2,3,4], where the output
        // matrix is [[1,2],[3,4]]
        let m: Matrix<i32> = Matrix::from_vec(n, n, matches);

        // km is a vector of size n where the value at each index
        // corresponds to the index of a blob
        let (_c, km) = kuhn_munkres_min(&m);

        for i in 0..n {
            result.insert(ids[i], blobs[km[i]].clone());
        }
        Some(result)
    }

    // this will take commands_to_finalize from the old snapshot into the new
    // one if an error is found produced
    pub fn correct(&mut self, blobs: &[impl Blob]) -> Option<Snapshot> {
        let blob_matching = self.match_with_blobs(blobs)?;
        let mut was_error = false;
        let new_droplets: Map<_, _> = blob_matching
            .iter()
            .map(|(&id, blob)| {
                let d = self.droplets.get_mut(&id).unwrap();
                let d_new = blob.to_droplet(id);
                if d.location != d_new.location || d.dimensions != d_new.dimensions {
                    info!("Found error in droplet {:?}", id);
                    debug!("Droplet error\n  Expected: {:#?}\n  Found: {:#?}", d, d_new);
                    was_error = true;
                }
                // HACK FIXME this mutation is not great
                if (d.volume - d_new.volume).abs() > 1.0 {
                    info!(
                        "volume of {} changed: {} -> {}",
                        id.id, d.volume, d_new.volume
                    )
                }
                d.volume = d_new.volume;
                (id, d_new)
            }).collect();

        if was_error {
            let mut new_snapshot = Snapshot {
                droplets: new_droplets,
                commands_to_finalize: Vec::new(),
            };
            ::std::mem::swap(
                &mut new_snapshot.commands_to_finalize,
                &mut self.commands_to_finalize,
            );
            Some(new_snapshot)
        } else {
            None
        }
    }

    pub fn diff_droplet(&self, id: &DropletId, other: &Snapshot) -> DropletDiff {
        use self::DropletDiff::*;
        let droplet = self
            .droplets
            .get(id)
            .expect("id should be in self snapshot");
        if let Some(other_droplet) = other.droplets.get(id) {
            // NOTE we only care about location diffs for now
            let loc = droplet.location;
            let other_loc = other_droplet.location;
            if loc != other_loc {
                // for now, just assert that we are only moving one spot at a time
                // FIXME HACK
                // assert_eq!((&loc - &other_loc).norm(), 1);
                Moved {
                    from: loc,
                    to: other_loc,
                }
            } else {
                DidNotMove
            }
        } else {
            Disappeared
        }
    }

    pub fn get_error_edges(
        &self,
        planned_outcome: &Snapshot,
        actual_outcome: &Snapshot,
    ) -> Vec<(Location, Location)> {
        use self::DropletDiff::*;

        self.droplets
            .keys()
            .filter_map(|id| {
                let planned_diff = self.diff_droplet(id, planned_outcome);
                let actual_diff = self.diff_droplet(id, actual_outcome);
                match (planned_diff, actual_diff) {
                    (Moved { from, to }, DidNotMove) => {
                        if (&from - &to).norm() == 1 {
                            Some((from, to))
                        } else {
                            warn!("Droplet {} jumped from {} to {}!", id.id, from, to);
                            None
                        }
                    }
                    _ => None,
                }
            }).collect()
    }
}

#[derive(Debug)]
pub enum ExecResponse {
    Step(Snapshot),
    NotReady,
    Done,
}

impl GridView {
    pub fn new(grid: Grid) -> GridView {
        let mut planned = VecDeque::new();
        planned.push_back(Snapshot::default());

        #[cfg(feature = "pi")]
        let pi = match ::std::env::var("PUDDLE_PI") {
            Ok(s) => if s == "1" {
                let mut pi = RaspberryPi::new().unwrap();
                info!("Initialized the pi!");
                Some(pi)
            } else {
                warn!("Couldn't read PUDDLE_PI={}", s);
                None
            },
            Err(_) => {
                info!("Did not start the pi!");
                None
            }
        };

        GridView {
            grid: grid,
            planned,
            completed: Vec::new(),
            done: false,
            bad_edges: Set::new(),
            #[cfg(feature = "pi")]
            pi,
        }
    }

    pub fn close(&mut self) {
        info!("Marking gridview as DONE!");
        self.done = true;
    }

    pub fn execute(&mut self) -> ExecResponse {
        use self::ExecResponse::*;

        // compare with len - 1 because we wouldn't want to "write out" a state
        // that hasn't been fully planned
        let resp = if let Some(planned_snapshot) = self.planned.pop_front() {
            Step(planned_snapshot)
        } else if self.done {
            Done
        } else {
            NotReady
        };

        trace!(
            "execute sending {:?}. Completed: {}, planned: {}.",
            resp,
            self.completed.len(),
            self.planned.len(),
        );
        resp
    }

    pub fn commit_pending(&mut self, mut snapshot: Snapshot) {
        #[cfg(not(feature = "pi"))]
        snapshot.finalize();
        #[cfg(feature = "pi")]
        snapshot.finalize(self.pi.as_mut());

        self.completed.push(snapshot);
    }

    pub fn snapshot(&self) -> &Snapshot {
        self.planned.back().unwrap()
    }

    // TODO probably shouldn't provide this
    pub fn snapshot_mut(&mut self) -> &mut Snapshot {
        self.planned.back_mut().unwrap()
    }

    pub fn snapshot_ensure(&mut self) {
        if self.planned.is_empty() {
            let last = self.completed.last().unwrap();
            self.planned.push_back(last.new_with_same_droplets())
        }
    }

    pub fn exec_snapshot(&self) -> &Snapshot {
        self.completed.last().unwrap()
    }

    fn tick(&mut self) {
        let new_snapshot = {
            let just_planned = self.planned.back().unwrap();
            if let Some(col) = just_planned.get_collision() {
                panic!("collision: {:#?}", col);
            };

            just_planned.new_with_same_droplets()
        };

        self.planned.push_back(new_snapshot);
        trace!("TICK! len={}", self.planned.len());
    }

    fn update(&mut self, id: DropletId, func: impl FnOnce(&mut Droplet)) {
        let now = self.planned.back_mut().unwrap();
        let droplet = now
            .droplets
            .get_mut(&id)
            .unwrap_or_else(|| panic!("Tried to remove a non-existent droplet: {:?}", id));
        func(droplet);
    }

    pub fn plan_droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        // gets from the planner for now
        self.planned.back().unwrap().droplet_info(pid_option)
    }

    pub fn take_paths(&mut self, paths: &Map<DropletId, Path>, final_tick: bool) {
        let max_len = paths.values().map(|path| path.len()).max().unwrap_or(0);

        // make sure that all droplets start where they are at this time step
        for (id, path) in paths.iter() {
            let snapshot = self.planned.back().unwrap();
            let droplet = &snapshot.droplets[&id];
            assert_eq!(droplet.location, path[0]);
        }

        for i in 1..max_len {
            for (&id, path) in paths.iter() {
                if i < path.len() {
                    self.update(id, |droplet| {
                        assert!(droplet.location.distance_to(&path[i]) <= 1);
                        droplet.location = path[i];
                    });
                }
            }

            if i < max_len - 1 || final_tick {
                self.tick();
            }
        }
    }

    pub fn subview(
        &mut self,
        ids: impl IntoIterator<Item = DropletId>,
        mapping: Map<Location, Location>,
    ) -> GridSubView {
        GridSubView {
            backing_gridview: self,
            mapping: mapping,
            ids: ids.into_iter().collect(),
        }
    }

    pub fn register(&mut self, cmd: Box<dyn Command>) {
        // this goes in the *just planned* thing, not the one currently being planned.
        let just_planned = self.planned.len() - 2;
        self.planned[just_planned].commands_to_finalize.push(cmd)
    }

    pub fn rollback(&mut self, new_snapshot: &Snapshot) {
        let old_planned: Vec<_> = self.planned.drain(..).collect();
        self.planned
            .push_back(new_snapshot.new_with_same_droplets());
        assert_eq!(self.planned.len(), 1);

        for planned_snapshot in old_planned {
            planned_snapshot.abort(self)
        }
    }

    pub fn perturb(&self, rng: &mut impl Rng, snapshot: &Snapshot) -> Option<Snapshot> {
        let now = snapshot;
        let then = self.completed.last()?;

        let id = {
            let ids: Vec<_> = now.droplets.keys().collect();
            match rng.choose(ids.as_slice()) {
                Some(&&id) => id,
                None => return None,
            }
        };

        let mut now2 = now.new_with_same_droplets();

        if let Some(old_droplet) = then.droplets.get(&id) {
            let was_there = now2.droplets.insert(id, old_droplet.clone());
            assert!(was_there.is_some());
        }

        Some(now2)
    }

    pub fn add_error_edges(&mut self, planned: &Snapshot, actual: &Snapshot) {
        let previous = self.completed.last().unwrap();
        let edges = previous.get_error_edges(planned, actual);
        let n_edges = edges.len();
        warn!(
            "Added error {} edges, now there are {}: {:?}",
            n_edges,
            self.bad_edges.len() / 2,
            edges,
        );
        for (loc1, loc2) in edges {
            // for now, insert edges both ways
            self.bad_edges.insert((loc1, loc2));
            self.bad_edges.insert((loc2, loc1));
        }
    }
}

pub struct GridSubView<'a> {
    backing_gridview: &'a mut GridView,
    mapping: Map<Location, Location>,
    ids: Set<DropletId>,
}

impl<'a> GridSubView<'a> {
    pub fn tick(&mut self) {
        self.backing_gridview.tick()
    }

    #[cfg(feature = "pi")]
    pub fn with_pi<T>(&mut self, f: impl FnOnce(&mut RaspberryPi) -> T) -> Option<T> {
        self.backing_gridview.pi.as_mut().map(f)
    }

    pub fn get_electrode(&self, loc: &Location) -> Option<&Electrode> {
        let actual_loc = self.mapping.get(loc)?;
        self.backing_gridview.grid.get_cell(&actual_loc)
    }

    // TODO: translate or somehow hide the untranslated location of this
    pub fn get(&self, id: &DropletId) -> &Droplet {
        assert!(self.ids.contains(&id));
        &self.backing_gridview.snapshot().droplets[id]
    }

    fn get_mut(&mut self, id: &DropletId) -> &mut Droplet {
        assert!(self.ids.contains(&id));
        self.backing_gridview
            .snapshot_mut()
            .droplets
            .get_mut(id)
            .unwrap()
    }

    pub fn insert(&mut self, mut droplet: Droplet) {
        let new_loc = self.mapping.get(&droplet.location);
        trace!("Inserting {:#?} at {:?}", droplet, new_loc);
        droplet.location = *new_loc.unwrap();
        let was_not_there = self.ids.insert(droplet.id);
        assert!(was_not_there);
        let snapshot = self.backing_gridview.snapshot_mut();
        let was_there = snapshot.droplets.insert(droplet.id, droplet);
        assert!(was_there.is_none());
    }

    pub fn remove(&mut self, id: &DropletId) -> Droplet {
        let was_there = self.ids.remove(id);
        assert!(was_there);
        let snapshot = self.backing_gridview.snapshot_mut();
        let mut droplet = snapshot.droplets.remove(id).unwrap();
        // FIXME this is pretty dumb
        let (unmapped_loc, _) = self
            .mapping
            .iter()
            .find(|(_, &v)| v == droplet.location)
            .unwrap();
        droplet.location = *unmapped_loc;
        droplet
    }

    fn check_droplet(&self, id: &DropletId) {
        // TODO will this have translated or real location??
        let droplet = self.get(id);
        let mapped_to: Set<_> = self.mapping.values().collect();
        // TODO this is pretty slow
        for i in 0..droplet.dimensions.y {
            for j in 0..droplet.dimensions.x {
                let loc = Location {
                    y: droplet.location.y + i,
                    x: droplet.location.x + j,
                };
                if !mapped_to.contains(&loc) {
                    panic!("{} was unmapped!, mapping: {:#?}", loc, self.mapping);
                }
            }
        }
    }

    fn update(&mut self, id: &DropletId, func: impl FnOnce(&mut Droplet)) {
        func(self.get_mut(id));
        self.check_droplet(id);
    }

    pub fn move_west(&mut self, id: DropletId) {
        trace!("Moving droplet {:?} west", id);
        self.update(&id, |droplet| {
            droplet.location = droplet.location.west();
        })
    }

    pub fn move_east(&mut self, id: DropletId) {
        trace!("Moving droplet {:?} east", id);
        self.update(&id, |droplet| {
            droplet.location = droplet.location.east();
        })
    }

    pub fn move_north(&mut self, id: DropletId) {
        trace!("Moving droplet {:?} north", id);
        self.update(&id, |droplet| {
            droplet.location = droplet.location.north();
        })
    }

    pub fn move_south(&mut self, id: DropletId) {
        trace!("Moving droplet {:?} south", id);
        self.update(&id, |droplet| {
            droplet.location = droplet.location.south();
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use grid::parse::tests::parse_strings;

    pub fn id2c(id: &DropletId) -> char {
        assert!(id.id < 255);
        (id.id as u8) as char
    }

    pub fn c2id(c: char) -> DropletId {
        for u in 0x00u8..0xff {
            let c2 = u as char;
            if c == c2 {
                return DropletId {
                    id: u as usize,
                    process_id: 0,
                };
            }
        }
        panic!("Can't make {} a u8", c);
    }

    pub fn parse_gridview(strs: &[&str]) -> GridView {
        // same chars are guaranteed to have the same ids

        let (grid, blobs) = parse_strings(&strs);
        let mut snapshot = Snapshot::default();

        for (ch, blob) in blobs.iter() {
            let id = c2id(*ch);
            snapshot.droplets.insert(id, blob.to_droplet(id));
        }

        let mut gv = GridView::new(grid);
        gv.planned[0] = snapshot;
        gv
    }

    pub fn parse_snapshot(strs: &[&str]) -> Snapshot {
        let mut gv = parse_gridview(strs);
        gv.planned.remove(0).unwrap()
    }

    fn check_all_matched(
        snapshot_strs: &[&str],
        blob_strs: &[&str],
    ) -> Option<Map<DropletId, SimpleBlob>> {
        let snapshot = parse_snapshot(&snapshot_strs);
        let (_, chip_blobs) = parse_strings(&blob_strs);

        let blobs: Vec<SimpleBlob> = chip_blobs.values().cloned().collect();
        let result: Map<DropletId, SimpleBlob> = snapshot.match_with_blobs(&blobs)?;

        // create the expected map by mapping the ids in the snapshot
        // to the associated blob which corresponds to the character
        let mut expected: Map<DropletId, SimpleBlob> = Map::new();
        for id in snapshot.droplets.keys() {
            expected.insert(*id, chip_blobs[&id2c(id)].clone());
        }

        for id in expected.keys() {
            // we can't compare blobs or droplets, so we get the droplet_info
            assert_eq!(
                result.get(id).map(|blob| blob.to_droplet(*id).info()),
                expected.get(id).map(|blob| blob.to_droplet(*id).info())
            )
        }

        Some(result)
    }

    #[test]
    fn test_no_diff() {
        let strs = vec![
            "aa..........c",
            ".....bb......",
            ".............",
            ".............",
        ];
        assert!(check_all_matched(&strs, &strs).is_some());
    }

    #[test]
    fn test_location_diff() {
        let exec_strs = vec![
            "aa..........c",
            ".....bb......",
            ".............",
            ".............",
        ];

        let chip_strs = vec![
            "aa...........",
            "............c",
            ".....bb......",
            ".............",
        ];

        assert!(check_all_matched(&exec_strs, &chip_strs).is_some());
    }

    #[test]
    fn test_dimension_diff() {
        let exec_strs = vec![
            "aa..........c",
            ".....bb......",
            ".............",
            ".............",
        ];

        let chip_strs = vec![
            "aa.........cc",
            ".....b.......",
            ".....b.......",
            ".............",
        ];

        assert!(check_all_matched(&exec_strs, &chip_strs).is_some());
    }

    #[test]
    fn test_mix_split_diff() {
        let exec_strs = vec![
            "aa...........",
            ".....bb..c...",
            ".............",
            ".............",
        ];

        let chip_strs = vec![
            "aa...........",
            ".....bbb.....",
            ".............",
            ".............",
        ];

        assert!(check_all_matched(&exec_strs, &chip_strs).is_none());
    }

    #[test]
    fn test_droplet_diff() {
        use self::DropletDiff::*;

        let old = parse_snapshot(&[
            ".a...........",
            ".....bb..c...",
            ".............",
            ".............",
        ]);

        let new = parse_snapshot(&[
            ".............",
            ".a...bb......",
            ".............",
            ".............",
        ]);

        // locations for droplet a
        let from = Location { y: 0, x: 1 };
        let to = Location { y: 1, x: 1 };

        assert_eq!(old.diff_droplet(&c2id('a'), &new), Moved { from, to });
        assert_eq!(old.diff_droplet(&c2id('b'), &new), DidNotMove);
        assert_eq!(old.diff_droplet(&c2id('c'), &new), Disappeared);

        let error_edges = {
            let planned = &new;
            let actual = &old;
            old.get_error_edges(planned, actual)
        };

        assert_eq!(error_edges.len(), 1);
        assert_eq!(error_edges[0], (from, to));
    }
}
