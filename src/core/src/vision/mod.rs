use std::convert::AsRef;
use std::ffi::CString;
use std::path::Path;

mod cpp {
    use std::os::raw::c_char;

    extern "C" {
        pub fn detect_droplets(frame_path: *const c_char, background_path: *const c_char);
    }
}

pub fn detect_droplets(frame_path: impl AsRef<Path>, background_path: impl AsRef<Path>) {
    let frame = frame_path.as_ref();
    let background = background_path.as_ref();
    assert!(frame.is_file());
    assert!(background.is_file());

    let c_frame = CString::new(frame.to_str().unwrap()).unwrap();
    let c_background = CString::new(background.to_str().unwrap()).unwrap();
    unsafe {
        cpp::detect_droplets(c_frame.as_ptr(), c_background.as_ptr());
    }
}
