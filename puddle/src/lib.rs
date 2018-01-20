#[macro_use]
extern crate serde_derive;
extern crate serde;

extern crate serde_yaml;

#[cfg(test)]
extern crate glob;

#[cfg(test)]
#[macro_use]
extern crate proptest;


mod minheap;

// these need to be pub until we have an api
pub mod arch;
pub mod routing;
