use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::thread;

use log::*;

use puddle_core::{
    grid::droplet::{DropletId, SimpleBlob},
    grid::gridview::GridView,
    grid::parse::{ParsedGrid, PolarityConfig},
    grid::{Grid, Location},
    util::seconds_duration,
};
use puddle_pi::RaspberryPi;

#[derive(Debug)]
struct MyDuration(std::time::Duration);

impl std::str::FromStr for MyDuration {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let float: f64 = s.parse()?;
        if float < 0.0 {
            panic!("Float should be non-negative");
        }
        Ok(MyDuration(seconds_duration(float)))
    }
}

// TODO don't need to do this
extern crate structopt;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum SubCommand {
    #[structopt(name = "set-polarity")]
    SetPolarity {
        frequency: f64,
        duty_cycle: f64,
        seconds: MyDuration,
    },
    #[structopt(name = "set-gpio")]
    SetGpio { pin: usize, seconds: MyDuration },
    #[structopt(name = "set-loc")]
    SetLoc {
        location: Location,
        #[structopt(default_value = "(1,1)")]
        dimensions: Location,
        #[structopt(default_value = "1")]
        seconds: MyDuration,
    },
    #[structopt(name = "circle")]
    Circle {
        location: Location,
        dimensions: Location,
        circle_size: Location,
        #[structopt(default_value = "1")]
        seconds: MyDuration,
    },
    #[structopt(name = "back-and-forth")]
    BackAndForth {
        #[structopt(short = "d", long = "dimensions", default_value = "2,2")]
        dimensions: Location,
        #[structopt(short = "x", long = "x-distance", default_value = "3")]
        x_distance: i32,
        #[structopt(long = "spacing", default_value = "1")]
        spacing: u32,
        #[structopt(long = "starting-location", default_value = "1,0")]
        starting_location: Location,
        #[structopt(short = "n", long = "n-droplets", default_value = "1")]
        n_droplets: u32,
        #[structopt(short = "s", long = "seconds", default_value = "1")]
        seconds: MyDuration,
        #[structopt(
            long = "stagger",
            help = "additional seconds to stagger the movement of droplets"
        )]
        stagger: Option<MyDuration>,
    },
    // Dac,
    // Pwm,
    // Temp,
    // Heat,
    // Pins,
}

fn main() -> Result<(), Box<Error>> {
    // enable logging
    let _ = env_logger::try_init();

    let sub = SubCommand::from_args();

    let config: ParsedGrid = match std::env::var("PI_CONFIG") {
        Ok(path) => {
            println!("Using PI_CONFIG={}", path);
            mk_grid(&path)?
        }
        Err(e) => {
            eprintln!("Please set environment variable PI_CONFIG");
            return Err(e.into());
        }
    };
    let grid = config.to_grid();

    let mut pi = RaspberryPi::new(config.pi_config)?;

    use SubCommand::*;
    match sub {
        SetPolarity {
            frequency,
            duty_cycle,
            seconds,
        } => {
            let polarity_config = PolarityConfig {
                frequency,
                duty_cycle,
            };
            pi.hv507.set_polarity(&polarity_config)?;
            thread::sleep(seconds.0);
        }
        SetGpio { pin, seconds } => {
            pi.hv507.set_pin_hi(pin);
            pi.hv507.shift_and_latch();
            thread::sleep(seconds.0);
        }
        SetLoc {
            location,
            dimensions,
            seconds,
        } => {
            let gv = mk_gridview(
                grid.clone(),
                &[SimpleBlob {
                    location,
                    dimensions,
                    volume: 0.0,
                }],
            );
            pi.output_pins(&gv);
            thread::sleep(seconds.0);
        }
        BackAndForth {
            dimensions,
            starting_location,
            spacing,
            n_droplets,
            x_distance,
            seconds,
            stagger,
        } => {
            let blobs: Vec<_> = (0..n_droplets)
                .map(|i| {
                    let y_offset = (dimensions.y + spacing as i32) * i as i32;
                    let location = starting_location + Location { y: y_offset, x: 0 };
                    let volume = 0.0;
                    SimpleBlob {
                        volume,
                        dimensions,
                        location,
                    }
                })
                .collect();

            let mut gv = mk_gridview(grid.clone(), &blobs);
            let ids: Vec<_> = (0..n_droplets).map(|i| mk_id(i as usize)).collect();

            let xs: Vec<i32> = {
                let start = starting_location.x;
                let end = start + x_distance;
                assert!(end >= 0);

                if start < end {
                    let xs = start..end;
                    (xs.clone()).chain(xs.rev()).collect()
                } else {
                    let xs = end..start;
                    (xs.clone().rev()).chain(xs).collect()
                }
            };

            println!("Moving to x's: {:?}", xs);

            for x in xs {
                for id in &ids {
                    let droplet = gv.droplets.get_mut(id).unwrap();
                    droplet.location.x = x as i32;
                    if let Some(stagger) = &stagger {
                        pi.output_pins(&gv);
                        thread::sleep(stagger.0);
                    }
                }
                let locs: Vec<_> = gv.droplets.values().map(|d| d.location).collect();
                pi.output_pins(&gv);
                println!("Droplets at {:?}", locs);

                thread::sleep(seconds.0);
            }
        }
        Circle {
            location,
            dimensions,
            circle_size,
            seconds,
        } => {
            let mut gv = mk_gridview(
                grid.clone(),
                &[SimpleBlob {
                    location,
                    dimensions,
                    volume: 0.0,
                }],
            );
            let id = mk_id(0);

            //     pi.output_pins(&grid, &snapshot);

            let mut set = |yo, xo| {
                let loc = Location {
                    y: location.y + yo,
                    x: location.x + xo,
                };
                gv.droplets.get_mut(&id).unwrap().location = loc;
                pi.output_pins(&gv);
                println!("Droplet at {}", loc);
                thread::sleep(seconds.0);
            };

            for xo in 0..circle_size.x {
                set(0, xo);
            }
            for yo in 0..circle_size.y {
                set(yo, circle_size.x - 1);
            }
            for xo in 0..circle_size.x {
                set(circle_size.y - 1, circle_size.x - 1 - xo);
            }
            for yo in 0..circle_size.y {
                set(circle_size.y - 1 - yo, 0);
            }
        }
    }

    Ok(())
}

fn mk_grid(path_str: &str) -> Result<ParsedGrid, Box<Error>> {
    let path = Path::new(path_str);
    let reader = File::open(path)?;
    debug!("Read config file successfully");
    let grid = ParsedGrid::from_reader(reader)?;
    Ok(grid)
}

fn mk_id(i: usize) -> DropletId {
    DropletId {
        id: i,
        process_id: 42,
    }
}

fn mk_gridview(_grid: Grid, _blobs: &[SimpleBlob]) -> GridView {
    unimplemented!()
    // let n = blobs.len();
    // let ids: Vec<_> = (0..n)
    //     .map(|i| DropletId {
    //         id: i,
    //         process_id: 0,
    //     })
    //     .collect();

    // let droplets: Map<DropletId, Droplet> = ids
    //     .iter()
    //     .zip(blobs)
    //     .map(|(&id, blob)| (id, blob.to_droplet(id)))
    //     .collect();

    // let snapshot = Snapshot {
    //     droplets,
    //     commands_to_finalize: vec![],
    // };
    // (ids, snapshot)
}
