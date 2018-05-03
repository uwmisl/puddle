use rand::{IsaacRng, Rng};
use rand::distributions::Normal;

use super::{Droplet, DropletId, DropletInfo, Grid, Location};

use exec::Action;

use process::ProcessId;
use util::collections::Map;

#[derive(Debug, Clone)]
pub struct GridView {
    pub grid: Grid,
    pub droplets: Map<DropletId, Droplet>,
    rng: IsaacRng,
    split_error_stdev: Option<Normal>,
}

#[derive(Default, Deserialize)]
pub struct ErrorOptions {
    #[serde(default)]
    pub split_error_stdev: f64,
}

impl GridView {
    pub fn new(grid: Grid, opts: ErrorOptions) -> GridView {
        GridView {
            grid: grid,
            droplets: Map::new(),
            rng: IsaacRng::new_from_u64(0),
            split_error_stdev: Some(Normal::new(0.0, opts.split_error_stdev)),
        }
    }

    pub fn new_with_defaults(grid: Grid) -> GridView {
        GridView::new(grid, ErrorOptions::default())
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

                let collide = self.grid
                    .neighbors_dimensions(&droplet1.location, &droplet1.dimensions)
                    .into_iter()
                    .any(|loc| {
                        Droplet::get_locations(&droplet2.location, &droplet2.dimensions)
                            .contains(&loc)
                    });

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

                let collide = self.grid
                    .neighbors_dimensions(&dest1, &droplet1.dimensions)
                    .into_iter()
                    .any(|loc| Droplet::get_locations(&dest2, &droplet2.dimensions).contains(&loc));

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
            .filter(|&d| pid_option.map_or(true, |pid| d.id.process_id == pid))
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
            AddDroplet {
                id,
                location,
                dimensions,
                volume,
            } => self.insert(Droplet::new(id, volume, location, dimensions)),
            RemoveDroplet { id } => {
                self.remove(id);
            }
            Mix { in0, in1, out } => {
                let d0 = self.remove(in0);
                let d1 = self.remove(in1);
                let vol = d0.volume + d1.volume;
                // TODO right now this only mixes horizontally
                // it should somehow communicate with the Mix command to control the mixed droplets dimensions
                let dim = Location {
                    y: d0.dimensions.y.max(d1.dimensions.y),
                    x: d0.dimensions.x + d1.dimensions.x,
                };
                assert_eq!(d0.location.y, d1.location.y);
                assert_eq!(d0.location.x + d0.dimensions.x, d1.location.x);
                self.insert(Droplet::new(out, vol, d0.location, dim));
            }
            Split { inp, out0, out1 } => {
                let d = self.remove(inp);
                let vol = d.volume / 2.0;
                // create the error and clamp it to reasonable values
                let err = self.split_error_stdev.map_or(0.0, |dist| {
                    self.rng.sample(dist).min(d.volume).max(-d.volume)
                });

                // TODO: this should be related to volume in some fashion
                // currently, take the ceiling of the division of the split by two
                let dim = Location {
                    y: d.dimensions.y,
                    x: (d.dimensions.x + 1) / 2,
                };
                let offset = Location {
                    y: 0,
                    x: d.dimensions.x / 2,
                };

                let vol0 = vol - err;
                let vol1 = vol + err;

                self.insert(Droplet::new(out0, vol0, d.location, dim));
                self.insert(Droplet::new(out1, vol1, &d.location + &offset, dim));
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
            Tick => {
                self.get_collision().map(|(id1, id2)| {
                    let d1 = &self.droplets[&id1];
                    let d2 = &self.droplets[&id2];
                    error!("Collision between {:?}, {:?}", d1, d2);
                    panic!("collision");
                });
            }
            // NOTE: ping does nothing here by default
            Ping { .. } => {}
        }
    }
}

#[cfg(test)]
pub mod tests {

    // TODO make some unit tests

}
