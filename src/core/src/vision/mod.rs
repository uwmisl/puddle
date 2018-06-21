use std::convert::AsRef;
use std::ffi::CString;
use std::path::Path;
use std::process::Command;

mod cpp {
    use std::os::raw::c_char;

    extern "C" {
        pub fn detect_from_files(frame_path: *const c_char, background_path: *const c_char);
        pub fn detect_from_camera();
    }
}

// no whitespace, these are passed to the shell
const VIDEO_CONFIG: &[&str] = &[
    "iso_sensitivity_auto=0",
    "white_balance_auto_preset=0",
    "auto_exposure=0",
];

pub fn initialize_camera() {
    for config in VIDEO_CONFIG {
        let output = Command::new("v4l2-ctl")
            .arg("-c")
            .arg(config)
            .output()
            .expect("command failed to run");

        if !output.status.success() {
            error!(
                "Trying to set {}, failed with code {}: \nstdout: '{}'\nstderr: '{}'",
                config,
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            panic!("Failed");
        }
    }
}

pub fn detect_from_camera() {
    unsafe {
        cpp::detect_from_camera();
    }
}

pub fn detect_from_files(frame_path: impl AsRef<Path>, background_path: impl AsRef<Path>) {
    let frame = frame_path.as_ref();
    let background = background_path.as_ref();
    assert!(frame.is_file());
    assert!(background.is_file());

    let c_frame = CString::new(frame.to_str().unwrap()).unwrap();
    let c_background = CString::new(background.to_str().unwrap()).unwrap();
    unsafe {
        cpp::detect_from_files(c_frame.as_ptr(), c_background.as_ptr());
    }
}
