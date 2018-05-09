use rand::IsaacRng;
use rand::distributions::Normal;

use super::{Droplet, DropletId, DropletInfo, Grid, Location};

use command::Action;
use plan::Path;
use process::ProcessId;
use util::collections::{Map, Set};

#[derive(Debug, Clone)]
pub struct RootGridView {
    pub grid: Grid,
    pub droplets: Map<DropletId, Droplet>,
    pub rng: IsaacRng,
    pub split_error_stdev: Option<Normal>,
    pub is_exec: bool,
}

#[derive(Default, Deserialize)]
pub struct ErrorOptions {
    #[serde(default)]
    pub split_error_stdev: f64,
}

impl RootGridView {
    pub fn new(grid: Grid, opts: ErrorOptions, is_exec: bool) -> RootGridView {
        RootGridView {
            grid: grid,
            droplets: Map::new(),
            rng: IsaacRng::new_from_u64(0),
            split_error_stdev: Some(Normal::new(0.0, opts.split_error_stdev)),
            is_exec: is_exec,
        }
    }

    /// Returns an invalid droplet, if any.
    pub fn get_collision(&self) -> Option<(DropletId, DropletId)> {
        for (id1, droplet1) in self.droplets.iter() {
            for (id2, droplet2) in self.droplets.iter() {
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

    pub fn get_destination_collision(&self) -> Option<(DropletId, DropletId)> {
        let dest_droplets = self.droplets
            .iter()
            .filter_map(|(id, d)| {
                d.destination.map(|dest| {
                    (
                        *id,
                        Droplet {
                            location: dest,
                            ..d.clone()
                        },
                    )
                })
            })
            .collect::<Vec<(DropletId, Droplet)>>();

        for &(ref id1, ref droplet1) in dest_droplets.iter() {
            for &(ref id2, ref droplet2) in dest_droplets.iter() {
                if id1 == id2 {
                    continue;
                }
                if droplet1.collision_group == droplet2.collision_group {
                    continue;
                }
                if droplet1.collision_distance(&droplet2) <= 0 {
                    return Some((*id1, *id2));
                }
            }
        }
        None
    }

    pub fn droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        self.droplets
            .values()
            .filter(|&d| pid_option.map_or(true, |pid| d.id.process_id == pid))
            .map(|d| d.info())
            .collect()
    }

    pub fn take_paths(&mut self, paths: &Map<DropletId, Path>, callback: impl Fn(&RootGridView)) {
        let max_len = paths.values().map(|path| path.len()).max().unwrap_or(0);

        for i in 0..max_len {
            for (&id, path) in paths.iter() {
                if i < path.len() {
                    let droplet = self.get_mut(id);
                    assert!(droplet.location.distance_to(&path[i]) <= 1);
                    droplet.location = path[i];
                }
            }
            callback(self);
        }
    }

    fn insert(&mut self, droplet: Droplet) {
        let was_there = self.droplets.insert(droplet.id, droplet);
        assert!(was_there.is_none());
    }

    fn remove(&mut self, id: DropletId) -> Droplet {
        self.droplets
            .remove(&id)
            .expect(&format!("Tried to remove a non-existent droplet: {:?}", id))
    }

    fn get_mut(&mut self, id: DropletId) -> &mut Droplet {
        self.droplets
            .get_mut(&id)
            .expect(&format!("Tried to get a non-existent droplet: {:?}", id))
    }
}

#[derive(Clone)]
pub struct PreGridSubView {
    pub mapping: Map<Location, Location>,
    pub ids: Set<DropletId>,
}

impl PreGridSubView {
    pub fn back(self, gv: &mut RootGridView) -> GridSubView {
        GridSubView {
            backing_gridview: gv,
            mapping: self.mapping,
            ids: self.ids,
        }
    }
}

pub struct GridSubView<'a> {
    // FIXME this shoudn't be pub
    pub backing_gridview: &'a mut RootGridView,
    pub mapping: Map<Location, Location>,
    pub ids: Set<DropletId>,
}

impl<'a> GridSubView<'a> {
    pub fn droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        self.backing_gridview.droplet_info(pid_option)
    }

    pub fn insert(&mut self, mut droplet: Droplet) {
        let new_loc = self.mapping.get(&droplet.location);
        trace!("Inserting {:#?} at {:?}", droplet, new_loc);
        droplet.location = *new_loc.unwrap();
        let was_not_there = self.ids.insert(droplet.id);
        assert!(was_not_there);
        self.backing_gridview.insert(droplet);
    }

    pub fn remove(&mut self, id: DropletId) -> Droplet {
        let was_there = self.ids.remove(&id);
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

    fn check_droplet(&mut self, id: DropletId) {
        let droplet = self.backing_gridview.get_mut(id);
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

    fn update(&mut self, id: DropletId, func: impl FnOnce(&mut Droplet)) {
        assert!(self.ids.contains(&id));
        func(self.backing_gridview.get_mut(id));
        self.check_droplet(id);
    }

    pub fn run_action(&mut self, action: Action<'a>) {
        (action.func)(self);
        if let Some(coll) = self.backing_gridview.get_collision() {
            panic!("collision! {:#?}", coll);
        }
    }

    pub fn move_west(&mut self, id: DropletId) {
        trace!("Moving droplet {:?} west", id);
        self.update(id, |droplet| {
            droplet.location = droplet.location.west();
        })
    }

    pub fn move_east(&mut self, id: DropletId) {
        trace!("Moving droplet {:?} east", id);
        self.update(id, |droplet| {
            droplet.location = droplet.location.east();
        })
    }

    pub fn move_north(&mut self, id: DropletId) {
        trace!("Moving droplet {:?} north", id);
        self.update(id, |droplet| {
            droplet.location = droplet.location.north();
        })
    }

    pub fn move_south(&mut self, id: DropletId) {
        trace!("Moving droplet {:?} south", id);
        self.update(id, |droplet| {
            droplet.location = droplet.location.south();
        })
    }

    pub fn is_exec(&self) -> bool {
        self.backing_gridview.is_exec
    }
}

#[cfg(test)]
pub mod tests {

    // TODO make some unit tests

}
