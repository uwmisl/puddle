
use std::ops::{Add, Sub};
use std::collections::{HashSet};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
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

#[derive(PartialEq, Eq, Hash)]
pub struct Droplet {
    pub location: Location,
    pub destination: Location,
}

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
    pub fn rectangle(h: u32, w: u32) -> Grid {
        let always_cell = |_| Some(Cell {});
        Grid::from_function(always_cell, h, w)
    }

    pub fn from_function<F>(f: F, height: u32, width: u32) -> Grid
        where F: FnMut(Location) -> Option<Cell>
    {

        let size = height * width;

        let i2loc = |i| {
            Location {
                y: (i / width) as i32,
                x: (i % width) as i32,
            }
        };

        Grid {
            vec: (0..size).map(i2loc).map(f).collect(),
            height: height,
            width: width,
        }
    }

    // from here on out, functions only return valid locations

    pub fn get(&self, loc: &Location) -> Option<&Cell> {
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
            .filter(|loc| self.get(loc).is_some())
            .collect()
    }

    pub fn neighbors(&self, loc: &Location) -> Vec<Location> {
        self.locations_from_offsets(loc, NEIGHBORS_4.into_iter())
    }
}


pub struct Cell {}


pub struct Architecture {
    pub grid: Grid,
    pub droplets: HashSet<Droplet>,
}
