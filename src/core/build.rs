// build.rs

extern crate cc;
extern crate pkg_config;

use std::env;

fn main() {
    if let Ok(_) = env::var("CARGO_FEATURE_VISION") {
        pkg_config::Config::new()
            .atleast_version("3.0.0")
            .statik(false)
            .probe("opencv")
            .unwrap();

        cc::Build::new()
            .cpp(true)
        // .flag("--std=c++11")
        // .cpp_link_stdlib("c++")
            .static_flag(true)
            .shared_flag(false)
            .file("src/vision/droplet_detect.cpp")
            .compile("vision");
    }
}
