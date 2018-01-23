use std::collections::HashMap;

use arch::{Location};
use arch::parse::ParsedGrid;

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone, Copy)]
pub struct Cell {
    pin: u32,
}

impl Cell {
    #[allow(unused_variables)]
    fn is_compatible(&self, other: &Self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct Grid {
    pub vec: Vec<Option<Cell>>,
    pub height: u32,
    pub width: u32,
}

#[cfg_attr(rustfmt, rustfmt_skip)]
const NEIGHBORS_8: [Location; 8] = [
    Location { y: -1, x: -1 },
    Location { y:  0, x: -1 },
    Location { y:  1, x: -1 },
    Location { y: -1, x:  0 },
    // Location {y:  0, x:  0},
    Location { y:  1, x:  0 },
    Location { y: -1, x:  1 },
    Location { y:  0, x:  1 },
    Location { y:  1, x:  1 }
];

#[cfg_attr(rustfmt, rustfmt_skip)]
const NEIGHBORS_4: [Location; 4] = [
    Location { y:  0, x: -1 },
    Location { y: -1, x:  0 },
    // Location {y:  0, x:  0},
    Location { y:  1, x:  0 },
    Location { y:  0, x:  1 },
];

impl Grid {
    pub fn rectangle(h: u32, w: u32) -> Self {
        let mut pin = 0;
        let always_cell = |_| {
            let cell = Some(Cell { pin: pin });
            pin += 1;
            cell
        };
        Grid::from_function(always_cell, h, w)
    }

    fn from_parsed_grid(pg: ParsedGrid) -> Self {
        let height = pg.board.len();
        let width = pg.board.iter().map(|row| row.len()).max().unwrap();
        let size = height * width;

        let mut next_pin = 0;

        let mut vec = Vec::new();

        use arch::parse::CellIndex::*;
        use arch::parse::Mark::*;

        for i in 0..size {
            let y = i / width;
            let x = i % width;
            let row = &pg.board[y];
            let cell_opt = if x >= row.len() {
                None
            } else {
                match row[x] {
                    Marked(Empty) => None,
                    Marked(Auto) => {
                        let n = next_pin;
                        next_pin += 1;
                        Some(Cell { pin: n })
                    }
                }
            };

            vec.push(cell_opt);
        }

        Grid {
            height: height as u32,
            width: width as u32,
            vec: vec,
        }
    }

    fn locations<'a>(&'a self) -> Box<Iterator<Item = Location> + 'a> {
        // TODO this is a little ugly, maybe a custom iterator could be better
        let size = self.vec.len();
        let iter = (0..size).map(move |i| Location::from_index(i as u32, self.width));
        Box::new(iter)
    }

    pub fn locations_with_cells<'a>(&'a self) -> Box<Iterator<Item = (Location, Cell)> + 'a> {
        let iter = self.locations()
            .filter_map(move |loc| {
                self.get_cell(&loc)
                    .map(|cell| (loc, *cell))
            });
        Box::new(iter)
    }


    /// Tests if this grid is compatible within `bigger` when `offset` is applied
    /// to `self`
    fn is_compatible_within(&self, offset: Location, bigger: &Self) -> bool {
        self.locations_with_cells()
            .all(|(loc, my_cell)| {
                let their_loc = &loc + &offset;
                bigger.get_cell(&their_loc)
                    .map_or(false, |theirs| my_cell.is_compatible(&theirs))
            })
    }

    fn mapping_into_other_from_offset(&self,
                                      offset: Location,
                                      bigger: &Self)
                                      -> HashMap<Location, Location> {
        assert!(self.is_compatible_within(offset, bigger));

        let mut map = HashMap::new();

        for (loc, _) in self.locations_with_cells() {
            map.insert(loc, &loc + &offset);
        }

        assert_eq!(map.len(), self.locations_with_cells().count());
        let l: Vec<Location> = self.locations().collect();
        let locs: Vec<(Location, Cell)> = self.locations_with_cells().collect();
        println!("locs: {:?}", l);
        println!("locs_with: {:?}", locs);
        println!("vec: {:?}", self.vec);

        map
    }

    pub fn place(&self, smaller: &Self) -> Option<HashMap<Location, Location>> {
        let offset_found = self.locations()
            .find(|&offset| smaller.is_compatible_within(offset, self));
        println!("Placing with offset: {:?}", offset_found);

        offset_found.map(|offset|
                         smaller.mapping_into_other_from_offset(offset, self))
    }

    pub fn from_function<F>(f: F, height: u32, width: u32) -> Grid
        where F: FnMut(Location) -> Option<Cell>
    {

        let size = height * width;
        let i2loc = |i| Location::from_index(i, width);

        Grid {
            vec: (0..size).map(i2loc).map(f).collect(),
            height: height,
            width: width,
        }
    }

    // from here on out, functions only return valid locations

    pub fn get_cell(&self, loc: &Location) -> Option<&Cell> {
        let w = self.width as i32;
        if loc.x < 0 || loc.y < 0 || loc.x >= w {
            return None;
        }
        let i = (loc.y * w + loc.x) as usize;
        if i < self.vec.len() {
            self.vec[i].as_ref()
        } else {
            None
        }
    }

    fn locations_from_offsets<'a, I>(&self, loc: &Location, offsets: I) -> Vec<Location>
        where I: Iterator<Item = &'a Location>
    {
        offsets.map(|off| loc + &off)
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
}


#[cfg(test)]
use std::collections::HashSet;

#[cfg(test)]
impl Grid {
    pub fn is_connected(&self) -> bool {

        let first = self.locations_with_cells().next();

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
        let t = self.locations_with_cells().count();

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

    use proptest::prelude::*;
    use proptest::collection::vec;
    use proptest::option::weighted;


    #[test]
    fn test_connected() {

        let cell = Some(Cell { pin: 0 });
        let grid1 = Grid {
            width: 2,
            height: 2,
            vec: vec![None, cell, cell, None],
        };
        let grid2 = Grid {
            width: 2,
            height: 2,
            vec: vec![cell, cell, None, None],
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
        [pub] fn arb_grid (min_size: usize, max_size: usize, density: f64)
            (v in vec(weighted(density, arb_cell()), (min_size..max_size)))
            (h in 1..v.len()+1, mut v in Just(v))
             -> Grid
        {
            let height = h as u32;
            let width = 1 + (v.len() as u32 / height);

            while (v.len() as u32) < (height * width) {
                v.push(None);
            }

            Grid {
                vec: v,
                height: height,
                width: width
            }
        }
    }

    proptest! {
        #[test]
        fn grid_self_compatible(ref grid in arb_grid(1, 100, 0.5)) {
            let zero = Location {x: 0, y: 0};
            prop_assert!(grid.is_compatible_within(zero, &grid))
        }

        #[test]
        fn grid_self_place(ref grid in arb_grid(1, 100, 0.5)) {
            let num_cells = grid.locations_with_cells().count();
            prop_assume!( num_cells > 5 );

            let map = grid.place(&grid).unwrap();

            let my_locs: HashMap<Location, Location> = HashMap::from_iter(
                grid.locations_with_cells()
                    .map(|(loc, _)| (loc, loc))
            );
            prop_assert_eq!(&my_locs, &map);
        }
    }
}
