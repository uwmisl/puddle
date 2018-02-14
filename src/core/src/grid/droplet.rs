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

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub struct Droplet {
    // The droplet's id should never be modified once it has been created. They
    // are globally unique by construction.
    pub id: DropletId,
    pub location: Location,
    pub dimensions: Location,
    // TODO should droplets really know about their destinations?
    pub destination: Option<Location>,
    pub collision_group: usize,
}

#[derive(PartialEq, Eq, Hash, Debug, Serialize)]
pub struct DropletInfo {
    pub id: DropletId,
    pub location: Location,
    pub volume: i32,
    pub dimensions: Location,
}

impl Droplet {
    /// Creates a new Droplet given the Droplet ID, location, and dimensions.
    pub fn new(id: DropletId, location: Location, dimensions: Location) -> Droplet {
        if dimensions.y <= 0 || dimensions.x <= 0 {
            panic!("Dimensions for a droplet must be positive integers")
        }
        Droplet {
            id,
            location,
            dimensions,
            destination: None,
            collision_group: NEXT_COLLISION_GROUP.fetch_add(1, Relaxed),
        }
    }

    /// Returns a vector representing all locations this Droplet occupies
    // TODO does this need to be moved to `location.rs`?
    pub fn get_locations(location: &Location, dimensions: &Location) -> Vec<Location> {
        (0..dimensions.y)
            .flat_map(|y| (0..dimensions.x).map(move |x| &Location { y, x } + &location))
            .collect()
    }

    pub fn info(&self) -> DropletInfo {
        DropletInfo {
            id: self.id,
            location: self.location,
            volume: 1,
            dimensions: self.dimensions,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use uuid::Uuid;

    use super::{Droplet, DropletId, Location};

    #[test]
    fn test_valid_dimensions() {
        let dimensions = Location { y: 2, x: 1 };
        let droplet = Droplet::new(
            DropletId {
                id: 0,
                process_id: Uuid::new_v4(),
            },
            Location { y: 0, x: 0 },
            dimensions,
        );
        assert_eq!(
            Droplet::get_locations(&droplet.location, &dimensions).len(),
            2
        );
    }

    #[test]
    #[should_panic]
    fn test_invalid_dimensions() {
        Droplet::new(
            DropletId {
                id: 0,
                process_id: Uuid::new_v4(),
            },
            Location { y: 0, x: 0 },
            Location { y: 0, x: 0 },
        );
    }
}
