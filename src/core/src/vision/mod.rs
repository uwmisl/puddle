use std::ffi::CString;
use std::os::raw::c_char;
use std::path::Path;
use std::process::Command;
use std::slice;
use std::sync::{Arc, Mutex};

use grid::{Blob, Droplet, DropletId, Location, SimpleBlob};

mod transform;
use self::transform::GridTransformer;

use nalgebra::{Isometry2, Point2};
use ncollide2d as nc;
use ncollide2d::{
    bounding_volume::{HasBoundingVolume, AABB}, query::PointQuery, shape::ConvexPolygon,
};

// Points are x,y in nalgebra, but we just ignore those names. We only use
// pt[0], pt[1] instead because we use them as y,x
type Point = Point2<f32>;

extern "C" {
    fn detect_from_camera(
        state: *const DetectionState,
        response: *const DetectionResponse,
        should_draw: bool,
    ) -> bool;
    fn makeDetectionState(
        trackbars: bool,
        src_path: *const c_char,
        dst_path: *const c_char,
    ) -> *const DetectionState;
}

#[repr(C)]
struct MyPoint {
    y: u32,
    x: u32,
}

impl MyPoint {
    fn to_point(&self) -> Point {
        // Points in ncollide2d are x then y!
        Point::new(self.y as f32, self.x as f32)
    }
}

#[repr(C)]
struct Contour {
    len: usize,
    points: *const MyPoint,
}

impl Contour {
    fn to_point_vec(&self) -> Vec<Point> {
        let slice = unsafe { slice::from_raw_parts(self.points, self.len) };
        slice.iter().map(|my_point| my_point.to_point()).collect()
    }
}

enum DetectionState {}

#[repr(C)]
struct DetectionResponse {
    len: usize,
    contours: *const Contour,
    penta_center: MyPoint,
    square_center: MyPoint,
}

impl Default for DetectionResponse {
    fn default() -> DetectionResponse {
        DetectionResponse {
            len: 0,
            contours: ::std::ptr::null(),
            penta_center: MyPoint { y: 0, x: 0 },
            square_center: MyPoint { y: 0, x: 0 },
        }
    }
}

impl DetectionResponse {
    fn contours(&self) -> Vec<Vec<Point>> {
        let slice = unsafe { slice::from_raw_parts(self.contours, self.len) };
        slice.iter().map(|cont| cont.to_point_vec()).collect()
    }
}

pub struct Detector {
    state: *const DetectionState,
    response: DetectionResponse,
    transformer: GridTransformer,

    // these are only used to keep the paths alive until the detector might need the pointers
    #[allow(dead_code)]
    src_path: Option<CString>,
    #[allow(dead_code)]
    dst_path: Option<CString>,
}

impl Detector {
    pub fn new(trackbars: bool) -> Detector {
        initialize_camera();
        let null = ::std::ptr::null();
        Detector {
            state: unsafe { makeDetectionState(trackbars, null, null) },
            response: DetectionResponse::default(),
            transformer: GridTransformer::default(),
            src_path: None,
            dst_path: None,
        }
    }

    pub fn from_filename(src: &Path, dst: &Path) -> Detector {
        let c_src = {
            assert!(src.is_file());
            let src_str = src.to_str().expect("Source filename was invalid string!");
            CString::new(src_str).unwrap()
        };
        let c_dst = {
            let dst_str = dst.to_str()
                .expect("Destination filename was invalid string!");
            CString::new(dst_str).unwrap()
        };

        let trackbars = false;
        Detector {
            state: unsafe { makeDetectionState(trackbars, c_src.as_ptr(), c_dst.as_ptr()) },
            response: DetectionResponse::default(),
            transformer: GridTransformer::default(),
            src_path: Some(c_src),
            dst_path: Some(c_dst),
        }
    }

