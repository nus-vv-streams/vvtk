use clap::Parser;
use std::ops::Sub;
use nalgebra::Matrix3;
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use crate::formats::{PointCloud, pointxyzrgba::PointXyzRgba, pointxyzrgbanormal::PointXyzRgbaNormal};

use super::Subcommand;

#[derive(Parser)]
#[clap(
    about = "Performs normal estimation on point clouds.",
)]
pub struct Args {
    #[clap(short, long, default_value = "1.0")]
    radius: f64,
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
                    let normal_estimation_result = perform_normal_estimation(&pc, self.args.radius);
                    channel.send(PipelineMessage::IndexedPointCloudNormal(normal_estimation_result, i));
                }
                PipelineMessage::Metrics(_) | PipelineMessage::IndexedPointCloudNormal(_, _) | PipelineMessage::DummyForIncrement => {}
                PipelineMessage::End => {
                    channel.send(message);
                }
            }
        }
    }
}

fn perform_normal_estimation(pc: &PointCloud<PointXyzRgba>, radius: f64) -> PointCloud<PointXyzRgbaNormal> {
    // Select Neighboring Points
    let neighbors = select_neighboring_points(pc, radius);

    // Compute Covariance Matrix
    let covariance_matrices = compute_covariance_matrices(&pc, &neighbors);

    // Compute Eigenvalues and Eigenvectors
    let eigen_results = compute_eigenvalues_eigenvectors(&covariance_matrices);

    // Convert PointCloud<PointXyzRgba> to PointCloud<PointXyzRgbaNormal>
    let mut pc_normal: PointCloud<PointXyzRgbaNormal> = PointCloud {
        number_of_points: pc.number_of_points,
        points: pc.points.iter().map(|p| {
            PointXyzRgbaNormal {
                x: p.x,
                y: p.y,
                z: p.z,
                r: p.r,
                g: p.g,
                b: p.b,
                a: p.a,
                normal_x: 0.0, // Uninitialized normal values
                normal_y: 0.0,
                normal_z: 0.0,
            }
        }).collect(),
    };
    // Assign Normal Vector
    assign_normal_vectors(&mut pc_normal, &eigen_results);
    

    // // Complete Normal Estimation
    // let normal_estimation_result = complete_normal_estimation(&cleaned_cloud, &neighbors, &normals);

    // normal_estimation_result
    let point = PointXyzRgbaNormal {
        x: 1.0,
        y: 2.0,
        z: 3.0,
        r: 255,
        g: 0,
        b: 0,
        a: 255,
        normal_x: 0.0,
        normal_y: 0.0,
        normal_z: 1.0,
    };
    let point_cloud = PointCloud {
        number_of_points: 1,
        points: vec![point],
    };
    point_cloud
}

fn select_neighboring_points(pc: &PointCloud<PointXyzRgba>, radius: f64) -> Vec<Vec<usize>> {
    let mut neighbors: Vec<Vec<usize>> = vec![Vec::new(); pc.number_of_points];

    for i in 0..pc.number_of_points {
        let mut point_neighbors: Vec<usize> = Vec::new();
        let p1 = &pc.points[i];

        for j in 0..pc.number_of_points {
            if i != j {
                let p2 = &pc.points[j];
                let dist = distance(&[p1.x, p1.y, p1.z], &[p2.x, p2.y, p2.z]);

                if dist <= radius {
                    point_neighbors.push(j);
                }
            }
        }

        neighbors[i] = point_neighbors;
    }

    neighbors
}


