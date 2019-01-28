#![cfg_attr(feature = "cargo-clippy", allow(module_inception))]
#![cfg_attr(feature = "cargo-clippy", allow(redundant_field_names))]
#![deny(bare_trait_objects)]

// we use log macros everywhere, so just save the import
#[macro_use]
extern crate log;

// use matches::assert_matches doesn't work for some reason
#[macro_use]
extern crate matches;

// these need to be pub until we have an api
mod command;
mod exec;
pub mod grid;
pub mod plan;
mod process;
mod system;
pub mod util;

#[cfg(feature = "vision")]
pub mod vision;

#[cfg(feature = "pi")]
pub mod pi;

pub use crate::exec::Executor;
pub use crate::grid::parse;
pub use crate::grid::{Blob, DropletId, DropletInfo, Grid, Location};
pub use crate::process::*;

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
