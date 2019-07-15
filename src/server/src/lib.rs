use std::error::Error;
use std::fs::File;
use std::sync::Arc;

use jsonrpc_core::IoHandler;
use jsonrpc_http_server::{
    hyper::{Body, Method, Request, Response},
    RequestMiddlewareAction, ServerBuilder,
};

use puddle_core::prelude::{Grid, Manager};

use hyper_staticfile::Static;
use structopt::StructOpt;

use futures::Future;

mod rpc;
use rpc::Rpc;

use log::*;

#[derive(StructOpt, Debug)]
pub struct Server {
    #[structopt(long = "address", default_value = "127.0.0.1:8000")]
    address: std::net::SocketAddr,
    #[structopt(long = "threads", default_value = "2")]
    threads: usize,
    #[structopt(long = "static", default_value = "static")]
    static_dir: String,
    #[structopt(long = "grid")]
    grid_file: String,
    #[structopt(long = "sync")]
    should_sync: bool,
}

fn serve(req: Request<Body>, statik: &Static) -> RequestMiddlewareAction {
    let path = req.uri().path();

    trace!("{:?}", req);

    if path.contains("..") {
        warn!("Found '..' in path!");
        return Response::builder()
            .status(404)
            .body("cannot have '..' in path".into())
            .unwrap()
            .into();
    }

    let method = req.method();
    match (path, method) {
        ("/status", _) => {
            debug!("returning status ok");
            Response::new("Server running OK.".into()).into()
        }
        ("/rpc", _) => {
            debug!("rpc");
            req.into()
        }
        (_, &Method::GET) => match statik.serve(req).wait() {
            Ok(resp) => {
                debug!("returning static file");
                resp.into()
            }
            Err(err) => {
                debug!("failed getting static file");
                Response::builder()
                    .status(404)
                    .body(format!("{:#?}", err).into())
                    .unwrap()
                    .into()
            }
        },
        _ => {
            warn!("bad request");
            Response::builder()
                .status(404)
                .body(format!("{:#?}", req).into())
                .unwrap()
                .into()
        }
    }
}

impl Server {
    pub fn run(&self) -> std::result::Result<(), Box<dyn Error>> {
        debug!("grid_file: {}", self.grid_file);
        debug!("static_dir: {}", self.static_dir);
        debug!("threads: {}", self.threads);
        debug!("address: {}", self.address);

        let grid: Grid = if self.grid_file == "-" {
            serde_yaml::from_reader(std::io::stdin())?
        } else {
            let reader = File::open(&self.grid_file)?;
            serde_yaml::from_reader(reader)?
        };

        debug!("Grid parsed.");

        let manager = Arc::new(Manager::new(self.should_sync, grid));

        debug!("Manager created.");

        let mut io = IoHandler::default();
        io.extend_with(manager.to_delegate());

        debug!("IoHandler created.");

        let statik = Static::new(&self.static_dir);

        let server = ServerBuilder::new(io)
            .threads(self.threads)
            .request_middleware(move |req| serve(req, &statik))
            .start_http(&self.address)
            .expect("Unable to start RPC server");

        debug!("Server created.");

        server.wait();

        info!("Shutting down");
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse() {
        let args = "progname --static dir/ --address 1.2.3.4:9999 --grid dir/file.ext --threads 12";
        Server::from_iter(args.split_whitespace());
    }
}
