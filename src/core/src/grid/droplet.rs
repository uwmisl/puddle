use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;

use process::ProcessId;
use super::Location;

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

#[derive(Debug, Serialize)]
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
