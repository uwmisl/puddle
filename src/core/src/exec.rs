use std::env;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use rand::prelude::thread_rng;
use rand::Rng;

use grid::{DropletInfo, ExecResponse, GridView, Location};
use util::endpoint::Endpoint;

use pi::{RaspberryPi, GpioPin, GpioMode};

/// delay between steps in milliseconds
static STEP_DELAY: u64 = 100;

pub struct Executor {
    blocking: bool,
    gridview: Arc<Mutex<GridView>>,
    pi: Option<RaspberryPi>,
}

impl Executor {
    pub fn new(blocking: bool, gridview: Arc<Mutex<GridView>>) -> Self {
        let mut exec = Executor {
            blocking,
            gridview,
            pi: None,
        };
        match env::var("PUDDLE_PI") {
            Ok(s) => if s == "1" {
                exec.use_pins()
            } else {
                warn!("Couldn't read PUDDLE_PI={}", s)
            }
            Err(_) => {},
        }
        exec
    }

    pub fn use_pins(&mut self) {
        // setup the HV507 for serial data write
        // see row "LOAD S/R" in table 3-2 in
        // http://ww1.microchip.com/downloads/en/DeviceDoc/20005845A.pdf
        let mut pi = RaspberryPi::new().unwrap();

        // let pi_chip = 0;
        // let pi_number = 0;
        // let pwm = Pwm::new(pi_chip, pi_number).unwrap();
        // pwm.with_exported(|| {
        //     pwm.enable(true).unwrap();
        //     pwm.set_period_ns(20_000).unwrap();
        //     pwm.set_period_ns(20_000).unwrap();
        //     Ok(())
        // }).unwrap();

        use self::GpioPin::*;
        use self::GpioMode::*;

        pi.gpio_set_mode(Blank, Output).unwrap();
        pi.gpio_write(Blank, 1).unwrap();

        pi.gpio_set_mode(LatchEnable, Output).unwrap();
        pi.gpio_write(LatchEnable, 0).unwrap();

        pi.gpio_set_mode(Clock, Output).unwrap();
        pi.gpio_write(Clock, 0).unwrap();

        pi.gpio_set_mode(Data, Output).unwrap();
        pi.gpio_write(Data, 0).unwrap();

        self.pi = Some(pi);
    }

    fn output_pins(pi: &mut RaspberryPi, gv: &GridView, pins: &mut [u8]) {

        // reset pins to low by default
        for p in pins.iter_mut() {
            *p = 0;
        }

        // set pins to high if there's a droplet on that electrode
        for d in gv.exec_snapshot().droplets.values() {
            for i in 0..d.dimensions.y {
                for j in 0..d.dimensions.x {
                    let loc = Location {
                        y: d.location.y + i,
                        x: d.location.x + j,
                    };
                    let electrode = gv.grid.get_cell(&loc).unwrap();
                    pins[electrode.pin as usize] = 1;
                }
            }
        }

        use self::GpioPin::*;
        // actually write the pins and cycle the clock
        for pin in pins.iter() {
            pi.gpio_write(Data, *pin).unwrap();
            pi.gpio_write(Clock, 1).unwrap();
            pi.gpio_write(Clock, 0).unwrap();
        }

        // commit the latch
        pi.gpio_write(LatchEnable, 1).unwrap();
        pi.gpio_write(LatchEnable, 0).unwrap();
    }

    pub fn run(&mut self, endpoint: Endpoint<Vec<DropletInfo>, ()>) {
        let sleep_ms = env::var("PUDDLE_STEP_DELAY_MS")
            .ok()
            .map(|s| u64::from_str_radix(&s, 10).expect("Couldn't parse!"))
            .unwrap_or(STEP_DELAY);
        let sleep_time = Duration::from_millis(sleep_ms);

        let mut rng = thread_rng();
        let max_pin = self.gridview.lock().unwrap().grid.max_pin();
        let mut pins = vec![0; (max_pin + 1) as usize];

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
                        endpoint.send(gv.exec_droplet_info(None)).unwrap()
                    }

                    self.pi.as_mut().map(|pi| Executor::output_pins(pi, &gv, &mut pins));

                    let should_perturb = rng.gen_bool(0.0);
                    if should_perturb {
                        let blobs = gv.perturb(&mut rng, &snapshot)
                            .map(|perturbed_snapshot| perturbed_snapshot.to_blobs());

                        if let Some(blobs) = blobs {
                            info!("Simulating an error...");
                            snapshot.correct(&blobs).map(|new_snapshot| {
                                info!("old snapshot: {:#?}", snapshot);
                                info!("new snapshot: {:#?}", new_snapshot);
                                gv.rollback();
                                snapshot = new_snapshot;
                            });
                        }
                    }
                    gv.commit_pending(snapshot);
                }
                NotReady => {}
                Done => break,
            }
        }
        info!("Executor is terminating!")
    }
}
