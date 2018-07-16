extern crate clap;
extern crate env_logger;
extern crate puddle_core;
#[macro_use]
extern crate log;

use clap::{App, Arg, ArgMatches, SubCommand};
use std::error::Error;
use std::fs::File;
use std::thread;
use std::time::Duration;

use puddle_core::grid::{Droplet, DropletId, Grid, Location, Snapshot};
use puddle_core::pi::RaspberryPi;
use puddle_core::util::{collections::Map, seconds_duration};

fn main() -> Result<(), Box<Error>> {
    // enable logging
    let _ = env_logger::try_init();

    let matches = App::new("pi test")
        .version("0.1")
        .author("Max Willsey <me@mwillsey.com>")
        .about("Test out some of the hardware on the pi")
        .subcommand(SubCommand::with_name("off"))
        .subcommand(
            SubCommand::with_name("dac")
                .arg(Arg::with_name("value").takes_value(true).required(true)),
        )
        .subcommand(
            SubCommand::with_name("pwm")
                .arg(Arg::with_name("channel").takes_value(true).required(true))
                .arg(Arg::with_name("duty").takes_value(true).required(true))
                .arg(Arg::with_name("freq").takes_value(true).required(true))
                .arg(
                    Arg::with_name("duration")
                        .takes_value(true)
                        .default_value("1.0"),
                ),
        )
        .subcommand(
            SubCommand::with_name("pi-pwm")
                .arg(Arg::with_name("channel").takes_value(true).required(true))
                .arg(Arg::with_name("frequency").takes_value(true).required(true))
                .arg(Arg::with_name("duty").takes_value(true).required(true)),
        )
        .subcommand(
            SubCommand::with_name("set-loc")
                .arg(Arg::with_name("grid").takes_value(true).required(true))
                .arg(Arg::with_name("location").takes_value(true).required(true))
                .arg(
                    Arg::with_name("dimensions")
                        .takes_value(true)
                        .default_value("(1,1)"),
                ),
        )
        .subcommand(
            SubCommand::with_name("circle")
                .arg(Arg::with_name("grid").takes_value(true).required(true))
                .arg(Arg::with_name("location").takes_value(true).required(true))
                .arg(
                    Arg::with_name("dimensions")
                        .takes_value(true)
                        .default_value("(1,1)"),
                )
                .arg(
                    Arg::with_name("circle")
                        .takes_value(true)
                        .default_value("(2,2)"),
                )
                .arg(
                    Arg::with_name("sleep")
                        .takes_value(true)
                        .default_value("1000"),
                ),
        )
        .subcommand(SubCommand::with_name("temp"))
        .subcommand(
            SubCommand::with_name("heat")
                .arg(Arg::with_name("grid").takes_value(true).required(true))
                .arg(Arg::with_name("heater").takes_value(true).required(true))
                .arg(Arg::with_name("temp").takes_value(true).required(true))
                .arg(Arg::with_name("seconds").takes_value(true).required(true)),
        )
        .get_matches();

    let mut pi = RaspberryPi::new()?;
    debug!("Pi started successfully!");

    match matches.subcommand() {
        ("off", Some(_m)) => {
            pi.pca9685.all_off()?;
            pi.mcp4725.write(0)?;
            Ok(())
        }
        ("dac", Some(m)) => {
            let value = m.value_of("value").unwrap().parse()?;
            pi.mcp4725.write(value)?;
            Ok(())
        }
        ("pwm", Some(m)) => {
            let channel = m.value_of("channel").unwrap().parse()?;
            let duty = m.value_of("duty").unwrap().parse()?;
            let freq = m.value_of("freq").unwrap().parse()?;
            let seconds = m.value_of("duration").unwrap().parse()?;
            pi.pca9685.set_pwm_freq(freq)?;
            pi.pca9685.set_duty_cycle(channel, duty)?;
            thread::sleep(seconds_duration(seconds));
            Ok(())
        }
        ("pi-pwm", Some(m)) => {
            let channel = m.value_of("channel").unwrap().parse()?;
            let frequency = m.value_of("frequency").unwrap().parse()?;
            let duty = m.value_of("duty").unwrap().parse()?;
            pi.set_pwm(channel, frequency, duty)?;
            Ok(())
        }
        ("set-loc", Some(m)) => set_loc(&m, &mut pi),
        ("circle", Some(m)) => circle(&m, &mut pi),
        ("temp", Some(_)) => {
            let resistance = pi.max31865.read_one_resistance()?;
            let temp = pi.max31865.read_temperature()?;
            println!("Temp: {}C, Resistance: {} ohms", temp, resistance);
            Ok(())
        }
        ("heat", Some(m)) => heat(&m, &mut pi),
        _ => {
            println!("Please pick a subcommmand.");
            Ok(())
        }
    }

    // result.map_err(|e| e.into())
}

fn mk_grid(m: &ArgMatches) -> Result<Grid, Box<Error>> {
    let gridpath = m.value_of("grid").unwrap();
    let reader = File::open(gridpath)?;
    let grid = Grid::from_reader(reader)?;
    Ok(grid)
}

fn mk_snapshot(location: Location, dimensions: Location) -> (DropletId, Snapshot) {
    let mut droplets = Map::new();
    // just use a dummy id
    let id = DropletId {
        id: 0,
        process_id: 0,
    };
    let droplet = Droplet {
        id: id,
        location,
        dimensions,
        ..Droplet::default()
    };
    info!("Using {:#?}", droplet);
    droplets.insert(id, droplet);
    let snapshot = Snapshot {
        droplets: droplets,
        commands_to_finalize: vec![],
    };
    (id, snapshot)
}

fn set_loc(m: &ArgMatches, pi: &mut RaspberryPi) -> Result<(), Box<Error>> {
    let grid = mk_grid(m)?;
    let location = m.value_of("location").unwrap().parse()?;
    let dimensions = m.value_of("dimensions").unwrap().parse()?;
    let (_, snapshot) = mk_snapshot(location, dimensions);
    pi.output_pins(&grid, &snapshot);
    Ok(())
}

fn circle(m: &ArgMatches, pi: &mut RaspberryPi) -> Result<(), Box<Error>> {
    let grid = mk_grid(m)?;

    let location = m.value_of("location").unwrap().parse()?;
    let dimensions = m.value_of("dimensions").unwrap().parse()?;
    let (id, mut snapshot) = mk_snapshot(location, dimensions);

    let size: Location = m.value_of("circle").unwrap().parse()?;
    let duration = Duration::from_millis(m.value_of("sleep").unwrap().parse()?);

    pi.output_pins(&grid, &snapshot);

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

    loop {
        for xo in 0..size.x {
            set(xo, 0);
        }
        for yo in 0..size.y {
            set(size.x - 1, yo);
        }
        for xo in 0..size.x {
            set(size.x - 1 - xo, size.y - 1);
        }
        for yo in 0..size.y {
            set(0, size.y - 1 - yo);
        }
    }
}

fn heat(m: &ArgMatches, pi: &mut RaspberryPi) -> Result<(), Box<Error>> {
    let grid = mk_grid(&m)?;
    let heater_loc = m.value_of("heater").unwrap().parse()?;
    let temp = m.value_of("temp").unwrap().parse()?;
    let seconds = m.value_of("seconds").unwrap().parse()?;

    let heater = grid.get_cell(&heater_loc)
        .unwrap()
        .peripheral
        .expect("Given location wasn't a heater!");
    let duration = seconds_duration(seconds);

    pi.heat(&heater, temp, duration)?;

    Ok(())
}
