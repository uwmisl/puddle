use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use std::ops::{Deref, DerefMut, Drop};

use process::{Process, ProcessId, PuddleError, PuddleResult};
use grid::{DropletInfo, Grid, GridView};
use exec::Executor;

use util::collections::Map;
use util::endpoint::Endpoint;

use plan::Planner;

pub struct ProcessHandle<'a> {
    process: Option<Process>,
    manager: &'a Manager,
}

impl<'a> Drop for ProcessHandle<'a> {
    fn drop(&mut self) {
        let p = self.process
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
    processes: Mutex<Map<ProcessId, Process>>,
    planner: Arc<Mutex<Planner>>,
    exec_endpoint: Mutex<Endpoint<(), Vec<DropletInfo>>>,
    exec_thread: thread::JoinHandle<()>,
    blocking: bool,
}

// TODO impl drop

impl Manager {
    pub fn new(blocking: bool, grid: Grid) -> Manager {
        let (cmd_tx, cmd_rx) = channel();
        let (mine, execs) = Endpoint::pair();

        let mut executor = Executor::new(blocking, grid.clone());

        let exec_thread = thread::Builder::new()
            .name("exec".into())
            .spawn(move || executor.run(cmd_rx, execs))
            .expect("Execution thread failed to start!");

        let gridview = GridView::new(grid);
        let planner = Planner::new(gridview, cmd_tx);

        Manager {
            exec_thread: exec_thread,
            processes: Mutex::new(Map::new()),
            exec_endpoint: Mutex::new(mine),
            planner: Arc::new(Mutex::new(planner)),
            blocking: blocking,
        }
    }

    fn take_process(&self, pid: ProcessId) -> PuddleResult<Process> {
        self.processes
            .lock()
            .unwrap()
            .remove(&pid)
            .ok_or(PuddleError::NonExistentProcess(pid))
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
        let planner = Arc::clone(&self.planner);
        let process = Process::new(name.into(), planner);
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

    pub fn get_new_process<S>(&mut self, name: S) -> ProcessHandle
    where
        S: Into<String>,
    {
        let pid = self.new_process(name).expect("creation failed");
        self.get_process(pid).expect("get failed")
    }

    pub fn visualizer_droplet_info(&self) -> PuddleResult<Vec<DropletInfo>> {
        // DONT FLUSH
        let endp = self.exec_endpoint.lock().unwrap();
        endp.send(()).unwrap();
        let info = endp.recv().unwrap();
        Ok(info)
    }
}
