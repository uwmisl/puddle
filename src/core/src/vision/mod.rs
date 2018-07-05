use std::process::Command;
use std::slice;
use std::sync::{Arc, Mutex};

use grid::{Blob, Droplet, DropletId, Location};

use nalgebra::{geometry::Translation2, norm, Isometry2, Point2, Similarity2, UnitComplex};
use ncollide2d as nc;
use ncollide2d::{
    bounding_volume::{HasBoundingVolume, AABB}, query::PointQuery, shape::ConvexPolygon,
};


type Point = Point2<f32>;

extern "C" {
    fn detect_from_camera(
        state: *const DetectionState,
        response: *const DetectionResponse,
        should_draw: bool,
    ) -> bool;
    fn makeDetectionState(trackbars: bool) -> *const DetectionState;
}

#[repr(C)]
struct MyPoint {
    y: u32,
    x: u32,
}

impl MyPoint {
    fn to_point(&self) -> Point {
        // Points in ncollide2d are x then y!
        Point::new(self.x as f32, self.y as f32)
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
}

impl Detector {
    pub fn new(trackbars: bool) -> Detector {
        initialize_camera();
        Detector {
            state: unsafe { makeDetectionState(trackbars) },
            response: DetectionResponse::default(),
        }
    }

    pub fn detect(&mut self, should_draw: bool) -> (bool, Vec<PolygonBlob>) {
        // after detect_from_camera from camera is called, data is *unsafely*
        // stored in DetectionResponse
        let should_quit = unsafe { detect_from_camera(self.state, &self.response, should_draw) };

        let raw_contours = self.response.contours();
        let square_center = self.response.square_center.to_point();
        let penta_center = self.response.penta_center.to_point();

        // the y coordinates (first) were measured from an image
        // the x coordinates (second) are taken from the alignment of the design
        let square_center_measured = Point::new(0.5, -1.424);
        let penta_center_measured = Point::new(7.5, -1.357);

        let similarity = match_fiducial(
            square_center_measured,
            penta_center_measured,
            square_center,
            penta_center,
        );

        let blobs: Vec<_> = raw_contours
            .iter()
            .map(|points| {
                let transformed_points: Vec<_> = points.iter().map(|pt| similarity * pt).collect();
                let polygon = ConvexPolygon::try_from_points(&transformed_points).unwrap();
                PolygonBlob {polygon}
            })
            .collect();

        trace!("Found {} blobs: {:#?}", blobs.len(),
               blobs.iter().map(|b| {
                   let ident = Isometry2::identity();
                   let bbox: AABB<f32> = b.polygon.bounding_volume(&ident);
                   bbox
               }).collect::<Vec<_>>()
        );
        debug!("Blobs represent these droplets with fake ids: {:#?}", {
            let id = DropletId { id: 0, process_id: 0 };
            blobs.iter().map(|b| {
                // NOTE: to_droplet will panic if location or dimensions are negative
                ::std::panic::catch_unwind(|| b.to_droplet(id))
            }).collect::<Vec<_>>()
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
                break
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PolygonBlob {
    polygon: ConvexPolygon<f32>,
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

    fn to_droplet(&self, id: DropletId) -> Droplet {
        let ident = Isometry2::identity();
        let bbox: AABB<f32> = self.polygon.bounding_volume(&ident);
        let loc_point = bbox.mins();
        let dim_point = bbox.maxs() - loc_point;
        let location = Location {
            y: loc_point.y.round() as i32,
            x: loc_point.x.round() as i32,
        };
        let dimensions = Location {
            y: dim_point.y.round() as i32,
            x: dim_point.x.round() as i32,
        };
        // FIXME this is fake!
        let volume = 1.0;

        Droplet::new(id, volume, location, dimensions)
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
        Point::new(x, y),
        Point::new(x, y + dy),
        Point::new(x + dx, y + dy),
        Point::new(x + dx, y),
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

///
/// d0: the desired first fiducial coordinate
/// d1: the desired second fiducial coordinate
/// m0: the measured first fiducial coordinate
/// m1: the measured second fiducial coordinate
fn match_fiducial(d0: Point, d1: Point, m0: Point, m1: Point) -> Similarity2<f32> {
    let vec_d = d1 - d0;
    let vec_m_prescale = m1 - m0;
    let scale = norm(&vec_d) / norm(&vec_m_prescale);

    let m0_scaled = m0 * scale;
    let m1_scaled = m1 * scale;
    trace!("m0_scaled: {}", m0_scaled);
    trace!("m1_scaled: {}", m1_scaled);

    let vec_m = m1_scaled - m0_scaled;
    let rotation = UnitComplex::rotation_between(&vec_m, &vec_d);
    trace!("rotation: {}", rotation);

    let translation = Translation2::from_vector(d0 - rotation * m0_scaled);
    trace!("translation: {}", translation);
    Similarity2::from_parts(translation, rotation, scale)
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
                Point::new(dx, dy)
            })
        });

    (y_steps * x_steps, iter)
}

#[cfg(test)]
mod tests {

    use super::*;

    use nalgebra::{
        base::Unit, geometry::Translation2, norm, Point2, Similarity2, UnitComplex, Vector2,
    };
    use ncollide2d as nc;

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
                assert!(pt.y <= y1);
                assert!(x0 <= pt.x);
                assert!(pt.x <= x1);
            }
        }
        {
            let (n_pts, pts_iter) = points_in_area(loc, dim, 0.3);
            let pts: Vec<_> = pts_iter.collect();
            assert_eq!(n_pts, pts.len());
            assert_eq!(n_pts, 28);
            for pt in pts {
                assert!(y0 <= pt.y);
                assert!(pt.y <= y1);
                assert!(x0 <= pt.x);
                assert!(pt.x <= x1);
            }
        }
    }

