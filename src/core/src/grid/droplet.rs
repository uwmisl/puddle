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
    // TODO should droplets really know about their destinations?
    pub destination: Option<Location>,
    pub collision_group: usize,
}

#[derive(PartialEq, Eq, Hash, Debug, Serialize)]
pub struct DropletInfo {
    pub id: DropletId,
    pub location: Location,
    pub volume: i32,
    pub shape: Vec<Location>,
}

impl Droplet {
    pub fn new(id: DropletId, location: Location) -> Droplet {
        Droplet {
            id: id,
            location: location,
            destination: None,
            collision_group: NEXT_COLLISION_GROUP.fetch_add(1, Relaxed),
        }
    }

    pub fn info(&self) -> DropletInfo {
        DropletInfo {
            id: self.id,
            location: self.location,
            volume: 1,
            shape: vec![Location { y: 0, x: 0 }],
        }
    }
}
