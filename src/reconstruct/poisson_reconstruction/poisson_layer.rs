use super::conjugate_gradient::solve_conjugate_gradient;
use super::hgrid::HGrid;
use super::kdtree_data::KdTreeData;
use super::poisson_vector_field::PoissonVectorField;
use super::polynomial::TriQuadraticBspline;
use super::{
    poisson::{self, CellWithId},
    polynomial, Real,
};
use crate::formats::pointxyzrgbanormal::PointXyzRgbaNormal;
use kiddo::KdTree;
use nalgebra::{vector, DVector, Point3, Vector3};
use nalgebra_sparse::{CooMatrix, CscMatrix};
use parry3d_f64::bounding_volume::Aabb;
use parry3d_f64::partitioning::Qbvh;
use std::collections::HashMap;
#[derive(Clone)]
pub struct PoissonLayer {
    pub grid: HGrid<usize>,
    pub cells_qbvh: Qbvh<CellWithId>,
    pub grid_node_idx: HashMap<Point3<i64>, usize>,
    pub ordered_nodes: Vec<Point3<i64>>,
    pub node_weights: DVector<Real>,
    pub kd_tree: Option<KdTree<f64, KdTreeData, 3>>,
}

impl PoissonLayer {
    pub fn cell_width(&self) -> Real {
        self.grid.cell_width()
    }
}

impl PoissonLayer {
    pub fn from_points(
        vertices: &[PointXyzRgbaNormal],
        grid_origin: Point3<Real>,
        cell_width: Real,
        with_colour: bool,
    ) -> Self {
        let mut grid = HGrid::new(grid_origin, cell_width);
        let mut grid_node_idx = HashMap::new();
        let mut ordered_nodes = vec![];

        // for pt in points {
        //     let ref_node = grid.key(pt);
        //
        //     for corner_shift in CORNERS.iter() {
        //         let node = ref_node + corner_shift;
        //         let _ = grid_node_idx.entry(node).or_insert_with(|| {
        //             let center = grid.cell_center(&node);
        //             grid.insert(&center, 0);
        //             ordered_nodes.push(node);
        //             ordered_nodes.len() - 1
        //         });
        //     }
        // }

        // TODO: do we still need this when using the multigrid solver?
        let mut kd_tree: Option<KdTree<f64, KdTreeData, 3>> = if with_colour {
            Some(KdTree::new())
        } else {
            None
        };

        for (pid, vertice) in vertices.iter().enumerate() {
            let pt: Point3<Real> =
                Point3::new(vertice.x as f64, vertice.y as f64, vertice.z as f64);
            let ref_node = grid.key(&pt);
            let ref_center = grid.cell_center(&ref_node);
            grid.insert(&ref_center, pid);
            grid.update_cell_average(&pt);

            for i in -2..=2 {
                for j in -2..=2 {
                    for k in -2..=2 {
                        let node = ref_node + vector![i, j, k];
                        let center = grid.cell_center(&node);
                        let _ = grid_node_idx.entry(node).or_insert_with(|| {
                            grid.insert(&center, usize::MAX);
                            ordered_nodes.push(node);
                            ordered_nodes.len() - 1
                        });
                    }
                }
            }

            if let Some(tree) = kd_tree.as_mut() {
                let formatted_point: [f64; 3] = [pt.x, pt.y, pt.z];
                let data = KdTreeData {
                    index: pid,
                    color: [vertice.r, vertice.g, vertice.b, vertice.a],
                };
                let _ = tree.add(&formatted_point, data);
            }
        }

        Self::from_populated_grid(grid, grid_node_idx, ordered_nodes, kd_tree)
    }

    pub fn from_next_layer(points: &[Point3<Real>], layer: &Self) -> Self {
        let cell_width = layer.cell_width() * 2.0;
        let mut grid = HGrid::new(*layer.grid.origin(), cell_width);
        let mut grid_node_idx = HashMap::new();
        let mut ordered_nodes = vec![];

        // Add nodes to the new grid to form a comforming "octree".
        for sub_node_key in &layer.ordered_nodes {
            let pt = layer.grid.cell_center(sub_node_key);
            let my_key = grid.key(&pt);
            let my_center = grid.cell_center(&my_key);
            let quadrant = pt - my_center;

            let range = |x| {
                if x < 0.0 {
                    -2..=1
                } else {
                    -1..=2
                }
            };

            for i in range(quadrant.x) {
                for j in range(quadrant.y) {
                    for k in range(quadrant.z) {
                        let adj_key = my_key + vector![i, j, k];

                        let _ = grid_node_idx.entry(adj_key).or_insert_with(|| {
                            let adj_center = grid.cell_center(&adj_key);
                            grid.insert(&adj_center, usize::MAX);
                            ordered_nodes.push(adj_key);
                            ordered_nodes.len() - 1
                        });
                    }
                }
            }
        }

        for (pid, pt) in points.iter().enumerate() {
            let ref_node = grid.key(pt);
            let ref_center = grid.cell_center(&ref_node);
            grid.insert(&ref_center, pid);
            grid.update_cell_average(&pt);
        }

        Self::from_populated_grid(grid, grid_node_idx, ordered_nodes, None)
    }

