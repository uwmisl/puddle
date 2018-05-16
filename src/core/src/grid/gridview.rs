use super::{Droplet, DropletId, DropletInfo, Grid, Location};
use plan::Path;
use process::ProcessId;
use util::collections::{Map, Set};

// Send + 'static required because of jsonrpc
type Callback = Box<Fn(&GridView) + Send + 'static>;

pub struct GridView {
    pub grid: Grid,
    history: Vec<Map<DropletId, Droplet>>,
    exec_time: usize,
    callbacks: Map<usize, Vec<Callback>>,
    done: bool,
}

#[derive(Debug)]
pub enum ExecResponse {
    Step {
        // droplets: &'a Map<DropletId, Droplet>,
        // callbacks: &'a [Callback],
    },
    NotReady,
    Done,
}

impl GridView {
    pub fn new(grid: Grid) -> GridView {
        GridView {
            grid: grid,
            history: vec![Map::new()],
            exec_time: 0,
            callbacks: Map::new(),
            done: false,
        }
    }

    pub fn execute(&mut self) -> ExecResponse {
        use self::ExecResponse::*;

        // compare with len - 1 because we wouldn't want to "write out" a state
        // that hasn't been fully planned
        let resp = if self.exec_time < self.history.len() - 1 {
            self.callbacks.get(&self.exec_time).map(|cbs| {
                for cb in cbs {
                    cb(self)
                }
            });

            self.exec_time += 1;

            Step {
                // droplets, callbacks
            }
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

    pub fn droplets(&self) -> &Map<DropletId, Droplet> {
        self.history.last().unwrap()
    }

    pub fn droplets_mut(&mut self) -> &mut Map<DropletId, Droplet> {
        self.history.last_mut().unwrap()
    }

    fn insert(&mut self, droplet: Droplet) {
        let droplets = self.history.last_mut().unwrap();
        let was_there = droplets.insert(droplet.id, droplet);
        assert!(was_there.is_none());
    }

    fn remove(&mut self, id: &DropletId) -> Droplet {
        let droplets = self.history.last_mut().unwrap();
        droplets.remove(id).unwrap()
    }

    // fn check_droplet(&mut self, id: DropletId) {
    //     let droplet = self.backing_gridview.get_mut(id);
    //     let mapped_to: Set<_> = self.mapping.values().collect();
    //     // TODO this is pretty slow
    //     for i in 0..droplet.dimensions.y {
    //         for j in 0..droplet.dimensions.x {
    //             let loc = Location {
    //                 y: droplet.location.y + i,
    //                 x: droplet.location.x + j,
    //             };
    //             if !mapped_to.contains(&loc) {
    //                 panic!("{} was unmapped!, mapping: {:#?}", loc, self.mapping);
    //             }
    //         }
    //     }
    // }

    fn tick(&mut self) {
        //FIXME copy the stuff over
        let now = self.history.len() - 1;
        self.get_collision_at_time(now).map(|col| {
            panic!("collision: {:#?}", col);
        });

        let copy = self.droplets().clone();
        self.history.push(copy);
        trace!("TICK! len={}", self.history.len());
    }

    pub fn register(&mut self, callback: Callback) {
        let now = self.history.len() - 1;
        let cbs = self.callbacks.entry(now).or_insert_with(|| Vec::new());
        cbs.push(callback);
    }

    /// Returns an invalid droplet, if any.
    fn get_collision_at_time(&self, time: usize) -> Option<(DropletId, DropletId)> {
        let droplets = &self.history[time];
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
        let droplet = now.get_mut(&id)
            .unwrap_or_else(|| panic!("Tried to remove a non-existent droplet: {:?}", id));
        func(droplet);
    }

    pub fn exec_droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        // gets from the planner for now
        self.history[self.exec_time]
            .values()
            .filter(|&d| pid_option.map_or(true, |pid| d.id.process_id == pid))
            .map(|d| d.info())
            .collect()
    }

    pub fn droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        // gets from the planner for now
        self.history
            .last()
            .unwrap()
            .values()
            .filter(|&d| pid_option.map_or(true, |pid| d.id.process_id == pid))
            .map(|d| d.info())
            .collect()
    }

    pub fn take_paths(&mut self, paths: &Map<DropletId, Path>) {
        let max_len = paths.values().map(|path| path.len()).max().unwrap_or(0);

        // make sure that all droplets start where they are at this time step
        for (id, path) in paths.iter() {
            let droplet = &self.history.last().unwrap()[&id];
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
}

pub struct GridSubView<'a> {
    // FIXME this shoudn't be pub
    backing_gridview: &'a mut GridView,
    mapping: Map<Location, Location>,
    ids: Set<DropletId>,
}

impl<'a> GridSubView<'a> {
    pub fn register(&mut self, callback: Callback) {
        self.backing_gridview.register(callback)
    }

    pub fn tick(&mut self) {
        self.backing_gridview.tick()
    }

    // TODO: translate or somehow hide the untranslated location of this
    pub fn get(&self, id: &DropletId) -> &Droplet {
        assert!(self.ids.contains(&id));
        self.backing_gridview.droplets().get(id).unwrap()
    }

    fn get_mut(&mut self, id: &DropletId) -> &mut Droplet {
        assert!(self.ids.contains(&id));
        self.backing_gridview.droplets_mut().get_mut(id).unwrap()
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
