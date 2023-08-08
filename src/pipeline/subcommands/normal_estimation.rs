use crate::formats::{
    pointxyzrgba::PointXyzRgba, pointxyzrgbanormal::PointXyzRgbaNormal, PointCloud,
};
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use clap::Parser;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use nalgebra::{Matrix3, Vector3};
use rayon::prelude::*;
use std::collections::VecDeque;
use std::time::Instant;

use super::Subcommand;

type PointType = [f64; 3];

#[derive(Parser)]
#[clap(about = "Performs normal estimation on point clouds.")]
pub struct Args {
    #[clap(short, long, default_value = "30")]
    k: usize,
}

pub struct NormalEstimation {
    args: Args,
}

impl NormalEstimation {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        Box::from(NormalEstimation {
            args: Args::parse_from(args),
        })
    }
}

impl Subcommand for NormalEstimation {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        // Perform normal estimation for each point cloud in the messages
        for message in messages {
            match message {
                PipelineMessage::IndexedPointCloud(pc, i) => {
                    let normal_estimation_result = perform_normal_estimation(&pc, self.args.k);
                    channel.send(PipelineMessage::IndexedPointCloudNormal(
                        normal_estimation_result,
                        i,
                    ));
                }
                PipelineMessage::Metrics(_)
                | PipelineMessage::IndexedPointCloudNormal(_, _)
                | PipelineMessage::DummyForIncrement
                | PipelineMessage::IndexedPointCloudWithTriangleFaces(_, _, _) => {}
                PipelineMessage::End => {
                    channel.send(message);
                }
            }
        }
    }
}

fn perform_normal_estimation(
    pc: &PointCloud<PointXyzRgba>,
    k: usize,
) -> PointCloud<PointXyzRgbaNormal> {
    let start_total = Instant::now(); // Record start time of the entire process

    // Select Neighboring Points
    let start_neighbors = Instant::now();
    let neighbors = select_neighbors(pc, k);
    let elapsed_neighbors = start_neighbors.elapsed();

    // Compute Covariance Matrix
    let start_covariance = Instant::now();
    let covariance_matrices = compute_covariance_matrices(&pc, &neighbors);
    let elapsed_covariance = start_covariance.elapsed();

    // Compute Eigenvalues and Eigenvectors
    let start_eigen = Instant::now();
    let eigen_results = compute_eigenvalues_eigenvectors(&covariance_matrices);
    let elapsed_eigen = start_eigen.elapsed();

    // Convert PointCloud<PointXyzRgba> to PointCloud<PointXyzRgbaNormal>
    let mut pc_normal: PointCloud<PointXyzRgbaNormal> = PointCloud {
        number_of_points: pc.number_of_points,
        points: pc
            .points
            .iter()
            .map(|p| {
                PointXyzRgbaNormal {
                    x: p.x,
                    y: p.y,
                    z: p.z,
                    r: p.r,
                    g: p.g,
                    b: p.b,
                    a: p.a,
                    nx: 0.0, // Uninitialized normal values
                    ny: 0.0,
                    nz: 0.0,
                }
            })
            .collect(),
    };

    // Assign Normal Vector
    let start_assign = Instant::now();
    assign_normal_vectors(&mut pc_normal, &eigen_results);
    let elapsed_assign = start_assign.elapsed();

    // Complete Normal Estimation
    let start_propagate = Instant::now();
    propagate_normal_orientation(&mut pc_normal, &neighbors);
    let elapsed_propagate = start_propagate.elapsed();

    let elapsed_total = start_total.elapsed(); // Record end time of the entire process

    // Output the runtime of each method
    println!("Select Neighbors: {:?}", elapsed_neighbors);
    println!("Compute Covariance Matrix: {:?}", elapsed_covariance);
    println!("Compute Eigenvalues and Eigenvectors: {:?}", elapsed_eigen);
    println!("Assign Normal Vector: {:?}", elapsed_assign);
    println!("Complete Normal Estimation: {:?}", elapsed_propagate);
    println!("Total Runtime: {:?}", elapsed_total);

    pc_normal
}

fn build_kd_tree(points: &[PointXyzRgba]) -> KdTree<f64, usize, PointType> {
    let mut kdtree = KdTree::new(3);
    for (i, point) in points.iter().enumerate() {
        kdtree
            .add([point.x as f64, point.y as f64, point.z as f64], i)
            .unwrap();
    }
    kdtree
}

fn select_neighbors(pc: &PointCloud<PointXyzRgba>, k: usize) -> Vec<Vec<usize>> {
    let kdtree = build_kd_tree(&pc.points);
    pc.points
        .par_iter() // Parallel iterator
        .enumerate()
        .map(|(i, point)| {
            // Ask for k+1 neighbors to account for the point itself
            let ret = kdtree
                .nearest(
                    &[point.x as f64, point.y as f64, point.z as f64],
                    k + 1,
                    &squared_euclidean,
                )
                .unwrap();
            let mut neighbor_indices = Vec::new();
            for &(_dist, &index) in ret.iter() {
                // Exclude the point itself
                if index != i {
                    neighbor_indices.push(index);
                }
            }
            neighbor_indices
        })
        .collect()
}

