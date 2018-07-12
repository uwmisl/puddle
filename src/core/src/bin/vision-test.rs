extern crate clap;
extern crate env_logger;
extern crate puddle_core;
#[macro_use]
extern crate log;

use clap::{App, Arg, SubCommand};
use std::error::Error;
use std::sync::Arc;
use std::path::Path;

use puddle_core::vision;

fn main() -> Result<(), Box<Error>> {
    // enable logging
    let _ = env_logger::try_init();

    let matches = App::new("vision test")
        .version("0.1")
        .author("Max Willsey <me@mwillsey.com>")
        .about("Test out some vision stuff")
        .subcommand(SubCommand::with_name("cam"))
        .subcommand(SubCommand::with_name("file")
                    .arg(Arg::with_name("input").takes_value(true).required(true))
                    .arg(Arg::with_name("output").takes_value(true).required(true))
        )
        .get_matches();

    match matches.subcommand() {
        ("cam", Some(_m)) => {
            let trackbars = true;
            let should_draw = true;

            let mut detector = vision::Detector::new(trackbars);
            let blobs = Arc::default();
            let blob_ref = Arc::clone(&blobs);
            detector.run(should_draw, blob_ref);
        }
        ("file", Some(m)) => {
            let input = Path::new(m.value_of("input").unwrap());
            let output = Path::new(m.value_of("output").unwrap());

            let mut detector = vision::Detector::from_filename(input, output);
            let should_draw = false;
            detector.detect(should_draw);
        }
        _ => {
            println!("Please pick a subcommmand.");
        }
    };

    debug!("Done!");
    Ok(())
}
