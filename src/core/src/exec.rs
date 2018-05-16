use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use grid::{DropletInfo, ExecResponse, GridView};
use util::endpoint::Endpoint;

pub struct Executor {
    blocking: bool,
    gridview: Arc<Mutex<GridView>>,
}

impl Executor {
    pub fn new(blocking: bool, gridview: Arc<Mutex<GridView>>) -> Self {
        Executor { blocking, gridview }
    }

    pub fn run(&mut self, endpoint: Endpoint<Vec<DropletInfo>, ()>) {
        let sleep_time = Duration::from_millis(100);

        loop {
            if self.blocking {
                // wait on the visualizer
                trace!("Receiving from visualizer...");
                match endpoint.recv() {
                    Ok(()) => trace!("Got the go ahead from the visualizer!"),
                    Err(_) => return,
                }
            }

            // if the lock was poisoned, the planner probably just died before we did
            let mut gv = match self.gridview.lock() {
                Ok(gv) => gv,
                Err(_) => return,
            };

            use self::ExecResponse::*;
            match gv.execute() {
                Step {} => {
                    // TODO the callbacks could probably be called by the gv itself
                    if self.blocking {
                        // only reply after we have the gridview lock
                        endpoint.send(gv.droplet_info(None)).unwrap()
                    }
                }
                NotReady => {
                    // drop the lock before sleeping
                    ::std::mem::drop(gv);
                    sleep(sleep_time);
                }
                Done => return,
            }
        }
    }
}
