use puddle_server::Server;
use structopt::StructOpt;

use log::*;

fn main() {
    let _ = env_logger::try_init();
    let server = Server::from_args();
    debug!("Server parsed!");
    server.run().unwrap();
}
