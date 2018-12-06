use std::fmt;
use std::num::ParseIntError;
use std::ops::{Add, Sub};
use std::str::FromStr;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize)]
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

    pub fn north(&self) -> Location {
        Location {
            y: self.y - 1,
            x: self.x,
        }
    }

    pub fn west(&self) -> Location {
        Location {
            y: self.y,
            x: self.x - 1,
        }
    }

    pub fn south(&self) -> Location {
        Location {
            y: self.y + 1,
            x: self.x,
        }
    }

    pub fn east(&self) -> Location {
        Location {
            y: self.y,
            x: self.x + 1,
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

impl fmt::Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.y, self.x)
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.y, self.x)
    }
}

impl From<(i32, i32)> for Location {
    fn from(tuple: (i32, i32)) -> Location {
        Location {
            y: tuple.0,
            x: tuple.1,
        }
    }
}

impl FromStr for Location {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords: Vec<&str> = s
            .trim()
            .trim_matches(|p| p == '(' || p == ')')
            .split(",")
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

pub struct Rectangle {
    pub location: Location,
    pub dimensions: Location,
}

impl Rectangle {
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
        let y_dist = signed_min(
            self.bottom_edge() - other.top_edge(),
            self.top_edge() - other.bottom_edge(),
        );
        let x_dist = signed_min(
            self.right_edge() - other.left_edge(),
            self.left_edge() - other.right_edge(),
        );

        return y_dist.max(x_dist);
    }

    pub fn locations(self) -> impl Iterator<Item = Location> {
        let ys = 0..(self.dimensions.y);
        ys.flat_map(move |y| {
            let xs = 0..(self.dimensions.x);
            let base = self.location;
            xs.map(move |x| &base + &Location { y, x })
        })
    }
}

fn signed_min(a: i32, b: i32) -> i32 {
    let res = if (a < 0) == (b < 0) {
        i32::min(a.abs(), b.abs())
    } else {
        -i32::min(a.abs(), b.abs())
    };
    trace!("signed min({}, {}) = {}", a, b, res);
    res
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::HashMap;

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
        let mut labels = HashMap::new();
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
}
