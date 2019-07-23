use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)] // std
#[derive(Serialize, Deserialize)] // serde
pub struct Location {
    pub y: i32,
    pub x: i32,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl std::ops::Add for Location {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        yx(self.y + other.y, self.x + other.x)
    }
}

impl std::ops::Sub for Location {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        yx(self.y - other.y, self.x - other.x)
    }
}

#[inline(always)]
pub const fn yx(y: i32, x: i32) -> Location {
    Location { y, x }
}

impl Location {
    pub fn distance_to(self, other: Self) -> u32 {
        (self - other).norm()
    }

    pub fn norm(self) -> u32 {
        (self.y.abs() + self.x.abs()) as u32
    }

    pub fn north(self) -> Location {
        self + yx(-1, 0)
    }

    pub fn west(self) -> Location {
        self + yx(0, -1)
    }

    pub fn south(self) -> Location {
        self + yx(1, 0)
    }

    pub fn east(self) -> Location {
        self + yx(0, 1)
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
            .map(str::trim)
            .collect();

        if coords.len() != 2 {
            panic!("A location is 2 comma-separated ints. Given: '{}'", s)
        }

        let y = coords[0].parse()?;
        let x = coords[1].parse()?;

        Ok(yx(y, x))
    }
}

#[derive(Clone)]
pub struct Rectangle {
    pub location: Location,
    pub dimensions: Location,
}

impl Rectangle {
    pub fn new(location: Location, dimensions: Location) -> Rectangle {
        Rectangle {
            location,
            dimensions,
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
            xs.map(move |x| base + yx(y, x))
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

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
            Rectangle::new(yx(0, 0), yx(1, 1)),
            Rectangle::new(yx(2, 0), yx(1, 1)),
            1,
        );

        // diagonals should still collide
        // a.
        // .b
        check_dist(
            Rectangle::new(yx(0, 0), yx(1, 1)),
            Rectangle::new(yx(1, 1), yx(1, 1)),
            0,
        );

        // larger things
        // ....aa....
        // ....aa....
        // ..........
        // ..........
        // ...bbbb...
        check_dist(
            Rectangle::new(yx(0, 4), yx(2, 2)),
            Rectangle::new(yx(4, 3), yx(1, 4)),
            2,
        );

        // overlap
        // ....aa....
        // ....aXbb..
        check_dist(
            Rectangle::new(yx(0, 4), yx(2, 2)),
            Rectangle::new(yx(1, 5), yx(1, 3)),
            -1,
        );
    }
}
