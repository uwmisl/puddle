// build.rs

extern crate cc;

use std::env;

fn main() {
    if let Ok(_) = env::var("CARGO_FEATURE_VISION") {
        println!("cargo:rustc-link-lib=opencv_core");
        println!("cargo:rustc-link-lib=opencv_imgproc");
        println!("cargo:rustc-link-lib=opencv_imgcodecs");
        println!("cargo:rustc-link-lib=opencv_videoio");
        println!("cargo:rustc-link-lib=opencv_highgui");

        cc::Build::new()
            .cpp(true)
            .file("src/vision/droplet_detect.cpp")
            .compile("vision");
    }
}
