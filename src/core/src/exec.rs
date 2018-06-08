use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use rand::prelude::thread_rng;
use rand::Rng;

use grid::{DropletInfo, ExecResponse, GridView, Location};
use util::endpoint::Endpoint;

use rppal::gpio::{Gpio, Level, Mode};

// /// HV507 polarity
// /// Pin 32 - BCM 12 (PWM0)
// static POLARITY_PIN: u8 = 12;

// /// High voltage converter "analog" signal
// /// Pin 33 - BCM 13 (PWM1)
// static VOLTAGE_PIN: u8 = 13;

/// HV507 blank
/// Physical pin 11 - BCM 17
static BLANK_PIN: u8 = 17;

/// HV507 latch enable
/// Physical pin 27 - BCM 13
static LATCH_ENABLE_PIN: u8 = 13;

/// HV507 clock
/// Physical pin 22 - BCM 15
static CLOCK_PIN: u8 = 15;

/// HV507 data
/// Physical pin 23 - BCM 16
static DATA_PIN: u8 = 16;

/// delay between steps in milliseconds
static STEP_DELAY: u64 = 100;

pub struct Executor {
    blocking: bool,
    gridview: Arc<Mutex<GridView>>,
    gpio: Option<Gpio>,
}

impl Executor {
    pub fn new(blocking: bool, gridview: Arc<Mutex<GridView>>) -> Self {
        Executor {
            blocking,
            gridview,
            gpio: None,
        }
    }

    pub fn use_pins(&mut self) {
        // setup the HV507 for serial data write
        // see row "LOAD S/R" in table 3-2 in
        // http://ww1.microchip.com/downloads/en/DeviceDoc/20005845A.pdf
        self.gpio = None;

        let mut gpio = Gpio::new().expect("gpio init failed!");

        gpio.set_mode(BLANK_PIN, Mode::Output);
        gpio.write(BLANK_PIN, Level::High);

        gpio.set_mode(LATCH_ENABLE_PIN, Mode::Output);
        gpio.write(LATCH_ENABLE_PIN, Level::Low);

        gpio.set_mode(CLOCK_PIN, Mode::Output);
        gpio.write(CLOCK_PIN, Level::Low);

        gpio.set_mode(DATA_PIN, Mode::Output);
        gpio.write(DATA_PIN, Level::Low);

        self.gpio = Some(gpio)
    }

    fn output_pins(&self, gv: &GridView, pins: &mut [Level]) {
        // do nothing if we aren't set up to do gpio
        let gpio = match self.gpio {
            Some(ref g) => g,
            None => return,
        };

        // reset pins to low by default
        for p in pins.iter_mut() {
            *p = Level::Low;
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
                    pins[electrode.pin as usize] = Level::High;
                }
            }
        }

        // actually write the pins and cycle the clock
        for pin in pins.iter() {
            gpio.write(DATA_PIN, *pin);
            gpio.write(CLOCK_PIN, Level::High);
            gpio.write(CLOCK_PIN, Level::Low);
        }

        // commit the latch
        gpio.write(LATCH_ENABLE_PIN, Level::High);
        gpio.write(LATCH_ENABLE_PIN, Level::Low);
    }

    pub fn run(&self, endpoint: Endpoint<Vec<DropletInfo>, ()>) {
        let sleep_time = Duration::from_millis(STEP_DELAY);

        let mut rng = thread_rng();
        let max_pin = self.gridview.lock().unwrap().grid.max_pin();
        let mut pins = vec![Level::Low; (max_pin + 1) as usize];

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
                Step => {
                    if self.blocking {
                        endpoint.send(gv.exec_droplet_info(None)).unwrap()
                    }

                    self.output_pins(&gv, &mut pins);

                    let should_perturb = rng.gen_bool(0.0);
                    if should_perturb {
                        if let Some(new_snapshot) = gv.perturb(&mut rng) {
                            let _blobs = new_snapshot.to_blobs();
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
