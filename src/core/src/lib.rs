extern crate rand;

extern crate serde;
#[macro_use]
extern crate serde_derive;

extern crate serde_json;

#[cfg(test)]
extern crate glob;

extern crate jsonrpc_core;
#[macro_use]
extern crate jsonrpc_macros;

#[cfg(test)]
extern crate env_logger;
#[macro_use]
extern crate log;

extern crate crossbeam;

extern crate ena;

extern crate pathfinding;

extern crate float_ord;

extern crate rppal;

// these need to be pub until we have an api
mod command;
mod exec;
mod grid;
mod plan;
mod process;
mod util;

pub use exec::Executor;
pub use grid::parse;
pub use grid::{DropletId, DropletInfo, Grid, Location};
pub use process::*;

#[cfg(test)]
mod tests {
    use std::process::Command;

    pub fn project_root() -> String {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--show-toplevel")
            .output()
            .expect("Couldn't run `git`!");
        String::from_utf8_lossy(&output.stdout).trim().into()
    }

    pub fn project_path(s: &str) -> String {
        project_root() + s
    }
}
