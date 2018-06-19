extern crate clap;
extern crate env_logger;
extern crate puddle_core;
#[macro_use]
extern crate log;

use clap::{App, Arg, SubCommand};
use std::error::Error;

use puddle_core::vision::detect_droplets;

fn main() -> Result<(), Box<Error>> {
    // enable logging
    let _ = env_logger::try_init();

    let matches = App::new("pi test")
        .version("0.1")
        .author("Max Willsey <me@mwillsey.com>")
        .about("Test out some of the hardware on the pi")
        .subcommand(
            SubCommand::with_name("diff")
                .arg(Arg::with_name("frame").takes_value(true).required(true))
                .arg(
                    Arg::with_name("background")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("diff", Some(m)) => {
            let frame = m.value_of("frame").unwrap();
            let background = m.value_of("background").unwrap();
            detect_droplets(frame, background);
        }
        _ => {
            println!("Please pick a subcommmand.");
        }
    };

    debug!("Done!");
    Ok(())
}
