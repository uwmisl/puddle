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
pub mod command;
pub mod exec;
pub mod grid;
pub mod plan;
pub mod process;
pub mod util;

mod system;

pub mod prelude {
    pub use crate::{
        exec::Executor,
        grid::{Blob, DropletId, DropletInfo, Grid, Location},
        process::{Manager, Process, ProcessId, PuddleError},
    };
}

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
