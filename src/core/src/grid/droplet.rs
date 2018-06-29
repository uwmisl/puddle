use std::collections::HashSet;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;

use super::Location;
use process::ProcessId;

static NEXT_COLLISION_GROUP: AtomicUsize = AtomicUsize::new(0);

#[derive(PartialEq, Eq, PartialOrd, Hash, Ord, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DropletId {
    pub id: usize,
    pub process_id: ProcessId,
}

#[derive(Debug, Clone)]
pub struct Droplet {
    // The droplet's id should never be modified once it has been created. They
    // are globally unique by construction.
    pub id: DropletId,
    pub location: Location,
    pub dimensions: Location,
    pub volume: f64,
    // TODO should droplets really know about their destinations?
    pub destination: Option<Location>,
    pub collision_group: usize,
}

// derive PartialEq because Droplets don't, and it's useful to compare them.
// comparing the info is a safer way to do so
#[derive(Debug, Serialize, PartialEq)]
pub struct DropletInfo {
    pub id: DropletId,
    pub location: Location,
    pub volume: f64,
    pub dimensions: Location,
}

impl Droplet {
    /// Creates a new Droplet given the Droplet ID, location, and dimensions.
    pub fn new(id: DropletId, volume: f64, location: Location, dimensions: Location) -> Droplet {
        if dimensions.y <= 0 || dimensions.x <= 0 {
            panic!("Dimensions for a droplet must be positive integers")
        }
        Droplet {
            id,
            location,
            dimensions,
            destination: None,
            volume: volume,
            collision_group: NEXT_COLLISION_GROUP.fetch_add(1, Relaxed),
        }
    }

    fn corners(&self) -> [Location; 4] {
        [
            self.location,
            // subtract one, because the unit square is account for by
            // min_distance_to_box
            &self.location + &Location {
                y: self.dimensions.y - 1,
                x: 0,
            },
            &self.location + &Location {
                y: 0,
                x: self.dimensions.x - 1,
            },
            &self.location + &Location {
                y: self.dimensions.y - 1,
                x: self.dimensions.x - 1,
            },
        ]
    }

    pub fn collision_distance(&self, other: &Droplet) -> i32 {
        let my_corners = self.corners();
        let their_corners = other.corners();

        let d1 = my_corners
            .iter()
            .map(|mine| mine.min_distance_to_box(their_corners[0], their_corners[3]))
            .min()
            .unwrap();

        if d1 < 0 {
            return d1;
        }

        let d2 = their_corners
            .iter()
            .map(|theirs| theirs.min_distance_to_box(my_corners[0], my_corners[3]))
            .min()
            .unwrap();

        d1.min(d2)
    }

    pub fn info(&self) -> DropletInfo {
        DropletInfo {
            id: self.id,
            location: self.location,
            dimensions: self.dimensions,
            volume: self.volume,
        }
    }

    pub fn to_blob(&self) -> SimpleBlob {
        SimpleBlob {
            location: self.location,
            dimensions: self.dimensions,
            volume: self.volume,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SimpleBlob {
    pub location: Location,
    pub dimensions: Location,
    pub volume: f64,
}

pub trait Blob: Clone {
    fn get_similarity(&self, droplet: &Droplet) -> i32;
    fn to_droplet(&self, id: DropletId) -> Droplet;
}

// impl PartialEq for Blob {
//     fn eq(&self, other: &Blob) -> bool {
//         self.location == other.location && self.dimensions == other.dimensions
//             && float_epsilon_equal(self.volume, other.volume)
//     }
// }

fn float_epsilon_equal(float1: f64, float2: f64) -> bool {
    let epsilon = 0.00001f64;
    (float1 - float2).abs() < epsilon
}

impl SimpleBlob {
    pub fn from_locations(locs: &[Location]) -> Option<SimpleBlob> {
        let location = Location {
            y: locs.iter().map(|l| l.y).min().unwrap_or(0),
            x: locs.iter().map(|l| l.x).min().unwrap_or(0),
        };
        let far_corner = Location {
            y: locs.iter().map(|l| l.y).max().unwrap_or(0) + 1,
            x: locs.iter().map(|l| l.x).max().unwrap_or(0) + 1,
        };
        let dimensions = &far_corner - &location;

        let set1: HashSet<Location> = locs.iter().cloned().collect();
        let mut set2 = HashSet::new();

        // build set2 with all the locations the rectangle should have
        for i in 0..dimensions.y {
            for j in 0..dimensions.x {
                set2.insert(Location {
                    y: location.y + i,
                    x: location.x + j,
                });
            }
        }

        // using dimensions as volume for now
        let volume: f64 = (dimensions.x * dimensions.y).into();

        if set1 == set2 {
            Some(SimpleBlob {
                location,
                dimensions,
                volume,
            })
        } else {
            None
        }
    }
}

impl Blob for SimpleBlob {
    fn get_similarity(&self, droplet: &Droplet) -> i32 {
        self.location.distance_to(&droplet.location) as i32
            + self.dimensions.distance_to(&droplet.dimensions) as i32
            + ((self.volume - droplet.volume) as i32).abs()
    }

    fn to_droplet(&self, id: DropletId) -> Droplet {
        Droplet::new(id, self.volume, self.location, self.dimensions)
    }
}

#[cfg(test)]
pub mod tests {
    use super::{Droplet, DropletId, Location};

    #[test]
    #[should_panic]
    fn test_invalid_dimensions() {
        Droplet::new(
            DropletId {
                id: 0,
                process_id: 0,
            },
            1.0,
            Location { y: 0, x: 0 },
            Location { y: 0, x: 0 },
        );
    }
}
