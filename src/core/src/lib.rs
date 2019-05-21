#![warn(clippy::correctness)]
#![warn(clippy::style)]
#![warn(clippy::complexity)]
#![warn(clippy::perf)]
// #![warn(clippy::cargo)]
// #![warn(clippy::pedantic)]
// #![warn(clippy::nursery)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::module_inception)]

#[macro_use]
extern crate log;

// these need to be pub until we have an api
mod command;
mod exec;
pub mod grid;
pub mod plan;
mod process;
pub mod util;

mod system;

#[cfg(feature = "vision")]
pub mod vision;

#[cfg(feature = "pi")]
pub mod pi;

pub use exec::Executor;
pub use grid::parse;
pub use grid::{Blob, DropletId, DropletInfo, Grid, Location};
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
        project_root() + "/" + s
    }
}