#[derive(Debug, PartialEq)]
pub struct CovarianceMatrix {
    xx: f32,
    xy: f32,
    xz: f32,
    yy: f32,
    yz: f32,
    zz: f32,
}

impl CovarianceMatrix {
    fn zeros() -> Self {
        CovarianceMatrix {
            xx: 0.0,
            xy: 0.0,
            xz: 0.0,
            yy: 0.0,
            yz: 0.0,
            zz: 0.0,
        }
    }
}

fn compute_covariance_matrices(
    pc: &PointCloud<PointXyzRgba>,
    neighbors: &[Vec<usize>],
) -> Vec<CovarianceMatrix> {
    let mut covariance_matrices = Vec::with_capacity(pc.number_of_points);

    for (i, point_neighbors) in neighbors.iter().enumerate() {
        let num_neighbors = point_neighbors.len();
        let total_points = num_neighbors + 1;

        if total_points < 3 {
            // Insufficient points to compute covariance matrix, set it as all zeros
            covariance_matrices.push(CovarianceMatrix::zeros());
            continue;
        }

        let mut mean_x = 0.0;
        let mut mean_y = 0.0;
        let mut mean_z = 0.0;

        for &neighbor_index in point_neighbors {
            mean_x += pc.points[neighbor_index].x;
            mean_y += pc.points[neighbor_index].y;
            mean_z += pc.points[neighbor_index].z;
        }

        // Include the point itself in the mean calculation
        mean_x += pc.points[i].x;
        mean_y += pc.points[i].y;
        mean_z += pc.points[i].z;

        mean_x /= total_points as f32;
        mean_y /= total_points as f32;
        mean_z /= total_points as f32;

        let mut cov_xx = 0.0;
        let mut cov_xy = 0.0;
        let mut cov_xz = 0.0;
        let mut cov_yy = 0.0;
        let mut cov_yz = 0.0;
        let mut cov_zz = 0.0;

        for &neighbor_index in point_neighbors {
            let neighbor = &pc.points[neighbor_index];
            let dx = neighbor.x - mean_x;
            let dy = neighbor.y - mean_y;
            let dz = neighbor.z - mean_z;

            cov_xx += dx * dx;
            cov_xy += dx * dy;
            cov_xz += dx * dz;
            cov_yy += dy * dy;
            cov_yz += dy * dz;
            cov_zz += dz * dz;
        }

        // Include the point itself in the covariance calculation
        let dx = pc.points[i].x - mean_x;
        let dy = pc.points[i].y - mean_y;
        let dz = pc.points[i].z - mean_z;

        cov_xx += dx * dx;
        cov_xy += dx * dy;
        cov_xz += dx * dz;
        cov_yy += dy * dy;
        cov_yz += dy * dz;
        cov_zz += dz * dz;

        let inv_num_neighbors = 1.0 / (total_points as f32);

        cov_xx *= inv_num_neighbors;
        cov_xy *= inv_num_neighbors;
        cov_xz *= inv_num_neighbors;
        cov_yy *= inv_num_neighbors;
        cov_yz *= inv_num_neighbors;
        cov_zz *= inv_num_neighbors;

        covariance_matrices.push(CovarianceMatrix {
            xx: cov_xx,
            xy: cov_xy,
            xz: cov_xz,
            yy: cov_yy,
            yz: cov_yz,
            zz: cov_zz,
        });
    }

    covariance_matrices
}

#[derive(Debug)]
struct EigenData {
    eigenvectors: Matrix3<f32>,
    eigenvalues: Vector3<f32>,
}

fn compute_eigenvalues_eigenvectors(covariance_matrices: &[CovarianceMatrix]) -> Vec<EigenData> {
    let mut eigen_data_vec = Vec::with_capacity(covariance_matrices.len());

    for covariance_matrix in covariance_matrices {
        let cov_matrix = Matrix3::new(
            covariance_matrix.xx,
            covariance_matrix.xy,
            covariance_matrix.xz,
            covariance_matrix.xy,
            covariance_matrix.yy,
            covariance_matrix.yz,
            covariance_matrix.xz,
            covariance_matrix.yz,
            covariance_matrix.zz,
        );

        let eigendecomp = cov_matrix.symmetric_eigen();

        let eigenvectors = eigendecomp.eigenvectors;
        let eigenvalues = eigendecomp.eigenvalues;

        let eigen_data = EigenData {
            eigenvectors,
            eigenvalues,
        };
        eigen_data_vec.push(eigen_data);
    }

    eigen_data_vec
}

