use crate::formats::{
    pointxyzrgba::PointXyzRgba, pointxyzrgbanormal::PointXyzRgbaNormal,
    triangle_face::TriangleFace, PointCloud,
};
use crate::reconstruct::poisson_reconstruction::poisson::PoissonReconstruction;
use crate::reconstruct::poisson_reconstruction::Real;
use nalgebra::Vector3;

pub fn reconstruct(
    points: PointCloud<PointXyzRgbaNormal>,
    screening: f64,
    density_estimation_depth: usize,
    max_depth: usize,
    max_relaxation_iters: usize,
    with_colour: bool,
    with_faces: bool,
) -> (PointCloud<PointXyzRgba>, Option<Vec<TriangleFace>>) {
    let surface: Vec<PointXyzRgba> = reconstruct_surface(
        &points.points,
        screening,
        density_estimation_depth,
        max_depth,
        max_relaxation_iters,
        with_colour,
    );

    let num_of_points = surface.len();
    let mut triangle_faces: Option<Vec<TriangleFace>> = None;

    if with_faces {
        triangle_faces = Some(TriangleFace::get_default_mesh(num_of_points as i32));
    }
    (
        PointCloud::<PointXyzRgba> {
            number_of_points: num_of_points,
            points: surface,
        },
        triangle_faces,
    )
    //points
}

pub fn reconstruct_surface(
    vertices: &[PointXyzRgbaNormal],
    screening: f64,
    density_estimation_depth: usize,
    max_depth: usize,
    max_relaxation_iters: usize,
    with_colour: bool,
) -> Vec<PointXyzRgba> {
    let normals: Vec<_> = vertices
        .iter()
        .map(|v| Vector3::new(v.nx as f64, v.ny as f64, v.nz as f64))
        .collect();
    let poisson: PoissonReconstruction = PoissonReconstruction::from_points_and_normals(
        vertices,
        normals.as_slice(),
        screening as Real,
        density_estimation_depth,
        max_depth,
        max_relaxation_iters,
        with_colour,
    );
    poisson.reconstruct_mesh()
}
