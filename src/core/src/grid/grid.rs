use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json;

use std::collections::HashSet;
use std::io::Read;

use super::{Location, Snapshot};
use util::collections::{Map, Set};

use grid::parse::{Mark, ParsedElectrode, ParsedGrid};

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct Electrode {
    pub pin: u32,
    pub peripheral: Option<Peripheral>,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Peripheral {
    Heater { pwm_channel: u8, spi_channel: u8 },
    Input { pwm_channel: u8, name: String },
    Output { pwm_channel: u8, name: String },
}

impl Electrode {
    fn is_compatible(&self, other: &Self) -> bool {
        let (mine, theirs) = match (&self.peripheral, &other.peripheral) {
            (None, None) => return true,
            (Some(p1), Some(p2)) => (p1, p2),
            _ => return false,
        };

        use self::Peripheral::*;
        match (mine, theirs) {
            (Input { name: n1, .. }, Input { name: n2, .. }) => {
                n1 == n2
            },
            (Output { name: n1, .. }, Output { name: n2, .. }) => {
                n1 == n2
            },
            (Heater { .. }, Heater { .. }) => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Grid {
    pub vec: Vec<Vec<Option<Electrode>>>,
}

#[cfg_attr(rustfmt, rustfmt_skip)]
const NEIGHBORS_8: [Location; 8] = [
    Location { y: -1, x: -1 },
    Location { y:  0, x: -1 },
    Location { y:  1, x: -1 },
    Location { y: -1, x: 0 },
    // Location {y:  0, x:  0},
    Location { y:  1, x: 0 },
    Location { y: -1, x: 1 },
    Location { y:  0, x: 1 },
    Location { y:  1, x: 1 }
];

#[cfg_attr(rustfmt, rustfmt_skip)]
const NEIGHBORS_4: [Location; 4] = [
    Location { y:  0, x: -1 },
    Location { y: -1, x: 0 },
    // Location {y:  0, x:  0},
    Location { y:  1, x: 0 },
    Location { y:  0, x: 1 },
];

impl Grid {
    pub fn to_parsed_grid(&self) -> ParsedGrid {
        let mut peripherals = Map::new();
        let board = self.vec
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .enumerate()
                    .map(|(j, e_opt)| match e_opt {
                        None => ParsedElectrode::Marked(Mark::Empty),
                        Some(e) => {
                            if let Some(ref peripheral) = e.peripheral {
                                let loc = Location {
                                    y: i as i32,
                                    x: j as i32,
                                };
                                peripherals.insert(loc.to_string(), peripheral.clone());
                            }
                            ParsedElectrode::Index(e.pin)
                        }
                    })
                    .collect()
            })
            .collect();
        ParsedGrid { board, peripherals }
    }

    pub fn rectangle(h: usize, w: usize) -> Self {
        let mut pin = 0;
        let always_cell = |_| {
            let cell = Some(Electrode {
                pin: pin,
                peripheral: None,
            });
            pin += 1;
            cell
        };
        Grid::from_function(always_cell, h, w)
    }

    pub fn max_height(&self) -> usize {
        self.vec.len()
    }

    pub fn max_width(&self) -> usize {
        self.vec.iter().map(|row| row.len()).max().unwrap_or(0)
    }

    pub fn max_pin(&self) -> u32 {
        self.vec
            .iter()
            .flat_map(|row| row.iter())
            .map(|e| e.as_ref().map_or(0, |e| e.pin))
            .max()
            .unwrap_or(0)
    }

    pub fn from_reader<R: Read>(reader: R) -> Result<Grid, serde_json::Error> {
        let parsed_grid: ParsedGrid = serde_json::from_reader(reader)?;
        Ok(parsed_grid.to_grid())
    }

    pub fn locations<'a>(&'a self) -> impl Iterator<Item = (Location, Electrode)> + 'a {
        self.vec.iter().enumerate().flat_map(|(i, row)| {
            row.iter().enumerate().filter_map(move |(j, cell_opt)| {
                cell_opt.as_ref().map(|cell: &Electrode| {
                    (
                        Location {
                            y: i as i32,
                            x: j as i32,
                        },
                        cell.clone(),
                    )
                })
            })
        })
    }

    /// Tests if this grid is compatible within `bigger` when `offset` is applied
    /// to `self`
    fn is_compatible_within(
        &self,
        offset: Location,
        bigger: &Self,
        snapshot: &Snapshot,
        bad_edges: &Set<(Location, Location)>,
    ) -> bool {
        self.locations().all(|(loc, my_cell)| {
            let their_loc = &loc + &offset;
            bigger.get_cell(&their_loc).map_or(false, |theirs| {
                // make sure that it's not the case that an internal edge to this subgrid is a bad edge in the larger grid
                my_cell.is_compatible(&theirs) && !snapshot.droplets.values().any(|droplet| {
                    let corner1 = droplet.location;
                    let corner2 = &droplet.location + &droplet.dimensions;
                    their_loc.min_distance_to_box(corner1, corner2) <= 0
                })
                    && !(self.get_cell(&loc.south()).is_some()
                        && bad_edges.contains(&(their_loc, their_loc.south())))
                    && !(self.get_cell(&loc.east()).is_some()
                        && bad_edges.contains(&(their_loc, their_loc.east())))
            })
        })
    }

    fn mapping_into_other_from_offset(
        &self,
        offset: Location,
        _bigger: &Self,
    ) -> Map<Location, Location> {
        // assert!(self.is_compatible_within(offset, bigger));

        let mut map = Map::new();

        for (loc, _) in self.locations() {
            map.insert(loc, &loc + &offset);
        }

        map
    }

    pub fn place(
        &self,
        smaller: &Self,
        snapshot: &Snapshot,
        bad_edges: &Set<(Location, Location)>,
    ) -> Option<Map<Location, Location>> {
        let offset_found = self.vec
            .iter()
            .enumerate()
            .flat_map(move |(i, row)| {
                (0..row.len()).map(move |j| Location {
                    y: i as i32,
                    x: j as i32,
                })
            })
            .find(|&offset| smaller.is_compatible_within(offset, self, snapshot, bad_edges));

        let result =
            offset_found.map(|offset| smaller.mapping_into_other_from_offset(offset, self));

        // verify the mapping by checking that each space is far enough away from the droplets
        if let Some(mapping) = result.as_ref() {
            for droplet in snapshot.droplets.values() {
                let corner1 = droplet.location;
                let corner2 = &droplet.location + &droplet.dimensions;
                for loc in mapping.values() {
                    assert!(loc.min_distance_to_box(corner1, corner2) > 0);
                }
            }
        };

        result
    }

    pub fn from_function<F>(mut f: F, height: usize, width: usize) -> Grid
    where
        F: FnMut(Location) -> Option<Electrode>,
    {
        let vec: Vec<Vec<_>> = (0..height)
            .map(move |i| {
                (0..width)
                    .map(|j| {
                        f(Location {
                            y: i as i32,
                            x: j as i32,
                        })
                    })
                    .collect()
            })
            .collect();

        Grid { vec }
    }

    // from here on out, functions only return valid locations

    pub fn get_cell(&self, loc: &Location) -> Option<&Electrode> {
        if loc.x < 0 || loc.y < 0 {
            return None;
        }
        let i = loc.y as usize;
        let j = loc.x as usize;
        self.vec
            .get(i)
            .and_then(|row| row.get(j).and_then(|cell_opt| cell_opt.as_ref()))
    }

    pub fn get_cell_mut(&mut self, loc: &Location) -> Option<&mut Electrode> {
        if loc.x < 0 || loc.y < 0 {
            return None;
        }
        let i = loc.y as usize;
        let j = loc.x as usize;
        self.vec
            .get_mut(i)
            .and_then(|row| row.get_mut(j).and_then(|cell_opt| cell_opt.as_mut()))
    }

    fn locations_from_offsets<'a, I>(&self, loc: &Location, offsets: I) -> Vec<Location>
    where
        I: Iterator<Item = &'a Location>,
    {
        offsets
            .map(|off| loc + off)
            .filter(|loc| self.get_cell(loc).is_some())
            .collect()
    }

    pub fn neighbors4(&self, loc: &Location) -> Vec<Location> {
        self.locations_from_offsets(loc, NEIGHBORS_4.into_iter())
    }

    pub fn neighbors8(&self, loc: &Location) -> Vec<Location> {
        self.locations_from_offsets(loc, NEIGHBORS_8.into_iter())
    }

    pub fn neighbors9(&self, loc: &Location) -> Vec<Location> {
        let mut vec = self.locations_from_offsets(loc, NEIGHBORS_8.into_iter());
        vec.push(*loc);
        vec
    }

    /// Returns a Vec representing the neighbors of the location combined with
    /// the dimensions of the droplet.
    pub fn neighbors_dimensions(&self, loc: &Location, dimensions: &Location) -> Vec<Location> {
        let mut dimensions_nbrhd: HashSet<Location> = HashSet::new();
        for y in 0..dimensions.y {
            for x in 0..dimensions.x {
                let new_loc = loc + &Location { y, x };
                dimensions_nbrhd.extend(self.neighbors9(&new_loc));
            }
        }
        dimensions_nbrhd.iter().cloned().collect()
    }
}

