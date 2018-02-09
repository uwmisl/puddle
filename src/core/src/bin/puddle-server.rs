extern crate puddle_core;

extern crate iron;
extern crate mount;
extern crate staticfile;

extern crate jsonrpc_core;

extern crate clap;

use std::fmt;
use std::fs::File;
use std::path::Path;
use std::io::Read;
use std::env;

use iron::prelude::*;
use iron::Handler;
use iron::headers::ContentType;
use iron::status;

use mount::Mount;
use staticfile::Static;

use jsonrpc_core::IoHandler;

use clap::{ArgMatches, App, Arg};

use puddle_core::api::{Session, Rpc};
use puddle_core::arch::Architecture;

#[derive(Debug)]
struct JsonRpcError;

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "jsonrpc did not respond")
    }
}

impl iron::error::Error for JsonRpcError {
    fn description(&self) -> &str {
        "jsonrpc did not respond"
    }

    fn cause(&self) -> Option<&iron::error::Error> {
        None
    }
}

// needed so we can implement the Handler trait on IoHandler
struct IoHandlerWrapper(IoHandler);

impl Handler for IoHandlerWrapper {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        // read the body into a string
        let mut body = String::new();
        req .body
            .read_to_string(&mut body)
            .map_err(|e| IronError::new(e, (status::InternalServerError, "Error reading request")))?;

        println!("req: ({})", body);

        // handle the request with jsonrpc, then convert to IronResult
        self.0.handle_request_sync(&body)
            .map(|resp| {
                println!("resp: ({})", resp);
                Response::with((ContentType::json().0, status::Ok, resp))
            })
            .ok_or(IronError::new(JsonRpcError, (status::InternalServerError,
                                                 "jsonrpc error")))
    }
}

fn status(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "All is good!")))
}

fn run(matches: ArgMatches) -> Result<(), Box<::std::error::Error>> {
    // required argument is safe to unwrap
    let path = matches.value_of("arch").unwrap();
    let reader = File::open(path)?;

    let static_dir = Path::new(matches.value_of("static").unwrap());

    let should_sync = matches.occurrences_of("sync") > 0
        || env::var("PUDDLE_VIZ").is_ok();

    let session = Session::new(
        Architecture::from_reader(reader),
    ).sync(should_sync);

    let mut mount = Mount::new();
    mount
        .mount("/status", status)
        .mount("/static", Static::new(static_dir))
        .mount("/", {
            let mut ioh = IoHandler::new();
            ioh.extend_with(session.to_delegate());
            IoHandlerWrapper(ioh)
        });

    // args that have defaults are safe to unwrap
    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap();
    let address = format!("{}:{}", host, port);
    println!("Listening on http://{}", address);
    Iron::new(mount).http(address)?;

    Ok(())
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
        .arg(Arg::with_name("arch")
             .value_name("ARCH_FILE")
             .help("The architecture file")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name("static")
             .long("static")
             .required(true)
             .takes_value(true)
             .validator(check_dir))
        .arg(Arg::with_name("host")
             .long("host")
             .default_value("localhost")
             .takes_value(true))
        .arg(Arg::with_name("port")
             .long("port")
             .default_value("3000")
             .takes_value(true))
        .arg(Arg::with_name("sync")
             .long("sync"))
        .get_matches();

    ::std::process::exit(match run(matches) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {}", err);
            1
        }
    });
}
