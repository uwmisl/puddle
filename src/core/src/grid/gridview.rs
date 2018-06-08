use rand::Rng;

use pathfinding::kuhn_munkres::*;
use pathfinding::matrix::*;

use command::Command;
use grid::droplet::Blob;
use plan::Path;
use process::ProcessId;
use util::collections::{Map, Set};

use super::{Droplet, DropletId, DropletInfo, Grid, Location};

pub struct GridView {
    pub grid: Grid,
    history: Vec<Snapshot>,
    exec_time: usize,
    done: bool,
}

#[derive(Default)]
pub struct Snapshot {
    pub droplets: Map<DropletId, Droplet>,
    commands_to_finalize: Vec<Box<Command>>,
}

impl Snapshot {
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

    pub fn droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        self.droplets
            .values()
            .filter(|&d| pid_option.map_or(true, |pid| d.id.process_id == pid))
            .map(|d| d.info())
            .collect()
    }

    pub fn to_blobs(&self) -> Vec<Blob> {
        self.droplets.values().map(|d| d.to_blob()).collect()
    }

    /// Takes a map of droplet ids to droplets (as in that
    /// of the planner/executor view) and a vector of blobs
    /// (as in that of the chip view) and returns a matching
    /// of droplet ids to closest matching blobs.
    ///
    /// Can currently only handle where both views contain
    /// the same number of 'droplets'
    pub fn match_with_blobs(&self, blobs: &[Blob]) -> Map<DropletId, Blob> {
        // Ensure lengths are the same
        if self.droplets.len() != blobs.len() {
            panic!("Expected and actual droplets are of different lengths");
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
        result
    }
}

#[derive(Debug)]
pub enum ExecResponse {
    Step,
    NotReady,
    Done,
}

impl GridView {
    pub fn new(grid: Grid) -> GridView {
        GridView {
            grid: grid,
            history: vec![Snapshot::default()],
            exec_time: 0,
            done: false,
        }
    }

    pub fn execute(&mut self) -> ExecResponse {
        use self::ExecResponse::*;

        // compare with len - 1 because we wouldn't want to "write out" a state
        // that hasn't been fully planned
        let resp = if self.exec_time < self.history.len() - 1 {
            // TODO should probably do this later when things have been validated
            self.history[self.exec_time].finalize();
            self.exec_time += 1;
            Step
        } else if self.done {
            Done
        } else {
            NotReady
        };

        trace!(
            "execute sending {:?} with exec_t={}, len={}",
            resp,
            self.exec_time,
            self.history.len()
        );
        resp
    }

    pub fn snapshot(&self) -> &Snapshot {
        self.history.last().unwrap()
    }

    pub fn exec_snapshot(&self) -> &Snapshot {
        &self.history[self.exec_time]
    }

    // TODO probably shouldn't provide this
    pub fn snapshot_mut(&mut self) -> &mut Snapshot {
        self.history.last_mut().unwrap()
    }

    fn insert(&mut self, droplet: Droplet) {
        let snapshot = self.history.last_mut().unwrap();
        let was_there = snapshot.droplets.insert(droplet.id, droplet);
        assert!(was_there.is_none());
    }

    fn remove(&mut self, id: &DropletId) -> Droplet {
        let snapshot = self.history.last_mut().unwrap();
        snapshot.droplets.remove(id).unwrap()
    }

    fn tick(&mut self) {
        let now = self.history.len() - 1;
        self.get_collision_at_time(now).map(|col| {
            panic!("collision: {:#?}", col);
        });

        let mut new_snapshot = Snapshot::default();
        new_snapshot.droplets = self.history[now].droplets.clone();
        self.history.push(new_snapshot);
        trace!("TICK! len={}", self.history.len());
    }

    /// Returns an invalid droplet, if any.
    fn get_collision_at_time(&self, time: usize) -> Option<(DropletId, DropletId)> {
        let droplets = &self.history[time].droplets;
        for (id1, droplet1) in droplets.iter() {
            for (id2, droplet2) in droplets.iter() {
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

    fn update(&mut self, id: DropletId, func: impl FnOnce(&mut Droplet)) {
        let now = self.history.last_mut().unwrap();
        let droplet = now.droplets
            .get_mut(&id)
            .unwrap_or_else(|| panic!("Tried to remove a non-existent droplet: {:?}", id));
        func(droplet);
    }

    pub fn exec_droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        // gets from the planner for now
        self.history[self.exec_time].droplet_info(pid_option)
    }

    pub fn plan_droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        // gets from the planner for now
        self.history.last().unwrap().droplet_info(pid_option)
    }

    pub fn take_paths(&mut self, paths: &Map<DropletId, Path>) {
        let max_len = paths.values().map(|path| path.len()).max().unwrap_or(0);

        // make sure that all droplets start where they are at this time step
        for (id, path) in paths.iter() {
            let snapshot = self.history.last().unwrap();
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
        let just_planned = self.history.len() - 2;
        self.history[just_planned].commands_to_finalize.push(cmd)
    }

    pub fn rollback(&mut self, snapshot: Snapshot) {
        self.history.truncate(self.exec_time + 1);
        self.history[self.exec_time] = snapshot;
    }

    pub fn perturb(&self, rng: &mut impl Rng) -> Option<Snapshot> {
        if self.exec_time < 1 {
            return None;
        }

        let then = &self.history[self.exec_time - 1];
        let now = &self.history[self.exec_time];

        let id = {
            let ids: Vec<_> = now.droplets.keys().collect();
            match rng.choose(ids.as_slice()) {
                Some(&&id) => id,
                None => return None,
            }
        };

        let mut now2 = Snapshot::default();
        now2.droplets = now.droplets.clone();

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
        self.backing_gridview.snapshot().droplets.get(id).unwrap()
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
        self.backing_gridview.insert(droplet);
    }

    pub fn remove(&mut self, id: &DropletId) -> Droplet {
        let was_there = self.ids.remove(id);
        assert!(was_there);
        let mut droplet = self.backing_gridview.remove(id);
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

        let blobs: Vec<Blob> = chip_blobs.values().cloned().collect();
        let result: Map<DropletId, Blob> = snapshot.match_with_blobs(&blobs);

        // create the expected map by mapping the ids in the snapshot
        // to the associated blob which corresponds to the character
        let mut expected: Map<DropletId, Blob> = Map::new();
        for id in snapshot.droplets.keys() {
            expected.insert(*id, chip_blobs[&id_to_char[id]].clone());
        }

        for id in expected.keys() {
            assert_eq!(result.get(id), expected.get(id));
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
