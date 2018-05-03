use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;

use grid::{DropletId, DropletInfo, Location};

use command;
use command::Command;

use plan::{PlanError, Planner};

#[derive(Debug)]
pub enum PuddleError {
    PlanError(PlanError),
    NonExistentDropletId(usize),
    NonExistentProcess(ProcessId),
}

use PuddleError::*;

pub type PuddleResult<T> = Result<T, PuddleError>;

pub type ProcessId = usize;

pub struct Process {
    id: ProcessId,
    #[allow(dead_code)]
    name: String,
    next_droplet_id: AtomicUsize,
    planner: Arc<Mutex<Planner>>,
    // TODO we probably want something like this for more precise flushing
    // unresolved_droplet_ids: Mutex<Set<DropletId>>,
}

static NEXT_PROCESS_ID: AtomicUsize = AtomicUsize::new(0);

impl Process {
    pub fn new(name: String, planner: Arc<Mutex<Planner>>) -> Process {
        Process {
            id: NEXT_PROCESS_ID.fetch_add(1, Relaxed),
            name: name,
            next_droplet_id: AtomicUsize::new(0),
            planner: planner,
        }
    }

    pub fn id(&self) -> ProcessId {
        self.id
    }

    fn new_droplet_id(&self) -> DropletId {
        DropletId {
            id: self.next_droplet_id.fetch_add(1, Relaxed),
            process_id: self.id,
        }
    }

    fn plan<C: Command>(&self, cmd: C) -> PuddleResult<()> {
        let mut planner = self.planner.lock().unwrap();
        planner.plan(cmd).map_err(PlanError)
    }
}

impl Process {
    pub fn flush(&self) -> PuddleResult<Vec<DropletInfo>> {
        let (tx, rx) = channel();
        let flush_cmd = command::Flush::new(self.id, tx);

        self.plan(flush_cmd)?;
        let info = rx.recv().unwrap();

        Ok(info)
    }

    pub fn input(
        &self,
        loc: Option<Location>,
        vol: f64,
        dim: Option<Location>,
    ) -> PuddleResult<DropletId> {
        let output = self.new_droplet_id();
        let input_cmd = command::Input::new(loc, vol, dim, output)?;
        self.plan(input_cmd)?;
        Ok(output)
    }

    pub fn move_droplet(&self, d1: DropletId, loc: Location) -> PuddleResult<DropletId> {
        let output = self.new_droplet_id();
        let move_cmd = command::Move::new(d1, loc, output)?;
        self.plan(move_cmd)?;
        Ok(output)
    }

    pub fn mix(&self, d1: DropletId, d2: DropletId) -> PuddleResult<DropletId> {
        let output = self.new_droplet_id();
        let mix_cmd = command::Mix::new(d1, d2, output)?;
        self.plan(mix_cmd)?;
        Ok(output)
    }

    pub fn split(&self, d: DropletId) -> PuddleResult<(DropletId, DropletId)> {
        let out1 = self.new_droplet_id();
        let out2 = self.new_droplet_id();
        let split_cmd = command::Split::new(d, out1, out2)?;
        self.plan(split_cmd)?;
        Ok((out1, out2))
    }
}

#[cfg(test)]
pub mod tests {
    // TODO do we need tests here?
}
