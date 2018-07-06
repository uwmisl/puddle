extern crate clap;
extern crate env_logger;
extern crate puddle_core;
#[macro_use]
extern crate log;

use clap::{App, SubCommand};
use std::error::Error;
use std::sync::Arc;

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
        ("cam", Some(_m)) => {
            let should_draw = true;
            let trackbars = true;

            let mut detector = vision::Detector::new(trackbars);
            let blobs = Arc::default();
            let blob_ref = Arc::clone(&blobs);
            detector.run(should_draw, blob_ref);
        }
        _ => {
            println!("Please pick a subcommmand.");
        }
    };

    debug!("Done!");
    Ok(())
}
