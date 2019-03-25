use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::thread;
use std::time::Duration;

extern crate log;
use log::{debug, info};

extern crate puddle_core;
use puddle_core::{
    grid::droplet::{Blob, Droplet, DropletId, SimpleBlob},
    grid::parse::{ParsedGrid, PolarityConfig},
    grid::{Grid, Location, Snapshot},
    pi::RaspberryPi,
    util::{collections::Map, seconds_duration},
};

// TODO don't need to do this
extern crate structopt;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum SubCommand {
    // Dac,
    // Pwm,
    #[structopt(name = "set-polarity")]
    SetPolarity {
        frequency: f64,
        duty_cycle: f64,
        seconds: f64,
    },
    #[structopt(name = "set-gpio")]
    SetGpio { pin: usize, seconds: f64 },
    #[structopt(name = "set-loc")]
    SetLoc {
        location: Location,
        #[structopt(default_value = "(1,1)")]
        dimensions: Location,
        #[structopt(default_value = "1")]
        seconds: f64,
    },
    #[structopt(name = "circle")]
    Circle {
        location: Location,
        dimensions: Location,
        circle_size: Location,
        #[structopt(default_value = "1")]
        seconds: f64,
    },
    #[structopt(name = "back-and-forth")]
    BackAndForth {
        #[structopt(short = "d", long = "dimensions", default_value = "2,2")]
        dimensions: Location,
        #[structopt(short = "x", long = "x-distance", default_value = "3")]
        x_distance: u32,
        #[structopt(long = "spacing", default_value = "1")]
        spacing: u32,
        #[structopt(long = "starting-location", default_value = "1,0")]
        starting_location: Location,
        #[structopt(short = "n", long = "n-droplets", default_value = "1")]
        n_droplets: u32,
        #[structopt(short = "s", long = "seconds", default_value = "1")]
        seconds: f64,
    },
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
            let duration = seconds_duration(seconds);
            thread::sleep(duration);
        }
        SetGpio { pin, seconds } => {
            pi.hv507.set_pin_hi(pin);
            pi.hv507.shift_and_latch();
            let duration = seconds_duration(seconds);
            thread::sleep(duration);
        }
        SetLoc {
            location,
            dimensions,
            seconds,
        } => {
            let (_, snapshot) = mk_snapshot(&[SimpleBlob {
                location,
                dimensions,
                volume: 0.0,
            }]);
            pi.output_pins(&grid, &snapshot);
            let duration = seconds_duration(seconds);
            thread::sleep(duration);
        }
        BackAndForth {
            dimensions,
            starting_location,
            spacing,
            n_droplets,
            x_distance,
            seconds,
        } => {
            let blobs: Vec<_> = (0..n_droplets)
                .map(|i| {
                    let y_offset = (dimensions.y + spacing as i32) * i as i32;
                    let location = &starting_location + &Location { y: y_offset, x: 0 };
                    let volume = 0.0;
                    SimpleBlob {
                        volume,
                        dimensions,
                        location,
                    }
                })
                .collect();

            let duration = seconds_duration(seconds);
            let (ids, mut snapshot) = mk_snapshot(&blobs);

            let xs = 0..=x_distance;
            let xs = (xs.clone()).chain(xs.rev());

            for x in xs {
                for id in &ids {
                    snapshot.droplets.get_mut(&id).unwrap().location.x = x as i32;
                }
                let locs: Vec<_> = snapshot.droplets.values().map(|d| d.location).collect();

                pi.output_pins(&grid, &snapshot);
                println!("Droplets at {:?}", locs);
                thread::sleep(duration);
            }
        }
        Circle {
            location,
            dimensions,
            circle_size,
            seconds,
        } => {
            let (ids, mut snapshot) = mk_snapshot(&[SimpleBlob {
                location,
                dimensions,
                volume: 0.0,
            }]);
            let id = ids[0];

            let duration = seconds_duration(seconds);

            //     pi.output_pins(&grid, &snapshot);

            let mut set = |yo, xo| {
                let loc = Location {
                    y: location.y + yo,
                    x: location.x + xo,
                };
                snapshot.droplets.get_mut(&id).unwrap().location = loc;
                pi.output_pins(&grid, &snapshot);
                println!("Droplet at {}", loc);
                thread::sleep(duration);
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
    let reader = File::open(path).expect(&format!("failed to read: {}", path_str));
    debug!("Read config file successfully");
    let grid = ParsedGrid::from_reader(reader)?;
    Ok(grid)
}

fn mk_snapshot(blobs: &[SimpleBlob]) -> (Vec<DropletId>, Snapshot) {
    let n = blobs.len();
    let ids: Vec<_> = (0..n)
        .map(|i| DropletId {
            id: i,
            process_id: 0,
        })
        .collect();

    let droplets: Map<DropletId, Droplet> = ids
        .iter()
        .zip(blobs)
        .map(|(&id, blob)| (id, blob.to_droplet(id)))
        .collect();

    let snapshot = Snapshot {
        droplets,
        commands_to_finalize: vec![],
    };
    (ids, snapshot)
}

// fn set_loc(m: &ArgMatches, pi: &mut RaspberryPi) -> Result<(), Box<Error>> {
//     let grid = mk_grid(m)?;
//     let location = m.value_of("location").unwrap().parse()?;
//     let dimensions = m.value_of("dimensions").unwrap().parse()?;
//     let (_, snapshot) = mk_snapshot(location, dimensions);
//     pi.output_pins(&grid, &snapshot);
//     Ok(())
// }

// fn circle(m: &ArgMatches, pi: &mut RaspberryPi) -> Result<(), Box<Error>> {
//     let grid = mk_grid(m)?;

//     let location = m.value_of("location").unwrap().parse()?;
//     let dimensions = m.value_of("dimensions").unwrap().parse()?;
//     let (id, mut snapshot) = mk_snapshot(location, dimensions);

//     let size: Location = m.value_of("circle").unwrap().parse()?;
//     let duration = Duration::from_millis(m.value_of("sleep").unwrap().parse()?);

//     pi.output_pins(&grid, &snapshot);

//     let mut set = |yo, xo| {
//         let loc = Location {
//             y: location.y + yo,
//             x: location.x + xo,
//         };
//         snapshot.droplets.get_mut(&id).unwrap().location = loc;
//         pi.output_pins(&grid, &snapshot);
//         println!("Droplet at {}", loc);
//         thread::sleep(duration);
//     };

//     loop {
//         for xo in 0..size.x {
//             set(xo, 0);
//         }
//         for yo in 0..size.y {
//             set(size.x - 1, yo);
//         }
//         for xo in 0..size.x {
//             set(size.x - 1 - xo, size.y - 1);
//         }
//         for yo in 0..size.y {
//             set(0, size.y - 1 - yo);
//         }
//     }
// }

// fn heat(m: &ArgMatches, pi: &mut RaspberryPi) -> Result<(), Box<Error>> {
//     let grid = mk_grid(&m)?;
//     let heater_loc = m.value_of("heater").unwrap().parse()?;
//     let temp = m.value_of("temp").unwrap().parse()?;
//     let seconds = m.value_of("seconds").unwrap().parse()?;

//     let heater = grid
//         .get_cell(&heater_loc)
//         .cloned()
//         .unwrap()
//         .peripheral
//         .expect("Given location wasn't a heater!");
//     let duration = seconds_duration(seconds);

//     pi.heat(&heater, temp, duration)?;

//     Ok(())
// }

// fn get_pin(pin: u32, grid: &Grid) -> Option<Location> {
//     for (loc, electrode) in grid.locations() {
//         if electrode.pin == pin {
//             return Some(loc);
//         }
//     }
//     None
// }

// fn test_pins(m: &ArgMatches, pi: &mut RaspberryPi) -> Result<(), Box<Error>> {
//     let grid = mk_grid(&m)?;
//     let millis = m.value_of("millis").unwrap().parse()?;
//     let duration = if millis == 0 {
//         println!("Press enter to step to next pin");
//         None
//     } else {
//         Some(Duration::from_millis(millis))
//     };
//     let pin_limit = grid.max_pin() + 1;

//     let mut stdin = io::stdin();
//     let mut stdout = io::stdout();

//     for i in 0..pin_limit {
//         if let Some(loc) = get_pin(i, &grid) {
//             println!("pin {} at {}", i, loc);
//             let (_, snapshot) = mk_snapshot(loc, Location { y: 1, x: 1 });
//             pi.output_pins(&grid, &snapshot);
//         } else {
//             println!("pin {} has no location", i);
//         }

//         // either wait or pause
//         if let Some(duration) = duration {
//             thread::sleep(duration);
//         } else {
//             print!("Outputting pin {}", i);
//             stdout.flush().unwrap();
//             let _ = stdin.read(&mut [0u8]).unwrap();
//         }
//     }

//     Ok(())
// }
