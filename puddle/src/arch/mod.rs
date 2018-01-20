pub mod parse;

use std::ops::{Add, Sub};
use std::collections::{HashMap, HashSet};

use self::parse::ParsedGrid;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone,
         Copy, Debug, Serialize, Deserialize)]
pub struct Location {
    pub x: i32,
    pub y: i32,
}

impl Location {
    pub fn distance_to(&self, other: &Self) -> u32 {
        (self - other).norm()
    }
    pub fn norm(&self) -> u32 {
        (self.y.abs() + self.x.abs()) as u32
    }
    pub fn from_index(i: u32, width: u32) -> Location {
        Location {
            y: (i / width) as i32,
            x: (i % width) as i32,
        }
    }
}

impl<'a> Add for &'a Location {
    type Output = Location;
    fn add(self, other: &Location) -> Location {
        Location {
            y: self.y + other.y,
            x: self.x + other.x,
        }
    }
}

impl<'a> Sub for &'a Location {
    type Output = Location;
    fn sub(self, other: &Location) -> Location {
        Location {
            y: self.y - other.y,
            x: self.x - other.x,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Cell {
    pin: u32,
}

impl Cell {
    #[allow(unused_variables)]
    fn is_compatible(&self, other: &Self) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct Grid {
    vec: Vec<Option<Cell>>,
    height: u32,
    width: u32,
}

#[cfg_attr(rustfmt, rustfmt_skip)]
#[allow(dead_code)]
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

        use self::parse::CellIndex::*;
        use self::parse::Mark::*;

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

    fn locations<'a>(&'a self) -> Box<Iterator<Item = (Location, Cell)> + 'a> {
        // TODO this is a little ugly, maybe a custom iterator could be better
        let size = self.vec.len();
        let iter = (0..size)
            .map(move |i| Location::from_index(i as u32, self.width))
            .filter_map(move |loc| {
                self.get_cell(&loc)
                    .map(|cell| (loc, *cell))
            });
        Box::new(iter)
    }


    /// Tests if this grid is compatible within `bigger` when `offset` is applied
    /// to `self`
    fn is_compatible_within(&self, offset: Location, bigger: &Self) -> bool {
        self.locations()
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

        for (loc, _) in self.locations() {
            map.insert(loc, &loc + &offset);
        }

        map
    }

    fn place(&self, smaller: &Self) -> Option<HashMap<Location, Location>> {
        self.locations()
            .map(|(loc, _)| loc)
            .find(|&offset| smaller.is_compatible_within(offset, self))
            .map(|offset| smaller.mapping_into_other_from_offset(offset, self))
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
        if loc.x < 0 || loc.y < 0 {
            return None;
        }
        let i = (loc.y * w + loc.x) as usize;
        if 0 < i && i < self.vec.len() {
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

    pub fn neighbors(&self, loc: &Location) -> Vec<Location> {
        self.locations_from_offsets(loc, NEIGHBORS_4.into_iter())
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct Droplet {
    pub location: Location,
    pub destination: Location,
}

pub struct Architecture {
    pub grid: Grid,
    pub droplets: HashSet<Droplet>,
}



#[cfg(test)]
mod tests {

}
