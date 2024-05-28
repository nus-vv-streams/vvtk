use super::cd::Cd;
use crate::formats::pointxyzrgba::PointXyzRgba;
use float_ord::FloatOrd;
use kiddo::KdTree;
use num_traits::Float;
use rayon::prelude::*;
pub struct CdPsnr;

fn negative_squared_euclidean<T: Float, const K: usize>(a: &[T; K], b: &[T; K]) -> T {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| -((*x) - (*y)) * ((*x) - (*y)))
        .fold(T::zero(), ::std::ops::Add::add)
}

impl CdPsnr {
    pub fn calculate_from_acd_or_cd(
        acd_rt: Option<f64>,
        acd_tr: Option<f64>,
        cd: Option<f64>,
        original: &Vec<PointXyzRgba>,
        original_tree: &KdTree<f32, usize, 3>,
        reconstructed: &Vec<PointXyzRgba>,
        reconstructed_tree: &KdTree<f32, usize, 3>,
    ) -> Option<f64> {
        let cd = match (cd, acd_rt, acd_tr) {
            (Some(cd), _, _) => Some(cd),
            (_, Some(acd_rt), Some(acd_tr)) => Some((acd_rt + acd_tr) / 2.0),
            _ => Cd::calculate_metric(original, original_tree, reconstructed, reconstructed_tree)
                .into(),
        };

        // Mr is the maximal distance between any two points in Pr, here Pr is the original point cloud
        let mr = original
            .par_iter()
            .map(|pt| {
                let nearest_points = original_tree
                    .nearest(&[pt.x, pt.y, pt.z], 2, &negative_squared_euclidean)
                    .unwrap();
                let (dist, _) = nearest_points[0];
                FloatOrd(dist)
            })
            .min()
            .unwrap();

        let mr: f64 = mr.0.into();
        let mr: f64 = mr.abs();
        let cd_psnr = 10.0 * ((mr) / cd.unwrap()).log10();
        Some(cd_psnr)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use kiddo::{distance::squared_euclidean, KdTree};
    #[test]
    fn test_neg_squared_euclidean() {
        assert!(-2.0 == negative_squared_euclidean(&[0.0, 0.0], &[1.0, 1.0]));
    }

    #[test]
    fn test_all() {
        let mut original_tree = KdTree::new();
        original_tree.add(&[0.0, 0.0], 0).unwrap();
        original_tree.add(&[1.0, 1.0], 1).unwrap();
        original_tree.add(&[2.0, 2.0], 2).unwrap();
        original_tree.add(&[1.0, 2.0], 3).unwrap();
        original_tree.add(&[2.0, 1.0], 4).unwrap();
        original_tree.add(&[5.0, 5.0], 5).unwrap();
        original_tree.add(&[0.0, 1.0], 6).unwrap();

        let xs = original_tree
            .nearest(&[5.0, 4.0], 2, &squared_euclidean)
            .unwrap();
        let distance = xs[0].0;
        assert!(distance == 1.0);

        let xs = original_tree
            .nearest(&[0.0, 0.0], 2, &negative_squared_euclidean)
            .unwrap();
        let distance = xs[0].0;
        assert!(distance == -50.0);
    }
}
