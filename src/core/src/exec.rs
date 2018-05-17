use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use rand::Rng;
use rand::prelude::thread_rng;

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

    pub fn run(&self, endpoint: Endpoint<Vec<DropletInfo>, ()>) {
        let sleep_time = Duration::from_millis(100);

        let mut rng = thread_rng();

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
                    if self.blocking {
                        endpoint.send(gv.exec_droplet_info(None)).unwrap()
                    }

                    let should_perturb = rng.gen_bool(0.0);
                    if should_perturb {
                        if let Some(new_snapshot) = gv.perturb(&mut rng) {
                            info!("Simulating an error...");
                            gv.rollback(new_snapshot);
                        }
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
