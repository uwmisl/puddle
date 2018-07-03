use rand::Rng;
use std::collections::VecDeque;

use pathfinding::kuhn_munkres::kuhn_munkres_min;
use pathfinding::matrix::Matrix;

use command::Command;
use grid::droplet::{Blob, SimpleBlob};
use plan::Path;
use process::ProcessId;
use util::collections::{Map, Set};

use super::{Droplet, DropletId, DropletInfo, Grid, Location};

pub struct GridView {
    pub grid: Grid,

    completed: Vec<Snapshot>,
    planned: VecDeque<Snapshot>,
    pub done: bool,
}

#[must_use]
#[derive(Debug, Default)]
pub struct Snapshot {
    pub droplets: Map<DropletId, Droplet>,
    pub commands_to_finalize: Vec<Box<Command>>,
}

impl Snapshot {
    fn new_with_same_droplets(&self) -> Snapshot {
        let mut new_snapshot = Snapshot::default();
        new_snapshot.droplets = self.droplets.clone();

        // clear out the destination because we're doing to replan
        for d in new_snapshot.droplets.values_mut() {
            d.destination = None;
        }

        new_snapshot
    }

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

    pub fn abort(mut self, gridview: &mut GridView) {
        for cmd in self.commands_to_finalize.drain(..) {
            debug!("Sending command back for replanning: {:#?}", cmd);
            gridview.plan(cmd).unwrap();
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
    fn get_collision(&self) -> Option<(DropletId, DropletId)> {
        for (id1, droplet1) in &self.droplets {
            for (id2, droplet2) in &self.droplets {
                if id1 == id2 {
                    continue;
                }
                if droplet1.collision_group == droplet2.collision_group {
                    continue;
                }
                if droplet1.collision_distance(droplet2) <= 0 {
                    return Some((*id1, *id2));
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
            return None
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
                matches.push(blob.get_similarity(&droplet));
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
                let d = &self.droplets[&id];
                if blob.get_similarity(d) > 0 {
                    info!("Found error in droplet {:?}", id);
                    was_error = true;
                }
                (id, blob.to_droplet(id))
            })
            .collect();

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
        GridView {
            grid: grid,
            planned,
            completed: Vec::new(),
            done: false,
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
        snapshot.finalize();
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
        let droplet = now.droplets
            .get_mut(&id)
            .unwrap_or_else(|| panic!("Tried to remove a non-existent droplet: {:?}", id));
        func(droplet);
    }

    pub fn plan_droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        // gets from the planner for now
        self.planned.back().unwrap().droplet_info(pid_option)
    }

    pub fn take_paths(&mut self, paths: &Map<DropletId, Path>) {
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
            self.tick();
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

    pub fn register(&mut self, cmd: Box<Command>) {
        // this goes in the *just planned* thing, not the one currently being planned.
        let just_planned = self.planned.len() - 2;
        self.planned[just_planned].commands_to_finalize.push(cmd)
    }

    pub fn rollback(&mut self) {
        let old_planned: Vec<_> = self.planned.drain(..).collect();
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
        let (unmapped_loc, _) = self.mapping
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
mod tests {
    use super::*;
    use grid::parse::tests::parse_strings;

    fn parse_snapshot(strs: &[&str]) -> (Map<DropletId, char>, Snapshot) {
        let (_, blobs) = parse_strings(&strs);
        let mut id_to_char = Map::new();
        let mut snapshot = Snapshot::default();

        for (i, (ch, blob)) in blobs.iter().enumerate() {
            let id = DropletId {
                id: i,
                process_id: 0,
            };
            id_to_char.insert(id, *ch);
            snapshot.droplets.insert(id, blob.to_droplet(id));
        }

        (id_to_char, snapshot)
    }

    fn check_all_matched(snapshot_strs: &[&str], blob_strs: &[&str]) {
        let (id_to_char, snapshot) = parse_snapshot(&snapshot_strs);
        let (_, chip_blobs) = parse_strings(&blob_strs);

        let blobs: Vec<SimpleBlob> = chip_blobs.values().cloned().collect();
        let result: Map<DropletId, SimpleBlob> = snapshot.match_with_blobs(&blobs).unwrap();

        // create the expected map by mapping the ids in the snapshot
        // to the associated blob which corresponds to the character
        let mut expected: Map<DropletId, SimpleBlob> = Map::new();
        for id in snapshot.droplets.keys() {
            expected.insert(*id, chip_blobs[&id_to_char[id]].clone());
        }

        for id in expected.keys() {
            // we can't compare blobs or droplets, so we get the droplet_info
            assert_eq!(result.get(id).map(|blob| blob.to_droplet(*id).info()),
                       expected.get(id).map(|blob| blob.to_droplet(*id).info()))
        }
    }

    #[test]
    fn test_no_diff() {
        let strs = vec![
            "aa..........c",
            ".....bb......",
            ".............",
            ".............",
        ];
        check_all_matched(&strs, &strs);
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

        check_all_matched(&exec_strs, &chip_strs);
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

        check_all_matched(&exec_strs, &chip_strs);
    }

    #[test]
    #[should_panic(expected = "Expected and actual droplets are of different lengths")]
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

        check_all_matched(&exec_strs, &chip_strs);
    }
}
