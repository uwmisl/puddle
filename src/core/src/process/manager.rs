use std::ops::{Deref, DerefMut, Drop};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use exec::Executor;
use grid::{DropletInfo, Grid, GridView};
use process::{Process, ProcessId, PuddleError, PuddleResult};
use system::{System};

use util::collections::Map;
use util::endpoint::Endpoint;

pub struct ProcessHandle<'a> {
    process: Option<Process>,
    manager: &'a Manager,
}

impl<'a> Drop for ProcessHandle<'a> {
    fn drop(&mut self) {
        let p = self
            .process
            .take()
            .expect("ProcessHandle process was None!");
        self.manager.put_process(p);
    }
}

impl<'a> Deref for ProcessHandle<'a> {
    type Target = Process;
    fn deref(&self) -> &Self::Target {
        self.process
            .as_ref()
            .expect("ProcessHandle process was None!")
    }
}

impl<'a> DerefMut for ProcessHandle<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.process
            .as_mut()
            .expect("ProcessHandle process was None!")
    }
}

#[allow(dead_code)]
pub struct Manager {
    system: Arc<Mutex<System>>,
    processes: Mutex<Map<ProcessId, Process>>,
    blocking: bool,
}

impl Manager {
    pub fn new(blocking: bool, grid: Grid) -> Manager {

        let system = Arc::new(Mutex::new(System::new(grid)));

        Manager {
            system,
            blocking,
            processes: Mutex::new(Map::new()),
        }
    }

    // pub fn gridview(&self) -> MutexGuard<GridView> {
    //     self.gridview.lock().unwrap()
    // }

    fn take_process(&self, pid: ProcessId) -> PuddleResult<Process> {
        self.processes
            .lock()
            .unwrap()
            .remove(&pid)
            .ok_or_else(|| PuddleError::NonExistentProcess(pid))
    }

    fn put_process(&self, process: Process) {
        let old = self.processes.lock().unwrap().insert(process.id(), process);
        assert!(old.is_none());
    }

    pub fn get_process(&self, pid: ProcessId) -> PuddleResult<ProcessHandle> {
        let p = self.take_process(pid)?;
        Ok(ProcessHandle {
            process: Some(p),
            manager: self,
        })
    }

    pub fn new_process<S>(&self, name: S) -> PuddleResult<ProcessId>
    where
        S: Into<String>,
    {
        let system = Arc::clone(&self.system);
        let process = Process::new(name.into(), system);
        let pid = process.id();
        let mut procs = self.processes.lock().unwrap();
        procs.insert(pid, process);
        Ok(pid)
    }

    pub fn close_process(&self, pid: ProcessId) -> PuddleResult<()> {
        let p = self.take_process(pid)?;
        p.flush()?;
        Ok(())
    }

    pub fn get_new_process<S>(&self, name: S) -> ProcessHandle
    where
        S: Into<String>,
    {
        let pid = self.new_process(name).expect("creation failed");
        self.get_process(pid).expect("get failed")
    }

    pub fn visualizer_droplet_info(&self) -> PuddleResult<Vec<DropletInfo>> {
        unimplemented!()
        // // DONT FLUSH
        // let endp = self.exec_endpoint.lock().unwrap();
        // endp.send(()).unwrap();
        // let info = endp.recv().unwrap();
        // Ok(info)
    }

}
