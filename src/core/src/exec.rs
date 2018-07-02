use std::env;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use rand::prelude::thread_rng;
use rand::Rng;

use grid::{DropletInfo, ExecResponse, GridView};
use util::endpoint::Endpoint;

#[cfg(feature = "pi")]
use pi::RaspberryPi;

/// delay between steps in milliseconds
#[cfg(feature = "pi")]
static STEP_DELAY: u64 = 100;
#[cfg(not(feature = "pi"))]
static STEP_DELAY: u64 = 1;

pub struct Executor {
    blocking: bool,
    gridview: Arc<Mutex<GridView>>,
    #[cfg(feature = "pi")]
    pi: Option<RaspberryPi>,
}

impl Executor {
    pub fn new(blocking: bool, gridview: Arc<Mutex<GridView>>) -> Self {
        #[cfg(feature = "pi")]
        let pi = match env::var("PUDDLE_PI") {
            Ok(s) => if s == "1" {
                let mut pi = RaspberryPi::new().unwrap();
                pi.init_hv507();
                Some(pi)
            } else {
                warn!("Couldn't read PUDDLE_PI={}", s);
                None
            },
            Err(_) => None,
        };

        Executor {
            blocking,
            gridview,
            #[cfg(feature = "pi")]
            pi,
        }
    }

    pub fn run(&mut self, endpoint: Endpoint<Vec<DropletInfo>, ()>) {
        let sleep_ms = env::var("PUDDLE_STEP_DELAY_MS")
            .ok()
            .map(|s| u64::from_str_radix(&s, 10).expect("Couldn't parse!"))
            .unwrap_or(STEP_DELAY);
        let sleep_time = Duration::from_millis(sleep_ms);

        let mut rng = thread_rng();

        #[cfg(feature = "vision")]
        let blobs = {
            use vision::Detector;
            use std::thread;
            let blobs = Arc::default();
            let blob_ref = Arc::clone(&blobs);
            let trackbars = false;
            let should_draw = true;
            let det_thread = thread::Builder::new().name("detector".into()).spawn(move || {
                let mut detector = Detector::new(trackbars);
                detector.run(should_draw, blob_ref)
            });
            blobs
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
                        self.pi
                            .as_mut()
                            .map(|pi| pi.output_pins(&gv.grid, &snapshot));

                        #[cfg(feature = "vision")]
                        {
                            let correction = snapshot.correct(&blobs.lock().unwrap());
                            if let Some(new_snapshot) = correction {
                                info!("old snapshot: {:#?}", snapshot);
                                info!("new snapshot: {:#?}", new_snapshot);
                                gv.rollback();
                                snapshot = new_snapshot;
                            };
                        }
                    }

                    let should_perturb = rng.gen_bool(0.0);
                    if should_perturb {
                        let blobs = gv.perturb(&mut rng, &snapshot)
                            .map(|perturbed_snapshot| perturbed_snapshot.to_blobs());

                        if let Some(blobs) = blobs {
                            info!("Simulating an error...");
                            if let Some(new_snapshot) = snapshot.correct(&blobs) {
                                info!("old snapshot: {:#?}", snapshot);
                                info!("new snapshot: {:#?}", new_snapshot);
                                gv.rollback();
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
