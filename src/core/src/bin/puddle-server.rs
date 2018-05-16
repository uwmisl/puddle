extern crate puddle_core;

extern crate jsonrpc_core;

extern crate clap;
#[macro_use]
extern crate rouille;

extern crate env_logger;
#[macro_use]
extern crate log;

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rouille::{Request, Response};

use jsonrpc_core::IoHandler;

use clap::{App, Arg, ArgMatches};

use puddle_core::*;

fn handle(ioh: &IoHandler, req: &Request) -> Response {
    // read the body into a string
    let mut req_string = String::new();
    let mut body = req.data().expect("body already retrieved!");
    body.read_to_string(&mut req_string).expect("read failed");

    info!("req: ({})", &req_string);

    // handle the request with jsonrpc, then convert to IronResult
    let resp_data = &ioh.handle_request_sync(&req_string)
        .expect("handle failed!");
    let resp = Response::from_data("application/json", resp_data.bytes().collect::<Vec<_>>());
    debug!("Resp: {:?}", resp_data);
    resp
}

fn run(matches: ArgMatches) -> Result<(), Box<::std::error::Error>> {
    // required argument is safe to unwrap
    let path = matches.value_of("arch").unwrap();
    let reader = File::open(path)?;

    let static_dir = PathBuf::from(matches.value_of("static").unwrap());

    let should_sync = matches.occurrences_of("sync") > 0 || env::var("PUDDLE_VIZ").is_ok();

    // let mut manager_opts = ErrorOptions::default();
    // if let Some(err) = matches.value_of("split-error") {
    //     manager_opts.split_error_stdev = err.parse()?;
    // };

    let grid = Grid::from_reader(reader)?;
    let manager = Manager::new(should_sync, grid);
    let arc = Arc::new(manager);

    let mut ioh = IoHandler::new();
    ioh.extend_with(arc.to_delegate());

    // args that have defaults are safe to unwrap
    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap();
    let address = format!("{}:{}", host, port);

    // this has to be a print, not a log, because the python lib looks for it
    println!("Listening on http://{}", address);

    rouille::start_server(address, move |request| {
        router!(
            request,
            (GET) (/status) => {
                // Builds a `Response` object that contains the "hello world" text.
                Response::text("Ok!")
            },
            (POST) (/rpc) => {
                handle(&ioh, request)
            },
            (GET) (/{path: String}) => {
                // FIXME this hack won't work on subdirectories
                if path == "" {
                    let mut pb = static_dir.clone();
                    pb.push("index.html");
                    Response::from_file(
                        "html",
                        File::open(pb).unwrap()
                    )
                } else {
                    rouille::match_assets(&request, &static_dir)
                }
            },

            _ => { rouille::Response::empty_404() }
        )
    });
}

fn check_dir(dir: String) -> Result<(), String> {
    if Path::new(&dir).is_dir() {
        Ok(())
    } else {
        Err("static should be a directory".to_string())
    }
}

fn main() {
    // enable logging
    let _ = env_logger::try_init();

    let matches = App::new("puddle")
        .version("0.1")
        .author("Max Willsey <me@mwillsey.com>")
        .about("Runs a server for Puddle")
        .arg(
            Arg::with_name("arch")
                .value_name("ARCH_FILE")
                .help("The architecture file")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("split-error")
                .long("split-error-stdev")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("static")
                .long("static")
                .required(true)
                .takes_value(true)
                .validator(check_dir),
        )
        .arg(
            Arg::with_name("host")
                .long("host")
                .default_value("localhost")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .default_value("3000")
                .takes_value(true),
        )
        .arg(Arg::with_name("sync").long("sync"))
        .get_matches();

    ::std::process::exit(match run(matches) {
        Ok(_) => 0,
        Err(err) => {
            error!("error: {}", err);
            1
        }
    });
}
