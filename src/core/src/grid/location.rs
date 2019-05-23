use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use derive_more::{Add, Display, From, Sub};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[derive(Serialize, Deserialize)] // from serde_derive
#[derive(From, Display, Add, Sub)] // from derive_more
#[display(fmt = "({}, {})", y, x)]
pub struct Location {
    pub y: i32,
    pub x: i32,
}

impl Location {
    pub fn distance_to(self, other: Self) -> u32 {
        (self - other).norm()
    }
    pub fn norm(self) -> u32 {
        (self.y.abs() + self.x.abs()) as u32
    }

    pub fn north(self) -> Location {
        Location {
            y: self.y - 1,
            x: self.x,
        }
    }

    pub fn west(self) -> Location {
        Location {
            y: self.y,
            x: self.x - 1,
        }
    }

    pub fn south(self) -> Location {
        Location {
            y: self.y + 1,
            x: self.x,
        }
    }

    pub fn east(self) -> Location {
        Location {
            y: self.y,
            x: self.x + 1,
        }
    }
}

impl fmt::Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl FromStr for Location {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords: Vec<&str> = s
            .trim()
            .trim_matches(|p| p == '(' || p == ')')
            .split(',')
            .map(|s| s.trim())
            .collect();

        if coords.len() != 2 {
            panic!("A location is 2 comma-separated ints. Given: '{}'", s)
        }

        let y = coords[0].parse()?;
        let x = coords[1].parse()?;

        Ok(Location { y, x })
    }
}

#[derive(Clone)]
pub struct Rectangle {
    pub location: Location,
    pub dimensions: Location,
}

impl Rectangle {
    pub fn new(location: impl Into<Location>, dimensions: impl Into<Location>) -> Rectangle {
        Rectangle {
            location: location.into(),
            dimensions: dimensions.into(),
        }
    }

    fn top_edge(&self) -> i32 {
        self.location.y
    }

    fn bottom_edge(&self) -> i32 {
        self.location.y + self.dimensions.y
    }

    fn left_edge(&self) -> i32 {
        self.location.x
    }

    fn right_edge(&self) -> i32 {
        self.location.x + self.dimensions.x
    }

    pub fn collision_distance(&self, other: &Rectangle) -> i32 {
        fn signed_min(a: i32, b: i32) -> i32 {
            if (a < 0) == (b < 0) {
                i32::min(a.abs(), b.abs())
            } else {
                -i32::min(a.abs(), b.abs())
            }
        }
        let y_dist = signed_min(
            self.bottom_edge() - other.top_edge(),
            self.top_edge() - other.bottom_edge(),
        );
        let x_dist = signed_min(
            self.right_edge() - other.left_edge(),
            self.left_edge() - other.right_edge(),
        );

        y_dist.max(x_dist)
    }

    pub fn locations(self) -> impl Iterator<Item = Location> {
        let ys = 0..(self.dimensions.y);
        ys.flat_map(move |y| {
            let xs = 0..(self.dimensions.x);
            let base = self.location;
            xs.map(move |x| base + Location { y, x })
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::util::HashMap;

    use ena::unify::{InPlaceUnificationTable, UnifyKey};

    #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
    struct IntKey(u32);

    impl UnifyKey for IntKey {
        type Value = ();
        fn index(&self) -> u32 {
            self.0
        }
        fn from_index(u: u32) -> IntKey {
            IntKey(u)
        }
        fn tag() -> &'static str {
            "IntKey"
        }
    }

    pub fn connected_components<'a, I>(locs: I) -> HashMap<Location, u32>
    where
        I: Iterator<Item = Location>,
    {
        // inputs must be in row major order
        let mut labels = HashMap::default();
        let mut equivs = InPlaceUnificationTable::<IntKey>::new();

        for loc in locs {
            let l_north = labels.get(&loc.north()).cloned();
            let l_west = labels.get(&loc.west()).cloned();

            let label = match (l_north, l_west) {
                (None, None) => equivs.new_key(()),
                (None, Some(l)) => l,
                (Some(l), None) => l,
                (Some(l1), Some(l2)) => {
                    equivs.union(l1, l2);
                    l2 // could be l1 too, doesn't matter
                }
            };

            labels.insert(loc, label);
        }

        // return all the locations associated with their root key
        labels
            .iter()
            .map(|(k, v)| {
                let vv = equivs.find(*v).index();
                (*k, vv)
            })
            .collect()
    }

    #[test]
    fn test_connected_components() {
        // check that diagonal is not connected, but adjacent is
        let la = [
            Location { y: 0, x: 0 },
            Location { y: 0, x: 1 },
            Location { y: 1, x: 2 },
        ];
        let ca = connected_components(la.iter().cloned());
        assert_eq!(ca[&la[0]], ca[&la[1]]);
        assert_ne!(ca[&la[1]], ca[&la[2]]);

        // check that a connected shape is connected
        let lb = [
            Location { y: 0, x: 0 },
            Location { y: 0, x: 1 },
            Location { y: 1, x: 0 },
            Location { y: 2, x: 0 },
            Location { y: 2, x: 1 },
        ];
        let cb = connected_components(lb.iter().cloned());
        assert!(cb.values().all(|v| *v == 0));
    }

    fn check_dist(r1: Rectangle, r2: Rectangle, expected: i32) {
        let actual1 = r1.collision_distance(&r2);
        let actual2 = r2.collision_distance(&r1);

        assert_eq!(actual1, actual2);
        assert_eq!(actual1, expected);
    }

    #[test]
    fn test_rectangle_distance() {
        // simple test
        // a.b
        check_dist(
            Rectangle::new((0, 0), (1, 1)),
            Rectangle::new((2, 0), (1, 1)),
            1,
        );

        // diagonals should still collide
        // a.
        // .b
        check_dist(
            Rectangle::new((0, 0), (1, 1)),
            Rectangle::new((1, 1), (1, 1)),
            0,
        );

        // larger things
        // ....aa....
        // ....aa....
        // ..........
        // ..........
        // ...bbbb...
        check_dist(
            Rectangle::new((0, 4), (2, 2)),
            Rectangle::new((4, 3), (1, 4)),
            2,
        );

        // overlap
        // ....aa....
        // ....aXbb..
        check_dist(
            Rectangle::new((0, 4), (2, 2)),
            Rectangle::new((1, 5), (1, 3)),
            -1,
        );
    }
}