fn assign_normal_vectors(pc: &mut PointCloud<PointXyzRgbaNormal>, eigen_results: &[EigenData]) {
    for (i, eigen_data) in eigen_results.iter().enumerate() {
        // Find the index of the smallest eigenvalue
        let min_index = eigen_data
            .eigenvalues
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(index, _value)| index)
            .unwrap_or(0); // If all else fails, default to 0

        // Select the eigenvector corresponding to the smallest eigenvalue
        let normal = eigen_data.eigenvectors.column(min_index).normalize();

        // Assign the normal vector to the point cloud
        pc.points[i].nx = normal[0];
        pc.points[i].ny = normal[1];
        pc.points[i].nz = normal[2];
    }
}

fn propagate_normal_orientation(pc: &mut PointCloud<PointXyzRgbaNormal>, neighbors: &[Vec<usize>]) {
    let root_point_index = 0; // Choose the root point index (e.g., 0)

    // Use a queue to perform a breadth-first search
    let mut queue = VecDeque::new();
    let mut visited = vec![false; pc.number_of_points];

    // Enqueue the root point
    queue.push_back(root_point_index);
    visited[root_point_index] = true;

    // Propagate normal orientation
    while let Some(current_point_index) = queue.pop_front() {
        let current_normal = Vector3::new(
            pc.points[current_point_index].nx,
            pc.points[current_point_index].ny,
            pc.points[current_point_index].nz,
        );

        // Check the orientation of neighbors and flip if necessary
        for &neighbor_index in &neighbors[current_point_index] {
            if !visited[neighbor_index] {
                let mut neighbor_normal = Vector3::new(
                    pc.points[neighbor_index].nx,
                    pc.points[neighbor_index].ny,
                    pc.points[neighbor_index].nz,
                );

                if current_normal.dot(&neighbor_normal) < 0.0 {
                    // Flip the neighbor's normal
                    neighbor_normal = -neighbor_normal;
                    pc.points[neighbor_index].nx = neighbor_normal[0];
                    pc.points[neighbor_index].ny = neighbor_normal[1];
                    pc.points[neighbor_index].nz = neighbor_normal[2];
                }

                // Enqueue the neighbor for further propagation
                queue.push_back(neighbor_index);
                visited[neighbor_index] = true;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;

    // #[test]
    // fn test_select_neighboring_points() {
    //     // Create a sample point cloud
    //     let points = vec![
    //         PointXyzRgba { x: 0.0, y: 0.0, z: 0.0, r: 0, g: 0, b: 0, a: 255 },
    //         PointXyzRgba { x: 1.0, y: 1.0, z: 1.0, r: 255, g: 255, b: 255, a: 255 },
    //         PointXyzRgba { x: 2.0, y: 2.0, z: 2.0, r: 255, g: 0, b: 0, a: 255 },
    //         PointXyzRgba { x: 3.0, y: 3.0, z: 3.0, r: 0, g: 255, b: 0, a: 255 },
    //         PointXyzRgba { x: 4.0, y: 4.0, z: 4.0, r: 0, g: 0, b: 255, a: 255 },
    //     ];

    //     let pc = PointCloud {
    //         number_of_points: points.len(),
    //         points,
    //     };

    //     let radius = 3.0; // Example radius value

    //     let neighbors = select_neighboring_points(&pc, radius);

    //     // Assert the expected neighbors for each point

    //     // Point 0 should have neighbors 1
    //     assert_eq!(neighbors[0], vec![1]);

    //     // Point 1 should have neighbors 0, 2
    //     assert_eq!(neighbors[1], vec![0, 2]);

    //     // Point 2 should have neighbors 1, 3
    //     assert_eq!(neighbors[2], vec![1, 3]);

    //     // Point 3 should have neighbors 2, 4
    //     assert_eq!(neighbors[3], vec![2, 4]);

    //     // Point 4 should have neighbors 3
    //     assert_eq!(neighbors[4], vec![3]);
    // }

    #[test]
    fn test_compute_eigenvalues_eigenvectors() {
        // Create a sample covariance matrix
        let covariance_matrix = CovarianceMatrix {
            xx: 2.0,
            xy: 1.0,
            xz: 1.0,
            yy: 3.0,
            yz: 2.0,
            zz: 4.0,
        };

        // Compute the eigen data
        let eigen_data = compute_eigenvalues_eigenvectors(&[covariance_matrix]);

        // Define the expected eigenvectors
        let expected_eigenvectors = Matrix3::new(
            0.52891886,
            -0.59959215,
            0.60068053,
            -0.5558934,
            0.23822187,
            0.79672605,
            0.6411168,
            0.7644144,
            0.068997495,
        );

        assert_relative_eq!(
            eigen_data[0].eigenvectors,
            expected_eigenvectors,
            epsilon = 1e-6
        );
    }
}
