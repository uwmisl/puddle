use std::io::Read;
use serde_json;
use std::collections::HashSet;

use util::collections::Map;
use super::{Droplet, DropletId, Location};

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone, Copy)]
pub struct Cell {
    pub pin: u32,
}

impl Cell {
    #[allow(unused_variables)]
    fn is_compatible(&self, other: &Self) -> bool {
        true
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Grid {
    #[serde(rename = "board")]
    #[serde(with = "super::parse")]
    pub vec: Vec<Vec<Option<Cell>>>,
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
    pub fn rectangle(h: usize, w: usize) -> Self {
        let mut pin = 0;
        let always_cell = |_| {
            let cell = Some(Cell { pin: pin });
            pin += 1;
            cell
        };
        Grid::from_function(always_cell, h, w)
    }

    pub fn from_reader<R: Read>(reader: R) -> Result<Grid, serde_json::Error> {
        serde_json::from_reader(reader)
    }

    pub fn locations<'a>(&'a self) -> Box<Iterator<Item = (Location, Cell)> + 'a> {
        let iter = self.vec.iter().enumerate().flat_map(move |(i, row)| {
            row.iter().enumerate().filter_map(move |(j, cell_opt)| {
                cell_opt.map(|cell| {
                    (
                        Location {
                            y: i as i32,
                            x: j as i32,
                        },
                        cell,
                    )
                })
            })
        });
        Box::new(iter)
    }

    /// Tests if this grid is compatible within `bigger` when `offset` is applied
    /// to `self`
    fn is_compatible_within(
        &self,
        offset: Location,
        bigger: &Self,
        droplets: &Map<DropletId, Droplet>,
    ) -> bool {
        self.locations().all(|(loc, my_cell)| {
            let their_loc = &loc + &offset;
            bigger.get_cell(&their_loc).map_or(false, |theirs| {
                my_cell.is_compatible(&theirs) && !droplets.values().any(|droplet| {
                    (their_loc.x - droplet.location.x).abs() < 3
                        && (their_loc.y - droplet.location.y).abs() < 3
                })
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
        droplets: &Map<DropletId, Droplet>,
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
            .find(|&offset| smaller.is_compatible_within(offset, self, droplets));

        offset_found.map(|offset| smaller.mapping_into_other_from_offset(offset, self))
    }

    pub fn from_function<F>(mut f: F, height: usize, width: usize) -> Grid
    where
        F: FnMut(Location) -> Option<Cell>,
    {
        Grid {
            vec: (0..height)
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
                .collect(),
        }
    }

    // from here on out, functions only return valid locations

    pub fn get_cell(&self, loc: &Location) -> Option<&Cell> {
        if loc.x < 0 || loc.y < 0 {
            return None;
        }
        let i = loc.y as usize;
        let j = loc.x as usize;
        self.vec
            .get(i)
            .and_then(|row| row.get(j).and_then(|cell_opt| cell_opt.as_ref()))
    }

    fn locations_from_offsets<'a, I>(&self, loc: &Location, offsets: I) -> Vec<Location>
    where
        I: Iterator<Item = &'a Location>,
    {
        offsets
            .map(|off| loc + &off)
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
        let first = self.locations().next();

        if first.is_none() {
            // no cells, it's vacuously connected
            return true;
        }

        let mut todo = vec![first.unwrap().0];
        let mut seen = HashSet::new();

        while let Some(loc) = todo.pop() {
            // insert returns false if it was already there
            if seen.insert(loc) {
                for next in self.neighbors4(&loc) {
                    if !seen.contains(&next) {
                        todo.push(next)
                    }
                }
            }
        }

        let s = seen.len();
        let t = self.locations().count();

        for x in seen.iter() {
            assert!(self.get_cell(x).is_some())
        }
        s == t
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::iter::FromIterator;
    use std::ops::Range;

    use proptest::prelude::*;
    use proptest::collection::vec;
    use proptest::option::weighted;

    #[test]
    fn test_connected() {
        let cell = Some(Cell { pin: 0 });
        let grid1 = Grid {
            vec: vec![vec![None, cell], vec![cell, None]],
        };
        let grid2 = Grid {
            vec: vec![vec![cell, cell], vec![None, None]],
        };

        assert!(!grid1.is_connected());
        assert!(grid2.is_connected())
    }

    prop_compose! {
        fn arb_cell()(pin in prop::num::u32::ANY) -> Cell {
            Cell { pin: pin }
        }
    }

    prop_compose! {
        [pub] fn arb_grid (height_range: Range<usize>, width_range: Range<usize>,
                           density: f64)
            (v in vec(vec(weighted(density, arb_cell()),
                          width_range),
                      height_range))
             -> Grid
        {
            Grid {
                vec: v,
            }
        }
    }

    proptest! {
        #[test]
        fn grid_self_compatible(ref grid in arb_grid((1..10), (1..10), 0.5)) {
            let zero = Location {x: 0, y: 0};
            prop_assert!(grid.is_compatible_within(zero, &grid, &Map::new()))
        }

        #[test]
        fn grid_self_place(ref grid in arb_grid((1..10), (1..10), 0.5)) {
            let num_cells = grid.locations().count();
            prop_assume!( num_cells > 5 );

            let map = grid.place(&grid, &Map::new()).unwrap();

            let my_locs: Map<Location, Location> = Map::from_iter(
                grid.locations()
                    .map(|(loc, _)| (loc, loc))
            );
            prop_assert_eq!(&my_locs, &map);
        }
    }
}
