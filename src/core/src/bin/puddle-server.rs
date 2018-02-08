extern crate puddle_core;

extern crate iron;
extern crate mount;
extern crate staticfile;

extern crate jsonrpc_core;

use std::fmt;
use std::fs::File;
use std::path::Path;
use std::io::Read;

use iron::prelude::*;
use iron::Handler;
use iron::headers::ContentType;
use iron::status;

use mount::Mount;
use staticfile::Static;

use jsonrpc_core::IoHandler;

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
        println!("body: {}", body);

        // handle the request with jsonrpc, then convert to IronResult
        self.0.handle_request_sync(&body)
            .map(|resp| Response::with((ContentType::json().0, status::Ok, resp)))
            .ok_or(IronError::new(JsonRpcError, (status::InternalServerError, "jsonrpc error")))
    }
}

fn status(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "All is good!")))
}

fn main() {
    use std::env::args;
    let path = args().nth(1).unwrap();
    let reader = File::open(path).expect("file not found");
    let session = Session::new(Architecture::from_reader(reader));


    let mut mount = Mount::new();
    mount
        .mount("/status", status)
        .mount("/static", Static::new(Path::new("target/doc/")))
        .mount("/", {
            let mut ioh = IoHandler::new();
            ioh.extend_with(session.to_delegate());
            IoHandlerWrapper(ioh)
        });

    let address = "localhost:3000";
    println!("Listening on http://{}", address);
    Iron::new(mount).http(address).unwrap();
}