fn distance<T>(p1: &[T; 3], p2: &[T; 3]) -> f64
where
    T: Sub<Output = T> + Into<f64> + Copy,
{
    let dx = (p1[0] - p2[0]).into();
    let dy = (p1[1] - p2[1]).into();
    let dz = (p1[2] - p2[2]).into();

    (dx * dx + dy * dy + dz * dz).sqrt()
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

fn compute_covariance_matrices(pc: &PointCloud<PointXyzRgba>, neighbors: &[Vec<usize>]) -> Vec<CovarianceMatrix> {
    let mut covariance_matrices = Vec::with_capacity(pc.number_of_points);

    for (i, point_neighbors) in neighbors.iter().enumerate() {
        let num_neighbors = point_neighbors.len();
        let total_points = num_neighbors + 1;

        if total_points < 3 {
            // Insufficient points to compute covariance matrix, set it as all zeros
            covariance_matrices.push(CovarianceMatrix::zeros());
            continue;
        }

        let mut mean_x = pc.points[i].x;
        let mut mean_y = pc.points[i].y;
        let mut mean_z = pc.points[i].z;

        for &neighbor_index in point_neighbors {
            mean_x += pc.points[neighbor_index].x;
            mean_y += pc.points[neighbor_index].y;
            mean_z += pc.points[neighbor_index].z;
        }

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

        // Include the point itself
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
}

fn compute_eigenvalues_eigenvectors(covariance_matrices: &[CovarianceMatrix]) -> Vec<EigenData> {
    let mut eigen_data_vec = Vec::with_capacity(covariance_matrices.len());

    for covariance_matrix in covariance_matrices {
        let cov_matrix = Matrix3::new(
            covariance_matrix.xx, covariance_matrix.xy, covariance_matrix.xz,
            covariance_matrix.xy, covariance_matrix.yy, covariance_matrix.yz,
            covariance_matrix.xz, covariance_matrix.yz, covariance_matrix.zz,
        );

        let eigendecomp = cov_matrix.symmetric_eigen();

        let eigenvectors = eigendecomp.eigenvectors;

        let eigen_data = EigenData { eigenvectors };
        eigen_data_vec.push(eigen_data);
    }

    eigen_data_vec
}


fn assign_normal_vectors(pc: &mut PointCloud<PointXyzRgbaNormal>, eigen_results: &[EigenData]) {
    for (i, eigen_data) in eigen_results.iter().enumerate() {
        let normal = eigen_data.eigenvectors.column(0).normalize(); // Select the first eigenvector
        pc.points[i].normal_x = normal[0];
        pc.points[i].normal_y = normal[1];
        pc.points[i].normal_z = normal[2];
    }
}


// fn complete_normal_estimation(
//     pc: &PointCloud<PointXyzRgba>,
//     neighbors: &[Vec<usize>],
//     normals: &[NormalVector],
// ) -> PointCloud<NormalVector> {
//     // After traversing all points in the point cloud and propagating the orientations,
//     // you will have estimated a normal vector for each point with orientations consistent across the entire point cloud
//     // Return the completed normal estimation as a new point cloud
// }

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_select_neighboring_points() {
        // Create a sample point cloud
        let points = vec![
            PointXyzRgba { x: 0.0, y: 0.0, z: 0.0, r: 0, g: 0, b: 0, a: 255 },
            PointXyzRgba { x: 1.0, y: 1.0, z: 1.0, r: 255, g: 255, b: 255, a: 255 },
            PointXyzRgba { x: 2.0, y: 2.0, z: 2.0, r: 255, g: 0, b: 0, a: 255 },
            PointXyzRgba { x: 3.0, y: 3.0, z: 3.0, r: 0, g: 255, b: 0, a: 255 },
            PointXyzRgba { x: 4.0, y: 4.0, z: 4.0, r: 0, g: 0, b: 255, a: 255 },
        ];
    
        let pc = PointCloud {
            number_of_points: points.len(),
            points,
        };
    
        let radius = 3.0; // Example radius value
    
        let neighbors = select_neighboring_points(&pc, radius);
    
        // Assert the expected neighbors for each point
    
        // Point 0 should have neighbors 1
        assert_eq!(neighbors[0], vec![1]);
    
        // Point 1 should have neighbors 0, 2
        assert_eq!(neighbors[1], vec![0, 2]);
    
        // Point 2 should have neighbors 1, 3
        assert_eq!(neighbors[2], vec![1, 3]);
    
        // Point 3 should have neighbors 2, 4
        assert_eq!(neighbors[3], vec![2, 4]);
    
        // Point 4 should have neighbors 3
        assert_eq!(neighbors[4], vec![3]);
    }

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
            0.52891886, -0.59959215, 0.60068053,
            -0.5558934, 0.23822187, 0.79672605,
            0.6411168, 0.7644144, 0.068997495,
        );

        assert_relative_eq!(eigen_data[0].eigenvectors, expected_eigenvectors, epsilon = 1e-6);
    }
    
    
}

