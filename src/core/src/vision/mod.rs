use std::process::Command;

mod bindings;
use self::bindings::{Contour, DetectionResponse, DetectionState, MyPoint};

// no whitespace, these are passed to the shell
const VIDEO_CONFIG: &[&str] = &[
    "iso_sensitivity_auto=0",
    "white_balance_auto_preset=0",
    "auto_exposure=0",
];

pub struct Detector {
    state: *const DetectionState,
    response: DetectionResponse,
}

impl Detector {
    pub fn new() -> Detector {
        initialize_camera();
        Detector {
            state: unsafe { bindings::makeDetectionState() },
            response: DetectionResponse::default(),
        }
    }

    pub fn detect(&mut self, should_draw: bool) -> bool {
        let should_quit =
            unsafe { bindings::detect_from_camera(self.state, &self.response, should_draw) };
        should_quit
    }
}

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

// pub fn match_contour_with_droplet(contour: Vec<Point2d>) {

// }

#[cfg(test)]
mod tests {

    use nalgebra::{
        base::Unit, geometry::Translation2, norm, Point2, Similarity2, UnitComplex, Vector2,
    };
    use ncollide2d as nc;

    #[test]
    fn test_something() {
        #[cfg_attr(rustfmt, rustfmt_skip)]
        let raw_pts = vec![
            (366, 256), (365, 257), (364, 257), (363, 258), (359, 258),
            (359, 259), (360, 260), (360, 261), (361, 260), (362, 260),
            (363, 261), (363, 263), (362, 264), (359, 264), (358, 263),
            (358, 261), (357, 262), (357, 263), (356, 264), (355, 264),
            (354, 265), (353, 264), (353, 263), (352, 263), (351, 262),
            (350, 262), (348, 264), (348, 266), (349, 267), (348, 268),
            (348, 272), (349, 273), (350, 273), (352, 275), (356, 275),
            (358, 277), (357, 278), (356, 278), (356, 280), (355, 281),
            (353, 279), (354, 278), (354, 277), (351, 277), (351, 281),
            (352, 282), (352, 283), (355, 283), (356, 284), (357, 283),
            (359, 285), (359, 286), (360, 285), (361, 286), (363, 286),
            (364, 287), (367, 287), (368, 286), (371, 286), (372, 285),
            (374, 285), (375, 284), (376, 284), (378, 282), (378, 278),
            (377, 277), (378, 276), (378, 275), (379, 274), (381, 274),
            (381, 271), (380, 270), (380, 269), (379, 268), (379, 266),
            (378, 265), (378, 260), (374, 256), (372, 256), (371, 257),
            (370, 256),
        ];

        // the y coordinates (first) were measured from an image
        // the x coordinates (second) are taken from the alignment of the design
        let square_center = Point2::new(-1.424, 0.5);
        let penta_center = Point2::new(-1.357, 7.5);

        let pts: Vec<Point2<f32>> = raw_pts
            .iter()
            .map(|&(y, x)| Point2::new(y as _, x as _))
            .collect();

        let shape = nc::shape::ConvexPolygon::try_from_points(&pts).unwrap();
    }

    // fn points_in_area(location: Location, dimension: Location, delta: f32) -> impl Iterator<Item=(f)>

    ///
    /// d0: the desired first fiducial coordinate
    /// d1: the desired second fiducial coordinate
    /// m0: the measured first fiducial coordinate
    /// m1: the measured second fiducial coordinate
    fn match_fiducial(
        d0: Point2<f32>,
        d1: Point2<f32>,
        m0: Point2<f32>,
        m1: Point2<f32>,
    ) -> Similarity2<f32> {
        let vec_d = d1 - d0;
        let vec_m_prescale = m1 - m0;
        let scale = norm(&vec_d) / norm(&vec_m_prescale);

        let m0_scaled = m0 * scale;
        let m1_scaled = m1 * scale;
        println!("m0_scaled: {}", m0_scaled);
        println!("m1_scaled: {}", m1_scaled);

        let vec_m = m1_scaled - m0_scaled;
        let rotation = UnitComplex::rotation_between(&vec_m, &vec_d);
        println!("rotation: {}", rotation);

        let translation = Translation2::from_vector(d0 - rotation * m0_scaled);
        println!("translation: {}", translation);
        Similarity2::from_parts(translation, rotation, scale)
    }

    fn assert_close(p0: Point2<f32>, p1: Point2<f32>) {
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
            let d0 = Point2::new(0.0, 0.0);
            let d1 = Point2::new(0.0, 1.0);
            let m0 = Point2::new(1.0, 1.0);
            let m1 = Point2::new(2.0, 2.0);
            let sim = match_fiducial(d0, d1, m0, m1);
            println!("sim: {}", sim);

            println!("d0:  {}", d0);
            println!("d1:  {}", d1);
            println!("m0': {}", sim * m0);
            println!("m1': {}", sim * m1);

            assert_close(d0, sim * m0);
            assert_close(d1, sim * m1);
        }

        {
            let d0 = Point2::new(-1.0, -1.0);
            let d1 = Point2::new(0.0, 1.0);
            let m0 = Point2::new(1.0, 1.0);
            let m1 = Point2::new(2.0, 2.0);
            let sim = match_fiducial(d0, d1, m0, m1);

            assert_close(d0, sim * m0);
            assert_close(d1, sim * m1);
        }
    }
}
