use super::{Droplet, DropletId, DropletInfo, Grid};

use exec::Action;

use process::ProcessId;
use util::collections::Map;

#[derive(Debug, Clone)]
pub struct GridView {
    pub grid: Grid,
    pub droplets: Map<DropletId, Droplet>,
}

impl GridView {
    pub fn new(grid: Grid) -> GridView {
        GridView {
            grid: grid,
            droplets: Map::new(),
        }
    }

    pub fn get_collision(&self) -> Option<(DropletId, DropletId)> {
        for (id1, droplet1) in self.droplets.iter() {
            for (id2, droplet2) in self.droplets.iter() {
                if id1 == id2 {
                    continue;
                }
                if droplet1.collision_group == droplet2.collision_group {
                    continue;
                }

                let collide = self.grid.neighbors9(&droplet1.location)
                    .into_iter()
                    // TODO this check will be more complicated when there are
                    // droplet shapes
                    .any(|loc| loc == droplet2.location);

                if collide {
                    return Some((*id1, *id2));
                }
            }
        }
        None
    }

    pub fn get_destination_collision(&self) -> Option<(DropletId, DropletId)> {
        for (id1, droplet1) in self.droplets.iter() {
            for (id2, droplet2) in self.droplets.iter() {
                if id1 == id2 {
                    continue;
                }
                if droplet1.collision_group == droplet2.collision_group {
                    continue;
                }

                if droplet1.destination.is_none() || droplet2.destination.is_none() {
                    continue;
                }

                let dest1 = droplet1.destination.unwrap();
                let dest2 = droplet2.destination.unwrap();

                let collide = self.grid.neighbors9(&dest1)
                    .into_iter()
                // TODO this check will be more complicated when there are
                // droplet shapes
                    .any(|loc| loc == dest2);

                if collide {
                    return Some((*id1, *id2));
                }
            }
        }
        None
    }

    pub fn droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        self.droplets
            .values()
            .filter(|&d| {
                pid_option.map_or(true, |pid| d.id.process_id == pid)
            })
            .map(|d| d.info())
            .collect()
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

    pub fn execute(&mut self, action: &Action) {
        use self::Action::*;
        match *action {
            AddDroplet { id, location } => {
                self.insert(Droplet::new(id, location));
            }
            RemoveDroplet { id } => {
                self.remove(id);
            }
            Mix { in0, in1, out } => {
                let d0 = self.remove(in0);
                let d1 = self.remove(in1);
                assert_eq!(d0.location, d1.location);
                self.insert(Droplet::new(out, d0.location))
            }
            Split { inp, out0, out1 } => {
                let d = self.remove(inp);
                self.insert(Droplet::new(out0, d.location));
                self.insert(Droplet::new(out1, d.location));
            }
            SetCollisionGroup { id, cg } => {
                self.get_mut(id).collision_group = cg;
            }
            UpdateDroplet { old_id, new_id } => {
                let mut d = self.remove(old_id);
                // NOTE this is pretty much the only place it's ok to change an id
                d.id = new_id;
                self.insert(d);
            }
            MoveDroplet { id, location } => {
                let droplet = self.get_mut(id);
                assert!(droplet.location.distance_to(&location) <= 1);
                droplet.location = location;
            }
            Tick => {},
            // NOTE: ping does nothing here by default
            Ping { tx: _ } => {}
        }
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use ::Location;

    use process::tests::*;

    use proptest::prelude::*;
    use proptest::collection::vec;
    use proptest::sample::select;

    use std::ops::Range;

    prop_compose! {
        fn arb_droplet_id()
            (id in prop::num::usize::ANY,
             pid in arb_process_id())
             -> DropletId
        {
            DropletId {
                id: id,
                process_id: pid
            }
        }
    }

    prop_compose! {
        fn arb_droplet(locations: Vec<Location>)
            (loc  in select(locations.clone()),
             id in arb_droplet_id(),
             dest in select(locations),
             cg in prop::num::usize::ANY)
            -> Droplet
        {
            Droplet {
                id: id,
                location: loc,
                destination: Some(dest),
                collision_group: cg,
            }
        }
    }

    pub fn arb_gridview(grid: Grid, n_droplets: Range<usize>) -> BoxedStrategy<GridView> {
        let locs = grid.locations().map(|(loc, _)| loc).collect();
        let droplet_gen = vec(arb_droplet(locs), n_droplets);
        let droplet_map_gen =
            droplet_gen.prop_map(|ds| ds.iter().map(|d| (d.id, d.clone())).collect());
        // can't use prop_compose! because we need to move the map here
        droplet_map_gen
            .prop_map(move |dmap| GridView {
                grid: grid.clone(),
                droplets: dmap,
            })
            .boxed()
    }
}