    fn assert_close(p0: Point, p1: Point) {
        let epsilon = 0.00001f32;
        let diff = p0 - p1;
        let dist = norm(&diff);
        if dist > epsilon {
            panic!("{} and {} too far: {}", p0, p1, dist)
        }
    }

    #[test]
    fn test_match_fiducial() {
        {
            let d0 = Point::new(0.0, 0.0);
            let d1 = Point::new(0.0, 1.0);
            let m0 = Point::new(1.0, 1.0);
            let m1 = Point::new(2.0, 2.0);
            let sim = match_fiducial(d0, d1, m0, m1);
            trace!("sim: {}", sim);

            trace!("d0:  {}", d0);
            trace!("d1:  {}", d1);
            trace!("m0': {}", sim * m0);
            trace!("m1': {}", sim * m1);

            assert_close(d0, sim * m0);
            assert_close(d1, sim * m1);
        }

        {
            let d0 = Point::new(-1.0, -1.0);
            let d1 = Point::new(0.0, 1.0);
            let m0 = Point::new(1.0, 1.0);
            let m1 = Point::new(2.0, 2.0);
            let sim = match_fiducial(d0, d1, m0, m1);

            assert_close(d0, sim * m0);
            assert_close(d1, sim * m1);
        }
    }

    fn droplet_from_corners(mins: (f32, f32), maxs: (f32, f32)) -> Droplet {
        let (y0, x0) = mins;
        let (y1, x1) = maxs;
        let polygon = ConvexPolygon::try_new(vec![
            Point::new(x0, y0),
            Point::new(x0, y1),
            Point::new(x1, y1),
            Point::new(x1, y0),
        ]).unwrap();
        let blob = PolygonBlob { polygon };
        blob.to_droplet(DropletId { id: 0, process_id: 0})
    }

    #[test]
    fn test_blob_to_droplet() {
        let d = droplet_from_corners((0.9, 0.1), (1.8, 1.4));
        println!("{:#?}", d);
        assert_eq!(d.location, Location {y: 1, x: 0});
        assert_eq!(d.dimensions, Location {y: 1, x: 1});

        let d = droplet_from_corners((4.7, 4.1), (5.8, 5.2));
        println!("{:#?}", d);
        assert_eq!(d.location, Location {y: 5, x: 4});
        assert_eq!(d.dimensions, Location {y: 1, x: 1});
    }
}
