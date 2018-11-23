use std::fmt;
use std::collections::HashSet;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;

use super::Location;
use process::ProcessId;

static NEXT_COLLISION_GROUP: AtomicUsize = AtomicUsize::new(0);

#[derive(PartialEq, Eq, PartialOrd, Hash, Ord, Clone, Copy, Serialize, Deserialize)]
pub struct DropletId {
    pub id: usize,
    pub process_id: ProcessId,
}

impl fmt::Debug for DropletId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)?;
        if self.process_id != 0 {
            write!(f, "p{}", self.process_id)?;
        }
        Ok(())
    }
}

#[cfg(test)]
impl From<usize> for DropletId {
    fn from(id: usize) -> DropletId {
        DropletId {id, process_id: 0}
    }
}

#[derive(Debug, Clone)]
pub struct Droplet {
    // The droplet's id should never be modified once it has been created. They
    // are globally unique by construction.
    pub id: DropletId,
    pub location: Location,
    pub dimensions: Location,
    pub volume: f64,

    // all this stuff is used for routing
    // TODO should droplets really know about their destinations?
    pub destination: Option<Location>,
    pub collision_group: usize,
    pub pinned: bool,
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
            pinned: false,
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

    pub fn collision_distance(&self, other: &Droplet) -> i32 {
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

fn signed_min(a: i32, b: i32) -> i32 {
    let res = if (a < 0) == (b < 0) {
        i32::min(a.abs(), b.abs())
    } else {
        -i32::min(a.abs(), b.abs())
    };
    trace!("signed min({}, {}) = {}", a, b, res);
    res
}

impl Default for Droplet {
    fn default() -> Self {
        // for locations, just use something bad that will crash if not replaced
        let bad_loc = Location { y: -1, x: -1 };
        let bad_id = DropletId {
            id: 0xc0ffee,
            process_id: 0xc0ffee,
        };
        Droplet {
            id: bad_id,
            location: bad_loc,
            dimensions: bad_loc,
            pinned: false,
            volume: 1.0,
            destination: None,
            collision_group: NEXT_COLLISION_GROUP.fetch_add(1, Relaxed),
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
    fn to_simple_blob(&self) -> SimpleBlob;
    fn to_droplet(&self, id: DropletId) -> Droplet {
        let simple_blob = self.to_simple_blob();
        Droplet::new(
            id,
            simple_blob.volume,
            simple_blob.location,
            simple_blob.dimensions,
        )
    }
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

    fn to_simple_blob(&self) -> SimpleBlob {
        self.clone()
    }
}

#[cfg(test)]
pub mod tests {
    use super::{Droplet, DropletId, Location};

    use env_logger;

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

    fn droplet_with_shape(loc: (i32, i32), dim: (i32, i32)) -> Droplet {
        Droplet {
            location: Location { y: loc.0, x: loc.1 },
            dimensions: Location { y: dim.0, x: dim.1 },
            ..Droplet::default()
        }
    }

    #[test]
    fn test_collision_distance() {
        let _ = env_logger::try_init();

        let a = droplet_with_shape((0, 2), (1, 1));
        let b = droplet_with_shape((2, 0), (1, 1));
        assert_eq!(a.collision_distance(&b), 1);

        let a = droplet_with_shape((0, 0), (3, 1));
        let b = droplet_with_shape((4, 0), (1, 1));
        assert_eq!(a.collision_distance(&b), 1);

        let a = droplet_with_shape((2, 7), (3, 1));
        let b = droplet_with_shape((0, 8), (3, 1));
        assert_eq!(a.collision_distance(&b), 0);
    }
}
