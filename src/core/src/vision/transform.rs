use super::Point;
use nalgebra::{geometry::Translation2, norm, MatrixN, Projective2, Similarity2, U3, UnitComplex};

pub struct GridTransformer {
    similarity: Similarity2<f32>,
    projection: Projective2<f32>,
}

impl GridTransformer {
    pub fn from_points(image_pts: &[Point], coord_pts: &[Point]) -> GridTransformer {
        let similarity = {
            let i = 0;
            let j = 2;
            mk_similarity(coord_pts[i], coord_pts[j], image_pts[i], image_pts[j])
        };
        let sim_image_pts: Vec<_> = image_pts.iter().map(|p| similarity * p).collect();
        let projection = mk_projective(&sim_image_pts, coord_pts);
        GridTransformer {
            similarity,
            projection,
        }
    }

    pub fn transform(&self, image_pt: &Point) -> Point {
        self.projection * (self.similarity * image_pt)
    }
}

impl Default for GridTransformer {
    fn default() -> Self {
        let image_pts = &[
            Point::new(91.0, 89.0),
            Point::new(48.0, 269.0),
            Point::new(152.0, 271.0),
            Point::new(199.0, 86.0),
        ];
        let coord_pts = &[
            Point::new(1.0, 7.0),
            Point::new(13.0, 10.0),
            Point::new(13.0, 3.0),
            Point::new(1.0, 0.0),
        ];
        GridTransformer::from_points(image_pts, coord_pts)
    }
}

/// d0: the desired first fiducial coordinate
/// d1: the desired second fiducial coordinate
/// m0: the measured first fiducial coordinate
/// m1: the measured second fiducial coordinate
fn mk_similarity(d0: Point, d1: Point, m0: Point, m1: Point) -> Similarity2<f32> {
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

fn mk_projective(image_pts: &[Point], coord_pts: &[Point]) -> Projective2<f32> {
    // from https://math.stackexchange.com/questions/296794/

    assert_eq!(image_pts.len(), 4);
    assert_eq!(coord_pts.len(), 4);

    let a = basis_mapping(image_pts);
    let b = basis_mapping(coord_pts);
    let a_inv = a.try_inverse().expect("image matrix was not invertible!");

    let mat = b * a_inv;
    Projective2::from_matrix_unchecked(mat)
}

fn basis_mapping(pts: &[Point]) -> MatrixN<f32, U3> {
    assert_eq!(pts.len(), 4);

    let (last, first3) = pts.split_last().unwrap();
    let homogeneous: Vec<_> = first3.iter().map(|p| p.to_homogeneous()).collect();
    let mut mat = MatrixN::<f32, U3>::from_columns(&homogeneous);
    let x = mat.qr().solve(&last.to_homogeneous()).unwrap();

    trace!("mat: {:#?}", mat);
    trace!("x: {:#?}", x);

    for j in 0..3 {
        for i in 0..3 {
            mat[(i, j)] *= x[j]
        }
    }
    trace!("mat': {}", mat);

    mat
}

#[cfg(test)]
mod tests {

    use super::*;

    use nalgebra::{
        base::Unit, geometry::Translation2, norm, Point2, Similarity2, UnitComplex, Vector2,
    };

    fn assert_close(p0: Point, p1: Point, epsilon: f32) {
        let diff = p0 - p1;
        let dist = norm(&diff);
        if dist > epsilon {
            panic!("{} and {} too far: {}", p0, p1, dist)
        }
    }

    #[test]
    fn test_similiarity() {
        let epsilon = 0.0001;
        {
            let d0 = Point::new(0.0, 0.0);
            let d1 = Point::new(0.0, 1.0);
            let m0 = Point::new(1.0, 1.0);
            let m1 = Point::new(2.0, 2.0);
            let sim = mk_similarity(d0, d1, m0, m1);
            trace!("sim: {}", sim);

            trace!("d0:  {}", d0);
            trace!("d1:  {}", d1);
            trace!("m0': {}", sim * m0);
            trace!("m1': {}", sim * m1);

            assert_close(d0, sim * m0, epsilon);
            assert_close(d1, sim * m1, epsilon);
        }

        {
            let d0 = Point::new(-1.0, -1.0);
            let d1 = Point::new(0.0, 1.0);
            let m0 = Point::new(1.0, 1.0);
            let m1 = Point::new(2.0, 2.0);
            let sim = mk_similarity(d0, d1, m0, m1);

            assert_close(d0, sim * m0, epsilon);
            assert_close(d1, sim * m1, epsilon);
        }
    }

    #[test]
    fn test_transformer() {
        let image_pts = &[
            Point::new(91.0, 89.0),
            Point::new(48.0, 269.0),
            Point::new(152.0, 271.0),
            Point::new(199.0, 86.0),
        ];
        let coord_pts = &[
            Point::new(1.0, 7.0),
            Point::new(13.0, 10.0),
            Point::new(13.0, 3.0),
            Point::new(1.0, 0.0),
        ];

        let tf = GridTransformer::from_points(image_pts, coord_pts);

        let epsilon = 0.015;

        for i in 0..4 {
            let coord_tf = tf.transform(&image_pts[i]);
            debug!("image_pt[{}]: {}", i, coord_tf);
            assert_close(coord_tf, coord_pts[i], epsilon);
        }
    }
}
