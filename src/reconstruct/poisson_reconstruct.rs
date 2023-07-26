use crate::formats::{pointxyzrgba::PointXyzRgba, triangle_face::TriangleFace, PointCloud};
use crate::reconstruct::poisson_reconstruction::poisson::PoissonReconstruction;
use crate::reconstruct::poisson_reconstruction::Real;
use nalgebra::{Point3, Vector3};

pub fn reconstruct(
    points: PointCloud<PointXyzRgba>,
) -> (PointCloud<PointXyzRgba>, Vec<TriangleFace>) {
    let surface: Vec<Point3<Real>> = reconstruct_surface(&points.points);
    let vec_points: Vec<PointXyzRgba> = surface
        .iter()
        .map(|p| PointXyzRgba {
            x: p.x as f32,
            y: p.y as f32,
            z: p.z as f32,
            nx: 0.0,
            ny: 0.0,
            nz: 0.0,
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        })
        .collect();
    let num_of_points = vec_points.len();
    println!("Length of reconstructed vertices: {}", num_of_points);
    (
        PointCloud::<PointXyzRgba> {
            number_of_points: num_of_points,
            points: vec_points,
        },
        TriangleFace::get_default_mesh(num_of_points as i32),
    )
    //points
}

pub fn reconstruct_surface(vertices: &[PointXyzRgba]) -> Vec<Point3<Real>> {
    let points: Vec<_> = vertices
        .iter()
        .map(|v| Point3::new(v.x as f64, v.y as f64, v.z as f64))
        .collect();
    let normals: Vec<_> = vertices
        .iter()
        .map(|v| Vector3::new(v.nx as f64, v.ny as f64, v.nz as f64))
        .collect();
    let poisson: PoissonReconstruction = PoissonReconstruction::from_points_and_normals(
        points.as_slice(),
        normals.as_slice(),
        0.5,
        6,
        6,
        10,
    );
    poisson.reconstruct_mesh()
}
