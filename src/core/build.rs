// build.rs

use std::env;

fn main() {
    // If we're in CI, just don't build the cpp
    if let Ok(_) = env::var("CI") {
        return;
    }
    if let Ok(_) = env::var("CARGO_FEATURE_VISION") {
        println!("cargo:rustc-link-lib=opencv_core");
        println!("cargo:rustc-link-lib=opencv_imgproc");
        println!("cargo:rustc-link-lib=opencv_imgcodecs");
        println!("cargo:rustc-link-lib=opencv_video");
        println!("cargo:rustc-link-lib=opencv_videoio");
        println!("cargo:rustc-link-lib=opencv_highgui");

        cc::Build::new()
            .cpp(true)
            .flag("-std=c++11")
            .file("src/vision/droplet_detect.cpp")
            .compile("vision");
    }
}