#[cfg(test)]
impl Grid {
    pub fn is_connected(&self) -> bool {
        use grid::location::tests::connected_components;
        let locs = self.locations().map(|(loc, _cell)| loc);
        let label_map = connected_components(locs);
        label_map.values().all(|v| *v == 0)
    }
}

impl Serialize for Grid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_parsed_grid().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Grid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ParsedGrid::deserialize(deserializer).map(|pg| pg.to_grid())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::iter::FromIterator;

    #[test]
    fn test_connected() {
        let el = || Some(Electrode {
            pin: 0,
            peripheral: None,
        });
        let grid1 = Grid {
            vec: vec![vec![None, el()], vec![el(), None]],
        };
        let grid2 = Grid {
            vec: vec![vec![el(), el()], vec![None, None]],
        };

        assert!(!grid1.is_connected());
        assert!(grid2.is_connected())
    }

    #[test]
    fn test_place_heater() {
        let mut grid = Grid::rectangle(3, 3);
        let heater_loc = Location { y: 2, x: 1 };
        grid.get_cell_mut(&heater_loc).unwrap().peripheral = Some(Peripheral::Heater {
            // these don't matter, they shouldn't be used for compatibility
            pwm_channel: 10,
            spi_channel: 42,
        });

        let mut small_grid = Grid::rectangle(1, 1);
        small_grid
            .get_cell_mut(&Location { y: 0, x: 0 })
            .unwrap()
            .peripheral = Some(Peripheral::Heater {
            pwm_channel: 0,
            spi_channel: 0,
        });

        let snapshot = &Snapshot::default();
        let bad_edges = &Set::default();

        let map = grid.place(&small_grid, snapshot, bad_edges).unwrap();

        assert_eq!(map.get(&Location { y: 0, x: 0 }), Some(&heater_loc));
    }

    #[test]
    fn grid_self_compatible() {
        let g1 = Grid::rectangle(5, 4);
        let g2 = Grid::rectangle(5, 4);
        let zero = Location { x: 0, y: 0 };
        let snapshot = &Snapshot::default();
        let bad_edges = &Set::default();
        assert!(g1.is_compatible_within(zero, &g2, snapshot, bad_edges))
    }

    #[test]
    fn grid_self_place() {
        let grid = Grid::rectangle(5, 4);

        let snapshot = &Snapshot::default();
        let bad_edges = &Set::default();
        let map = grid.place(&grid, snapshot, bad_edges).unwrap();

        let identity_locs: Map<Location, Location> =
            Map::from_iter(grid.locations().map(|(loc, _)| (loc, loc)));
        assert_eq!(&identity_locs, &map);
    }
}
