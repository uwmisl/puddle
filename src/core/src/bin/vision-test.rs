extern crate clap;
extern crate env_logger;
extern crate puddle_core;
#[macro_use]
extern crate log;

use clap::{App, Arg, SubCommand};
use std::error::Error;

use puddle_core::vision;

fn main() -> Result<(), Box<Error>> {
    // enable logging
    let _ = env_logger::try_init();

    let matches = App::new("vision test")
        .version("0.1")
        .author("Max Willsey <me@mwillsey.com>")
        .about("Test out some vision stuff")
        .subcommand(SubCommand::with_name("cam"))
        .get_matches();

    match matches.subcommand() {
        ("cam", Some(m)) => {
            let mut det = vision::Detector::new();
            loop {
                let should_quit = det.detect(true);
                if should_quit {
                    break
                }
            }
        }
        _ => {
            println!("Please pick a subcommmand.");
        }
    };

    debug!("Done!");
    Ok(())
}