    pub fn detect(&mut self, should_draw: bool) -> (bool, Vec<PolygonBlob>) {
        // after detect_from_camera from camera is called, data is *unsafely*
        // stored in DetectionResponse
        let should_quit = unsafe { detect_from_camera(self.state, &self.response, should_draw) };

        let raw_contours = self.response.contours();

        let blobs: Vec<_> = raw_contours
            .iter()
            .map(|points| {
                let transformed_points: Vec<_> = points
                    .iter()
                    .map(|pt| self.transformer.transform(pt))
                    .collect();
                let polygon = ConvexPolygon::try_from_points(&transformed_points).unwrap();
                PolygonBlob { polygon }
            })
            .collect();

        trace!(
            "Found {} blobs: {:#?}",
            blobs.len(),
            blobs
                .iter()
                .map(|b| {
                    let ident = Isometry2::identity();
                    let bbox: AABB<f32> = b.polygon.bounding_volume(&ident);
                    bbox
                })
                .collect::<Vec<_>>()
        );
        debug!("Blobs represent these droplets with fake ids: {:#?}", {
            let id = DropletId {
                id: 0,
                process_id: 0,
            };
            blobs
                .iter()
                .map(|b| {
                    // NOTE: to_droplet will panic if location or dimensions are negative
                    ::std::panic::catch_unwind(|| b.to_droplet(id))
                })
                .collect::<Vec<_>>()
        });

        if should_quit {
            info!("Detector should quit soon")
        }

        (should_quit, blobs)
    }

