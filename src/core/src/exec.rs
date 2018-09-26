use std::env;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use rand::Rng;

use grid::{DropletInfo, ExecResponse, GridView};
use util::endpoint::Endpoint;
use util::mk_rng;

/// delay between steps in milliseconds
#[cfg(feature = "pi")]
static STEP_DELAY: u64 = 100;
#[cfg(not(feature = "pi"))]
static STEP_DELAY: u64 = 1;

pub struct Executor {
    blocking: bool,
    gridview: Arc<Mutex<GridView>>,
}

impl Executor {
    pub fn new(blocking: bool, gridview: Arc<Mutex<GridView>>) -> Self {
        Executor { blocking, gridview }
    }

    pub fn run(&mut self, endpoint: Endpoint<Vec<DropletInfo>, ()>) {
        let sleep_ms = env::var("PUDDLE_STEP_DELAY_MS")
            .ok()
            .map(|s| u64::from_str_radix(&s, 10).expect("Couldn't parse!"))
            .unwrap_or(STEP_DELAY);
        let sleep_time = Duration::from_millis(sleep_ms);

        let mut rng = mk_rng();

        #[cfg(feature = "vision")]
        #[allow(unused_variables)]
        let blobs = {
            use std::thread;
            use vision::Detector;
            let blobs = Arc::default();
            let blob_ref = Arc::clone(&blobs);
            let trackbars = false;
            let should_draw = true;
            let det_thread = thread::Builder::new()
                .name("detector".into())
                .spawn(move || {
                    let mut detector = Detector::new(trackbars);
                    detector.run(should_draw, blob_ref)
                });
            blobs
        };

        let err_rate = env::var("PUDDLE_SIMULATE_ERROR")
            .map(|s| s.parse::<f64>().unwrap())
            .unwrap_or(0.0);

        let should_correct = {
            let n = env::var("PUDDLE_CORRECT_ERRORS")
                .map(|s| s.parse::<i32>().unwrap())
                .unwrap_or(1);
            match n {
                0 => false,
                1 => true,
                _ => panic!("couldn't parse PUDDLE_CORRECT_ERRORS"),
            }
        };

        let should_add_edges = {
            let n = env::var("PUDDLE_BAD_EDGES")
                .map(|s| s.parse::<i32>().unwrap())
                .unwrap_or(1);
            match n {
                0 => false,
                1 => true,
                _ => panic!("couldn't parse PUDDLE_BAD_EDGES"),
            }
        };

        loop {
            if self.blocking {
                // wait on the visualizer
                trace!("Receiving from visualizer...");
                match endpoint.recv() {
                    Ok(()) => trace!("Got the go ahead from the visualizer!"),
                    Err(_) => break,
                }
            }

            // if the lock was poisoned, the planner probably just died before we did
            sleep(sleep_time);
            let mut gv = match self.gridview.lock() {
                Ok(gv) => gv,
                Err(_) => break,
            };

            use self::ExecResponse::*;
            match gv.execute() {
                Step(mut snapshot) => {
                    if self.blocking {
                        endpoint.send(snapshot.droplet_info(None)).unwrap()
                    }

                    #[cfg(feature = "pi")]
                    {
                        // must `take` the pi out of the gv temporarily so we
                        // can use &gv.grid immutably
                        if let Some(mut pi) = gv.pi.take() {
                            pi.output_pins(&gv.grid, &snapshot);
                            gv.pi = Some(pi);
                        }

                        sleep(sleep_time);

                        #[cfg(feature = "vision")]
                        {
                            let correction = snapshot.correct(&blobs.lock().unwrap());
                            if should_correct {
                                if let Some(new_snapshot) = correction {
                                    info!("old snapshot: {:#?}", snapshot);
                                    info!("new snapshot: {:#?}", new_snapshot);
                                    if should_add_edges {
                                        gv.add_error_edges(&snapshot, &new_snapshot);
                                    }
                                    gv.rollback(&new_snapshot);
                                    snapshot = new_snapshot;
                                };
                            }
                        }
                    }

                    let should_perturb = rng.gen_bool(err_rate);
                    if should_perturb {
                        let blobs = gv
                            .perturb(&mut rng, &snapshot)
                            .map(|perturbed_snapshot| perturbed_snapshot.to_blobs());

                        if let Some(blobs) = blobs {
                            info!("Simulating an error...");
                            if let Some(new_snapshot) = snapshot.correct(&blobs) {
                                info!("old snapshot: {:#?}", snapshot);
                                info!("new snapshot: {:#?}", new_snapshot);
                                if should_add_edges {
                                    gv.add_error_edges(&snapshot, &new_snapshot);
                                }
                                gv.rollback(&new_snapshot);
                                snapshot = new_snapshot;
                            };
                        }
                    }
                    gv.commit_pending(snapshot);
                }
                NotReady => {}
                Done => break,
            }
        }
        info!("Executor is terminating!");
        ::std::mem::drop(endpoint);
    }
}
