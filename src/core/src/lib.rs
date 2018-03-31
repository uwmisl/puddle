extern crate rand;

extern crate serde;
#[macro_use]
extern crate serde_derive;

extern crate serde_json;

#[cfg(test)]
extern crate glob;

#[cfg(test)]
#[macro_use]
extern crate proptest;

#[macro_use]
extern crate lazy_static;

extern crate jsonrpc_core;
#[macro_use]
extern crate jsonrpc_macros;

extern crate uuid;

#[cfg(test)]
extern crate env_logger;
#[macro_use]
extern crate log;

extern crate crossbeam;

// these need to be pub until we have an api
mod grid;
mod util;
mod exec;
mod plan;
mod command;
mod process;

pub use grid::{DropletId, DropletInfo, Grid, Location};
pub use process::*;
