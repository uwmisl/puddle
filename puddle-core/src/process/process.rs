use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

use crate::util::seconds_duration;

use crate::grid::{DropletId, DropletInfo, Location};
use crate::system::System;

use crate::command;
use crate::command::BoxedCommand;

use crate::plan::PlanError;

#[derive(Debug)]
pub enum PuddleError {
    PlanError(PlanError),
    NonExistentDropletId(usize),
    NonExistentProcess(ProcessId),
}

pub type PuddleResult<T> = Result<T, PuddleError>;

pub type ProcessId = usize;

pub struct Process {
    id: ProcessId,
    #[allow(dead_code)]
    name: String,
    next_droplet_id: AtomicUsize,
    system: Arc<Mutex<System>>,
    // TODO we probably want something like this for more precise flushing
    // unresolved_droplet_ids: Mutex<Set<DropletId>>,
}

static NEXT_PROCESS_ID: AtomicUsize = AtomicUsize::new(0);

impl Process {
    pub fn new(name: String, system: Arc<Mutex<System>>) -> Process {
        Process {
            id: NEXT_PROCESS_ID.fetch_add(1, Relaxed),
            name: name,
            next_droplet_id: AtomicUsize::new(0),
            system,
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

    fn plan(&self, cmd: BoxedCommand) -> PuddleResult<()> {
        let mut sys = self.system.lock().unwrap();
        sys.add(cmd)
    }
}

impl Process {
    pub fn flush(&self) -> PuddleResult<Vec<DropletInfo>> {
        let (tx, rx) = channel();
        let flush_cmd = command::Flush::new(self.id, tx);

        self.plan(Box::new(flush_cmd))?;
        self.system.lock().unwrap().flush(&[]).unwrap();
        rx.recv().unwrap().map_err(PuddleError::PlanError)
    }

    pub fn create(
        &self,
        loc: Option<Location>,
        vol: f64,
        dim: Option<Location>,
    ) -> PuddleResult<DropletId> {
        let output = self.new_droplet_id();
        let create_cmd = command::Create::new(loc, vol, dim, output)?;
        self.plan(Box::new(create_cmd))?;
        Ok(output)
    }

    pub fn input(
        &self,
        name: impl Into<String>,
        vol: f64,
        dim: Location,
    ) -> PuddleResult<DropletId> {
        let output = self.new_droplet_id();
        let input_cmd = command::Input::new(name.into(), vol, dim, output)?;
        self.plan(Box::new(input_cmd))?;
        Ok(output)
    }

    pub fn output(&self, name: impl Into<String>, d: DropletId) -> PuddleResult<()> {
        let output_cmd = command::Output::new(name.into(), d)?;
        self.plan(Box::new(output_cmd))?;
        Ok(())
    }

    pub fn move_droplet(&self, d1: DropletId, loc: Location) -> PuddleResult<DropletId> {
        let output = self.new_droplet_id();
        let move_cmd = command::Move::new(d1, loc, output)?;
        self.plan(Box::new(move_cmd))?;
        Ok(output)
    }

    pub fn mix(&self, d1: DropletId, d2: DropletId) -> PuddleResult<DropletId> {
        let combine_out = self.new_droplet_id();
        let combine_cmd = command::Combine::new(d1, d2, combine_out)?;
        self.plan(Box::new(combine_cmd))?;

        let agitate_out = self.new_droplet_id();
        let agitate_cmd = command::Agitate::new(combine_out, agitate_out)?;
        self.plan(Box::new(agitate_cmd))?;

        Ok(agitate_out)
    }

    pub fn combine_into(&self, d1: DropletId, d2: DropletId) -> PuddleResult<DropletId> {
        let output = self.new_droplet_id();
        let combine_cmd = command::Combine::combine_into(d1, d2, output)?;
        self.plan(Box::new(combine_cmd))?;
        Ok(output)
    }

    pub fn split(&self, d: DropletId) -> PuddleResult<(DropletId, DropletId)> {
        let out1 = self.new_droplet_id();
        let out2 = self.new_droplet_id();
        let split_cmd = command::Split::new(d, out1, out2)?;
        self.plan(Box::new(split_cmd))?;
        Ok((out1, out2))
    }

    pub fn heat(&self, d: DropletId, temperature: f32, seconds: f64) -> PuddleResult<DropletId> {
        let out = self.new_droplet_id();
        let duration = seconds_duration(seconds);
        let heat_cmd = command::Heat::new(d, out, temperature, duration)?;
        self.plan(Box::new(heat_cmd))?;
        Ok(out)
    }

    pub fn ticks(&self) -> usize {
        self.system.lock().unwrap().ticks()
    }
}

#[cfg(test)]
pub mod tests {
    // TODO do we need tests here?
}
