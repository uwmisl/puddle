extern crate env_logger;
extern crate hyper_staticfile;
extern crate jsonrpc_core;
extern crate jsonrpc_http_server;
extern crate structopt;

extern crate puddle_core;

use std::{env, fs::File, net::SocketAddr, path::Path, sync::Arc};

use jsonrpc_core::{futures::Future, IoHandler};
use jsonrpc_http_server::{hyper, RequestMiddlewareAction, Response, ServerBuilder};
use structopt::StructOpt;

use puddle_core::{grid::parse::ParsedGrid, Grid, Manager, Rpc};

#[derive(StructOpt)]
struct PuddleServer {
    #[structopt(long, default_value = "127.0.0.1:3000")]
    addr: SocketAddr,
    #[structopt(long = "static")]
    static_dir: String,
    #[structopt(long)]
    should_sync: bool,
    #[structopt(long = "arch")]
    arch_file: String,
}

macro_rules! exit {
    ($($arg:tt)*) => ({
        eprintln!($($arg)*);
        ::std::process::exit(1);
    })
}

impl PuddleServer {
    fn run(&self) -> Result<(), Box<::std::error::Error>> {
        if !Path::new(&self.static_dir).is_dir() {
            exit!("static was not a directory: {}", self.static_dir)
        }

        if !Path::new(&self.arch_file).is_file() {
            exit!("arch was not a file: {}", self.arch_file)
        }

        // required argument is safe to unwrap
        let reader = File::open(&self.arch_file)?;

        let static_dir = hyper_staticfile::Static::new(&self.static_dir);

        let should_sync = self.should_sync || env::var("PUDDLE_VIZ").is_ok();

        let pg = ParsedGrid::from_reader(reader)?;
        let manager = Manager::new(should_sync, pg.to_grid(), pg.pi_config);
        let arc = Arc::new(manager);

        #[cfg(feature = "pi")]
        {
            println!("Make sure to manually set the voltage for the pi!");
            println!("Something like: pi-test dac 1000");
        }

        let mut io = IoHandler::new();
        io.extend_with(arc.to_delegate());

        let server = ServerBuilder::new(io)
            .request_middleware(
                move |request: hyper::Request<hyper::Body>| -> RequestMiddlewareAction {
                    if request.uri() == "/status" {
                        Response::ok("Server running OK.").into()
                    } else if request.uri() == "/rpc" {
                        // pass it along
                        request.into()
                    } else {
                        RequestMiddlewareAction::Respond {
                            should_validate_hosts: true,
                            response: Box::new(
                                static_dir.serve(request).map_err(|e| panic!("{:#?}", e)),
                            ),
                        }
                    }
                },
            ).start_http(&self.addr)
            .expect("Couldn't start server");

        server.wait();

        Ok(())
    }
}

fn main() {
    // enable logging
    let _ = env_logger::try_init();

    let server = PuddleServer::from_args();

    if let Err(e) = server.run() {
        exit!("error: {}", e);
    }
}