    pub fn run(&mut self, should_draw: bool, blobs: Arc<Mutex<Vec<PolygonBlob>>>) {
        loop {
            let (should_quit, new_blobs) = self.detect(should_draw);
            *blobs.lock().unwrap() = new_blobs;

            if should_quit {
                break;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PolygonBlob {
    polygon: ConvexPolygon<f32>,
}

impl PolygonBlob {
    pub fn touches_location(&self, location: Location) -> bool {
        self.touches_rectangle(location, Location { y: 1, x: 1 })
    }

    pub fn touches_rectangle(&self, location: Location, dimensions: Location) -> bool {
        let delta = 0.2;
        let ident = Isometry2::identity();
        let (n_pts, pts) = points_in_area(location, dimensions, delta);
        let n_pts_in_shape = pts.filter(|pt| self.polygon.contains_point(&ident, pt))
            .count();

        n_pts_in_shape > 0
    }

    pub fn area(&self) -> f32 {
        // from https://stackoverflow.com/a/451482/
        // e0 is points 0..n, e1 is points 1..n,0
        // zipping them gives you all the edges of the polygon
        let e0 = self.polygon.points().iter();
        let mut e1 = self.polygon.points().iter().cycle();
        e1.next();

        let area: f32 = e0.zip(e1)
            .map(|(p0, p1)| p0[0] * p1[1] - p0[1] * p1[0])
            .sum();
        area.abs() / 2.0
    }
}

const BASE_DISTANCE: i32 = 1000;

impl Blob for PolygonBlob {
    fn get_similarity(&self, droplet: &Droplet) -> i32 {
        let ident = Isometry2::identity();
        let distance =
            nc::query::distance(&ident, &self.polygon, &ident, &droplet_to_shape(droplet));

        if distance > 0.0 {
            // TODO this round is probably too low precision
            let i_distance = distance.ceil() as i32;
            return i_distance + BASE_DISTANCE;
        }

        // FIXME compare volumes

        let delta = 0.2;
        let (n_pts, pts) = points_in_area(droplet.location, droplet.dimensions, delta);
        let n_pts_in_shape = pts.filter(|pt| self.polygon.contains_point(&ident, pt))
            .count();

        assert!((n_pts as i32) < BASE_DISTANCE);

        BASE_DISTANCE - n_pts_in_shape as i32
    }

    fn to_simple_blob(&self) -> SimpleBlob {
        let ident = Isometry2::identity();
        let bbox: AABB<f32> = self.polygon.bounding_volume(&ident);
        let loc_point = bbox.mins();
        let dim_point = bbox.maxs() - loc_point;
        // note the xy flip here, in nalgebra::Point, x is the first field, and y is the seconds
        let location = Location {
            y: loc_point[0].round() as i32,
            x: loc_point[1].round() as i32,
        };
        let dimensions = Location {
            y: dim_point[0].round() as i32,
            x: dim_point[1].round() as i32,
        };
        let volume = self.area().into();

        SimpleBlob {
            location,
            dimensions,
            volume,
        }
    }
}

fn droplet_to_shape(droplet: &Droplet) -> ConvexPolygon<f32> {
    let y = droplet.location.y as f32;
    let x = droplet.location.x as f32;
    let dy = droplet.dimensions.y as f32;
    let dx = droplet.dimensions.x as f32;

    assert!(dy > 0.0);
    assert!(dx > 0.0);

    let corners = vec![
        Point::new(y, x),
        Point::new(y + dy, x),
        Point::new(y + dy, x + dx),
        Point::new(y, x + dx),
    ];

    // the try_new constructor *assumes* the convexity of the points
    ConvexPolygon::try_new(corners).unwrap()
}

// no whitespace, these are passed to the shell
const VIDEO_CONFIG: &[&str] = &[
    "iso_sensitivity=1",
    "white_balance_auto_preset=1",
    "auto_exposure=0",
    "red_balance=1000",
    "blue_balance=1000",
    "saturation=00",
    "exposure_time_absolute=1000",
];

fn initialize_camera() {
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

fn points_in_area(
    location: Location,
    dimension: Location,
    delta: f32,
) -> (usize, impl Iterator<Item = Point>) {
    let mut y = location.y as f32;
    let mut x = location.x as f32;

    assert!(dimension.y > 0);
    assert!(dimension.x > 0);

    // take the floor then add one to make sure we get the boundary
    let y_steps = (dimension.y as f32 / delta) as usize + 1;
    let x_steps = (dimension.x as f32 / delta) as usize + 1;

    let iter = (0..y_steps)
        .map(move |_| {
            let dy = y;
            y += delta;
            dy
        })
        .flat_map(move |dy| {
            (0..x_steps).map(move |_| {
                let dx = x;
                x += delta;
                Point::new(dy, dx)
            })
        });

    (y_steps * x_steps, iter)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_points_in_area() {
        let loc = Location { y: 0, x: 0 };
        let dim = Location { y: 2, x: 1 };

        let y0 = loc.y as f32;
        let x0 = loc.x as f32;
        let y1 = (loc.y + dim.y) as f32;
        let x1 = (loc.x + dim.x) as f32;

        {
            let (n_pts, pts_iter) = points_in_area(loc, dim, 0.5);
            let pts: Vec<_> = pts_iter.collect();
            assert_eq!(n_pts, pts.len());
            assert_eq!(n_pts, 15);
            for pt in pts {
                assert!(y0 <= pt.y);
                assert!(pt[0] <= y1);
                assert!(x0 <= pt.x);
                assert!(pt[1] <= x1);
            }
        }
        {
            let (n_pts, pts_iter) = points_in_area(loc, dim, 0.3);
            let pts: Vec<_> = pts_iter.collect();
            assert_eq!(n_pts, pts.len());
            assert_eq!(n_pts, 28);
            for pt in pts {
                assert!(y0 <= pt.y);
                assert!(pt[0] <= y1);
                assert!(x0 <= pt.x);
                assert!(pt[1] <= x1);
            }
        }
    }

    fn blob_from_pts(pts: &[(f32, f32)]) -> PolygonBlob {
        let points: Vec<Point> = pts.iter().map(|(y, x)| Point::new(*y, *x)).collect();
        let polygon = ConvexPolygon::try_from_points(&points).unwrap();
        PolygonBlob { polygon }
    }

    fn droplet_from_corners(mins: (f32, f32), maxs: (f32, f32)) -> Droplet {
        let (y0, x0) = mins;
        let (y1, x1) = maxs;
        let blob = blob_from_pts(&vec![(y0, x0), (y1, x0), (y1, x1), (y0, x1)]);
        blob.to_droplet(DropletId {
            id: 0,
            process_id: 0,
        })
    }

    #[test]
    fn test_blob_to_droplet() {
        let d = droplet_from_corners((0.9, 0.1), (1.8, 1.4));
        println!("{:#?}", d);
        assert_eq!(d.location, Location { y: 1, x: 0 });
        assert_eq!(d.dimensions, Location { y: 1, x: 1 });

        let d = droplet_from_corners((4.7, 4.1), (5.8, 5.2));
        println!("{:#?}", d);
        assert_eq!(d.location, Location { y: 5, x: 4 });
        assert_eq!(d.dimensions, Location { y: 1, x: 1 });
    }

    #[test]
    fn test_polygon_area() {
        let square_blob = blob_from_pts(&vec![(0.0, 1.0), (0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]);
        assert_eq!(square_blob.area(), 1.0);

        let penta_blob = blob_from_pts(&vec![
            (0.0, 0.0),
            (0.0, 2.0),
            (2.0, 4.0),
            (4.0, 3.0),
            (3.0, 1.0),
        ]);
        assert_eq!(penta_blob.area(), 9.5);
    }
}
