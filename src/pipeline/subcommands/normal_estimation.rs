use clap::Parser;
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use crate::formats::{PointCloud, pointxyzrgba::PointXyzRgba, pointxyzrgbanormal::PointXyzRgbaNormal};

use super::Subcommand;

#[derive(Parser)]
#[clap(
    about = "Performs normal estimation on point clouds.",
)]
pub struct Args {
    //TODO: Add any necessary arguments for the normal estimation
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
                    let normal_estimation_result = perform_normal_estimation(&pc);
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

fn perform_normal_estimation(pc: &PointCloud<PointXyzRgba>) -> PointCloud<PointXyzRgbaNormal> {
    // // Prepare the Point Cloud
    // let cleaned_cloud = prepare_point_cloud(pc);

    // // Select Neighboring Points
    // let neighbors = select_neighboring_points(&cleaned_cloud);

    // // Compute Covariance Matrix
    // let covariance_matrices = compute_covariance_matrices(&cleaned_cloud, &neighbors);

    // // Compute Eigenvalues and Eigenvectors
    // let eigen_results = compute_eigenvalues_and_eigenvectors(&covariance_matrices);

    // // Assign Normal Vector
    // let normals = assign_normal_vectors(&eigen_results);

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

// fn prepare_point_cloud(pc: &PointCloud<PointXyzRgba>) -> PointCloud<PointXyzRgba> {
//     // Perform any cleaning, denoising, or downsampling steps here
//     // Return the prepared point cloud
// }

// fn select_neighboring_points(pc: &PointCloud<PointXyzRgba>) -> Vec<Vec<usize>> {
//     // Select neighboring points for each point in the point cloud
//     // This could be done using radius search or k-nearest neighbors
//     // Return a vector containing the indices of neighboring points for each point
// }

// fn compute_covariance_matrices(pc: &PointCloud<PointXyzRgba>, neighbors: &[Vec<usize>]) -> Vec<CovarianceMatrix> {
//     // Compute the covariance matrix for each point and its neighbors
//     // Return a vector containing the covariance matrices
// }

// fn compute_eigenvalues_and_eigenvectors(covariance_matrices: &[CovarianceMatrix]) -> Vec<EigenResult> {
//     // Compute the eigenvalues and eigenvectors for each covariance matrix
//     // Return a vector containing the eigenvalue and eigenvector results
// }

// fn assign_normal_vectors(eigen_results: &[EigenResult]) -> Vec<NormalVector> {
//     // Assign the normal vector for each point based on the eigenvector corresponding to the smallest eigenvalue
//     // The normal vector can be derived from the eigenvector
//     // Return a vector containing the assigned normal vectors
// }

// fn complete_normal_estimation(
//     pc: &PointCloud<PointXyzRgba>,
//     neighbors: &[Vec<usize>],
//     normals: &[NormalVector],
// ) -> PointCloud<NormalVector> {
//     // After traversing all points in the point cloud and propagating the orientations,
//     // you will have estimated a normal vector for each point with orientations consistent across the entire point cloud
//     // Return the completed normal estimation as a new point cloud
// }
