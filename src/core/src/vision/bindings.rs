use std::ops::Deref;
use std::slice;

extern "C" {
    pub fn detect_from_camera(state: *const DetectionState, response: *const DetectionResponse, should_draw: bool) -> bool;
    pub fn makeDetectionState() -> *const DetectionState;
}

#[repr(C)]
pub struct MyPoint {
    pub y: u32,
    pub x: u32,
}

#[repr(C)]
pub struct Contour {
    len: usize,
    points: *const MyPoint,
}

impl Deref for Contour {
    type Target = [MyPoint];

    fn deref(&self) -> &[MyPoint] {
        unsafe { slice::from_raw_parts(self.points, self.len) }
    }
}

// dont' implement drop for now, they are freed on the C++ side
// impl Drop for Contour {
//     fn drop(&mut self) {
//         unsafe { libc::free(self.points as *mut libc::c_void) };
//     }
// }

pub enum DetectionState {}

#[repr(C)]
pub struct DetectionResponse {
    len: usize,
    contours: *const Contour,
    pub penta_center: MyPoint,
    pub square_center: MyPoint,
}

impl Default for DetectionResponse {
    fn default() -> DetectionResponse {
        DetectionResponse {
            len: 0,
            contours: ::std::ptr::null(),
            penta_center: MyPoint {y: 0, x: 0},
            square_center: MyPoint {y: 0, x: 0},
        }
    }
}

impl Deref for DetectionResponse {
    type Target = [Contour];

    fn deref(&self) -> &[Contour] {
        unsafe { slice::from_raw_parts(self.contours, self.len) }
    }
}

// impl Drop for DetectionResponse {
//     fn drop(&mut self) {
//         unsafe { libc::free(self.contours as *mut libc::c_void) };
//     }
// }
