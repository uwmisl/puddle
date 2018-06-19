// build.rs

extern crate cc;
extern crate pkg_config;

fn main() {
    pkg_config::Config::new()
        .atleast_version("3.0.0")
        .probe("opencv")
        .unwrap();

    cc::Build::new()
        .cpp(true)
        // .flag("--std=c++11")
        // .cpp_link_stdlib("c++")
        .file("src/vision/droplet_detect.cpp")
        .compile("vision");
}
