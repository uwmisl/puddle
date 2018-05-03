use std::ops::{Add, Sub};
use std::fmt;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Location {
    pub y: i32,
    pub x: i32,
}

impl Location {
    pub fn distance_to(&self, other: &Self) -> u32 {
        (self - other).norm()
    }
    pub fn norm(&self) -> u32 {
        (self.y.abs() + self.x.abs()) as u32
    }

    pub fn min_distance_to_box(&self, corner1: Location, corner2: Location) -> i32 {
        assert!(corner1.x <= corner2.x);
        assert!(corner1.y <= corner2.y);

        // these ds are negative if the point is inside, 0 if on boundary, else positive
        let dy = i32::max(corner1.y - (self.y + 1), self.y - corner2.y);
        let dx = i32::max(corner1.x - (self.x + 1), self.x - corner2.x);

        if dy < 0 && dx < 0 {
            -1
        } else {
            dy.max(0) + dx.max(0)
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

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.y, self.x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Pt = (i32, i32);
    fn dist_to_box(p: Pt, c1: Pt, c2: Pt) -> i32 {
        Location { y: p.0, x: p.1 }
            .min_distance_to_box(Location { y: c1.0, x: c1.1 }, Location { y: c2.0, x: c2.1 })
    }

    #[test]
    fn test_min_distance_to_box() {
        assert_eq!(dist_to_box((0, 0), (1, 1), (2, 2)), 0);
        assert_eq!(dist_to_box((1, 1), (1, 1), (2, 2)), -1);
        assert_eq!(dist_to_box((0, 0), (2, 2), (3, 3)), 2);
        assert_eq!(dist_to_box((1, 0), (2, 2), (3, 3)), 1);

        // the point intersects with the right side of the box
        assert_eq!(dist_to_box((0, 2), (1, 1), (2, 2)), 0);
        assert_eq!(dist_to_box((1, 2), (1, 1), (2, 2)), 0);
        assert_eq!(dist_to_box((2, 2), (1, 1), (2, 2)), 0);
    }
}
