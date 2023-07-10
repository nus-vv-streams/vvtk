use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};
use octree::Octree;
use poisson_reconstruction::{PoissonReconstruction, Real};
use nalgebra::{Point3, Vector3};

const BUCKET_SIZE: usize = 10000;

pub fn reconstruct(
    points: PointCloud<PointXyzRgba>,
) -> PointCloud<PointXyzRgba> {
    // let tmp = points.clone();
    // set octree
    // let _octree = create_octree(points);
    
    // compute vector field

    // compute indicator function

    //extract iso-surface
    let surface: Vec<Point3<Real>> = reconstruct_surface(&points.points);
    let vecPoints: Vec<PointXyzRgba> = surface
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
    println!("Length of vecPoints: {}", vecPoints.len());
    PointCloud::<PointXyzRgba> { number_of_points: vecPoints.len(), points: vecPoints }
    //points
}

pub fn reconstruct_surface(vertices: &[PointXyzRgba]) -> Vec<Point3<Real>> {
    let points: Vec<_> = vertices.iter().map(|v| Point3::new(v.x as f64, v.y as f64, v.z as f64)).collect();
    let normals: Vec<_> = vertices.iter().map(|v| Vector3::new(v.nx as f64, v.ny as f64, v.nz as f64)).collect();

    let poisson: PoissonReconstruction = PoissonReconstruction::from_points_and_normals(points.as_slice(), normals.as_slice(), 0.0, 6, 6, 10);
    poisson.reconstruct_mesh()
}

pub fn create_octree(point_cloud: PointCloud<PointXyzRgba>) -> Octree {
    let points_iter: Vec<[f64; 3]> = point_cloud.points
    .iter()
    .map(|point_xyzrgba| {
        let x = f64::from(point_xyzrgba.x);
        let y = f64::from(point_xyzrgba.y);
        let z = f64::from(point_xyzrgba.z);
        [x, y, z]
    })
    .collect();
    let mut _octree = Octree::new(points_iter);
    _octree.build(BUCKET_SIZE);
    _octree
}