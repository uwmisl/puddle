extern crate puddle_core;

#[macro_use]
extern crate rouille;
extern crate jsonrpc_core;
extern crate clap;

use std::fs::File;
use std::path::{Path, PathBuf};
use std::env;

use rouille::{Request, Response};
use rouille::input::json_input;

use jsonrpc_core::IoHandler;
use jsonrpc_core::futures::future::Future;

use clap::{ArgMatches, App, Arg};

use puddle_core::api::{Session, Rpc};
use puddle_core::arch::Architecture;

fn handle(ioh: &IoHandler, req: &Request) -> Response {
    // read the body into a string
    let json_req = json_input(req).unwrap();
    eprintln!("req: ({:?})", json_req);

    // handle the request with jsonrpc, then convert to IronResult
    let resp = Response::json(&ioh.handle_rpc_request(json_req).wait().unwrap());
    eprintln!("Resp: {:?}", resp);
    resp
}

fn run(matches: ArgMatches) -> Result<(), Box<::std::error::Error>> {
    // required argument is safe to unwrap
    let path = matches.value_of("arch").unwrap();
    let reader = File::open(path)?;

    let static_dir = PathBuf::from(matches.value_of("static").unwrap());

    let should_sync = matches.occurrences_of("sync") > 0 || env::var("PUDDLE_VIZ").is_ok();

    let session = Session::new(Architecture::from_reader(reader)).sync(should_sync);

    let mut ioh = IoHandler::new();
    ioh.extend_with(session.to_delegate());

    // args that have defaults are safe to unwrap
    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap();
    let address = format!("{}:{}", host, port);
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
            eprintln!("error: {}", err);
            1
        }
    });
}
