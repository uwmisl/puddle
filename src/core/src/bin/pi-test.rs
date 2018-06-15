extern crate clap;
extern crate env_logger;
extern crate puddle_core;
#[macro_use]
extern crate log;

use clap::{App, Arg, SubCommand};
use std::error::Error;
use std::fs::File;

use puddle_core::grid::{Droplet, DropletId, Grid, Location, Snapshot};
use puddle_core::pi::RaspberryPi;
use puddle_core::util::collections::Map;

fn main() -> Result<(), Box<Error>> {
    // enable logging
    let _ = env_logger::try_init();

    let matches = App::new("pi test")
        .version("0.1")
        .author("Max Willsey <me@mwillsey.com>")
        .about("Test out some of the hardware on the pi")
        .subcommand(
            SubCommand::with_name("dac")
                .arg(Arg::with_name("value").takes_value(true).required(true)),
        )
        .subcommand(
            SubCommand::with_name("pwm")
                .arg(Arg::with_name("channel").takes_value(true).required(true))
                .arg(Arg::with_name("duty").takes_value(true).required(true))
                .arg(Arg::with_name("freq").takes_value(true).required(true)),
        )
        .subcommand(
            SubCommand::with_name("pi-pwm")
                .arg(Arg::with_name("channel").takes_value(true).required(true))
                .arg(Arg::with_name("frequency").takes_value(true).required(true))
                .arg(Arg::with_name("duty").takes_value(true).required(true)),
        )
        .subcommand(
            SubCommand::with_name("set-loc")
                .arg(Arg::with_name("gridpath").takes_value(true).required(true))
                .arg(Arg::with_name("y").takes_value(true).required(true))
                .arg(Arg::with_name("x").takes_value(true).required(true)),
        )
        .get_matches();

    let mut pi = RaspberryPi::new()?;
    debug!("Pi started successfully!");

    let result = match matches.subcommand() {
        ("dac", Some(m)) => {
            let value = m.value_of("value").unwrap().parse().unwrap();
            pi.mcp4725.write(value)
        }
        ("pwm", Some(m)) => {
            let channel = m.value_of("channel").unwrap().parse().unwrap();
            let duty = m.value_of("duty").unwrap().parse().unwrap();
            let freq = m.value_of("freq").unwrap().parse().unwrap();
            pi.pca9685.set_pwm_freq(freq);
            pi.pca9685.set_duty_cycle(channel, duty);
            Ok(())
        }
        ("pi-pwm", Some(m)) => {
            let channel = m.value_of("channel").unwrap().parse().unwrap();
            let frequency = m.value_of("frequency").unwrap().parse().unwrap();
            let duty = m.value_of("duty").unwrap().parse().unwrap();
            pi.set_pwm(channel, frequency, duty)
        }
        ("set-loc", Some(m)) => {
            let gridpath = m.value_of("gridpath").unwrap();
            let y = m.value_of("y").unwrap().parse().unwrap();
            let x = m.value_of("x").unwrap().parse().unwrap();
            let mut droplets = Map::new();
            let id = DropletId {
                id: 0,
                process_id: 0,
            };
            let droplet = Droplet {
                id: id,
                location: Location { y, x },
                dimensions: Location { y: 1, x: 1 },
                volume: 1.0,
                destination: None,
                collision_group: 0,
            };
            info!("Using {:#?}", droplet);
            droplets.insert(id, droplet);
            let snapshot = Snapshot {
                droplets: droplets,
                commands_to_finalize: vec![],
            };

            let reader = File::open(gridpath)?;
            let grid = Grid::from_reader(reader)?;

            pi.init_hv507();
            pi.output_pins(&grid, &snapshot);
            Ok(())
        }
        _ => {
            println!("Please pick a subcommmand.");
            Ok(())
        }
    };

    result.map_err(|e| e.into())
}