    fn from_populated_grid(
        grid: HGrid<usize>,
        grid_node_idx: HashMap<Point3<i64>, usize>,
        ordered_nodes: Vec<Point3<i64>>,
        kd_tree: Option<KdTree<f64, KdTreeData, 3>>,
    ) -> Self {
        let cell_width = grid.cell_width();
        let mut cells_qbvh = Qbvh::new();
        cells_qbvh.clear_and_rebuild(
            ordered_nodes.iter().map(|key| {
                let center = grid.cell_center(key);
                let id = grid_node_idx[key];
                let half_width = Vector3::repeat(cell_width / 2.0);
                (
                    CellWithId { cell: *key, id },
                    Aabb::from_half_extents(center, half_width),
                )
            }),
            0.0,
        );

        let node_weights = DVector::zeros(grid_node_idx.len());

        Self {
            grid,
            cells_qbvh,
            ordered_nodes,
            grid_node_idx,
            node_weights,
            kd_tree,
        }
    }

    pub(crate) fn solve(
        layers: &[Self],
        curr_layer: usize,
        vector_field: &PoissonVectorField,
        points: &[Point3<Real>],
        normals: &[Vector3<Real>],
        screening: Real,
        niters: usize,
    ) -> DVector<Real> {
        let my_layer = &layers[curr_layer];
        let cell_width = my_layer.cell_width();
        assert_eq!(points.len(), normals.len());
        let convolution = polynomial::compute_quadratic_bspline_convolution_coeffs(cell_width);
        let num_nodes = my_layer.ordered_nodes.len();

        // Compute the gradient matrix.
        let mut grad_matrix = CooMatrix::new(num_nodes, num_nodes);
        let screen_factor =
            (2.0 as Real).powi(curr_layer as i32) * screening * vector_field.area_approximation()
                / (points.len() as Real);

        for (nid, node) in my_layer.ordered_nodes.iter().enumerate() {
            let center1 = my_layer.grid.cell_center(node);

            for i in -2..=2 {
                for j in -2..=2 {
                    for k in -2..=2 {
                        let other_node = node + vector![i, j, k];
                        let center2 = my_layer.grid.cell_center(&other_node);

                        if let Some(other_nid) = my_layer.grid_node_idx.get(&other_node) {
                            let ii = (i + 2) as usize;
                            let jj = (j + 2) as usize;
                            let kk = (k + 2) as usize;

                            let mut laplacian = convolution.laplacian[ii][jj][kk];

                            if screening != 0.0 {
                                for si in -1..=1 {
                                    for sj in -1..=1 {
                                        for sk in -1..=1 {
                                            let adj = node + vector![si, sj, sk];

                                            if let Some(pt_ids) = my_layer.grid.cell(&adj) {
                                                if pt_ids.len() > 1 {
                                                    let poly1 = TriQuadraticBspline::new(
                                                        center1, cell_width,
                                                    );
                                                    let poly2 = TriQuadraticBspline::new(
                                                        center2, cell_width,
                                                    );
                                                    let pt =
                                                        my_layer.grid.get_cell_average_point(&adj);
                                                    if pt != Point3::new(0.0, 0.0, 0.0) {
                                                        laplacian += screen_factor
                                                            * poly1.eval(pt)
                                                            * poly2.eval(pt)
                                                            * (pt_ids.len() as f64 - 1.0);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            grad_matrix.push(nid, *other_nid, laplacian);
                        }
                    }
                }
            }
        }

        // Build rhs
        let mut rhs = DVector::zeros(my_layer.ordered_nodes.len());
        vector_field.build_rhs(
            layers,
            curr_layer,
            &mut rhs,
            screening != 0.0,
            screen_factor,
        );

        // Solve the sparse system.
        let lhs = CscMatrix::from(&grad_matrix);
        solve_conjugate_gradient(&lhs, &mut rhs, niters);

        rhs
    }

    pub fn eval_triquadratic(&self, pt: &Point3<Real>) -> Real {
        poisson::eval_triquadratic(
            pt,
            &self.grid,
            &self.grid_node_idx,
            self.node_weights.as_slice(),
        )
    }

    pub fn eval_triquadratic_gradient(&self, pt: &Point3<Real>) -> Vector3<Real> {
        poisson::eval_triquadratic_gradient(
            pt,
            &self.grid,
            &self.grid_node_idx,
            self.node_weights.as_slice(),
        )
    }
}
