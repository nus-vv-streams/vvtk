use cgmath::{Matrix4, Point3, Transform};
use kiddo::{distance::squared_euclidean, KdTree};
use num_traits::Float;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{formats::{bounds::Bounds, pointxyzrgba::PointXyzRgba, PointCloud}, utils::get_pc_bound};
use std::{collections::{BTreeSet, HashSet}, time::Instant};

use super::{camera::CameraState, renderable::Renderable, resolution_controller::ResolutionController};

pub struct Upsampler {
    
}

const VIEWPORT_DIST_UPSAMPLING_THRESHOLD: f32 = 3.0;

impl Upsampler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn should_upsample(&self, point_cloud: &PointCloud<PointXyzRgba>, camera_state: &CameraState) -> bool {
        point_cloud.points.len() < 100_000
        // /*
        // 1. Get points in NDC
        // 2. Calculate the average distance normalised by viewport
        // 3. If greater than **threshold**, upsample
        //  */
        // let start = Instant::now();
        // let point_num = point_cloud.points.len();
        // if point_num == 0 || point_num > 100_000 {
        //     return false
        // }
        // let view_proj_matrix = Matrix4::from(camera_state.camera_uniform.view_proj);
        // let antialias = point_cloud.antialias();
        // let width = camera_state.get_window_size().width;
        // let height = camera_state.get_window_size().height;
        // let points_viewport = point_cloud.points.par_iter().map(|point| {
        //     let point_vec = Point3::new(point.x - antialias.x, point.y - antialias.y, point.z - antialias.z) / antialias.scale;
        //     let point_ndc = view_proj_matrix.transform_point(point_vec);
        //     let x = (point_ndc.x * (width as f32)) as i32;
        //     let y = (point_ndc.y * (height as f32))  as i32;
        //     (x, y)
        // }).collect::<BTreeSet<_>>().par_iter().map(|coords| {
        //     PointXyzRgba {
        //         x: coords.0 as f32,
        //         y: coords.1 as f32,
        //         z: 0 as f32,
        //         r: 0,
        //         g: 0,
        //         b: 0,
        //         a: 0,
        //     }
        // }).collect::<Vec<_>>();

        // let average_spacing = Self::calculate_spacing(&points_viewport);
        // println!("{:?}", average_spacing);
        // println!("Time taken {:?}", start.elapsed());
        // return average_spacing > VIEWPORT_DIST_UPSAMPLING_THRESHOLD
    }

    fn calculate_spacing(points: &Vec<PointXyzRgba>) -> f32 {
        let mut tree = KdTree::new();
        for (i, p) in points.iter().enumerate() {
            tree.add(&[p.x, p.y, p.z], i).unwrap();
        }

        let mut sum = 0.0;
        // The value is currently hard coded. Can potentially be improved with variance
        let k_nearest = 4;

        for p in points.iter() {
            let avg_spacing = tree
                .nearest(&[p.x, p.y, p.z], k_nearest, &squared_euclidean)
                .unwrap()
                .iter()
                .skip(1) // ignore the first point (same point)
                .map(|(d, _)| d.sqrt())
                .sum::<f32>()
                / (k_nearest - 1) as f32;

            sum += avg_spacing;
        }

        sum / points.len() as f32
    }

    pub fn contains(bound: &Bounds, point: &PointXyzRgba) -> bool {
        const ERROR_MARGIN_PERCENTAGE: f32 = 1.01;
        point.x * ERROR_MARGIN_PERCENTAGE >= bound.min_x 
            && point.x <= bound.max_x * ERROR_MARGIN_PERCENTAGE
            && point.y >= bound.min_y
            && point.y <= bound.max_y * ERROR_MARGIN_PERCENTAGE
            && point.z >= bound.min_z
            && point.z <= bound.max_z * ERROR_MARGIN_PERCENTAGE
    }
    
    fn partition(
        pc: &PointCloud<PointXyzRgba>,
        partitions: (usize, usize, usize),
    ) -> Vec<Vec<PointXyzRgba>> {
        let pc_bound = get_pc_bound(&pc);
        let child_bounds = pc_bound.partition(partitions);
    
        child_bounds.par_iter().map(|bound| {
            pc.points.iter().map(|point| point.clone()).filter(|point| Self::contains(bound, point)).collect::<Vec<_>>()
        }).collect::<Vec<_>>()
    }
    
    
    fn euclidean_distance_3d(point1: &PointXyzRgba, point2: &PointXyzRgba) -> f32 {
        let dx = point1.x - point2.x;
        let dy = point1.y - point2.y;
        let dz = point1.z - point2.z;
        (dx.powi(2) + dy.powi(2) + dz.powi(2)).sqrt()
    }
    
    fn get_middlepoint(point1: &PointXyzRgba, point2: &PointXyzRgba) -> PointXyzRgba {
        let geom_x = ((point1.x as f32) + (point2.x as f32)) / 2.0;
        let geom_y = ((point1.y as f32) + (point2.y as f32)) / 2.0;
        let geom_z = ((point1.z as f32) + (point2.z as f32)) / 2.0;
    
        let col_r = ((point1.r as f32) + (point2.r as f32)) / 2.0;
        let col_g = ((point1.g as f32) + (point2.g as f32)) / 2.0;
        let col_b = ((point1.b as f32) + (point2.b as f32)) / 2.0;
        let col_a = ((point1.a as f32) + (point2.a as f32)) / 2.0;
        PointXyzRgba {
            x: geom_x,
            y: geom_y,
            z: geom_z,
            r: col_r as u8,
            g: col_g as u8,
            b: col_b as u8,
            a: col_a as u8,
        }
    }
    
    fn get_circumference_order(neighbours: &Vec<usize>, points: &Vec<PointXyzRgba>) -> Vec<usize> {
        let mut curr = neighbours[0]; // Assuming this is valid
        let mut order = vec![curr];
        let mut seen = HashSet::new();
        seen.insert(curr);
        
        while order.len() < neighbours.len() {
            let mut min_distance = f32::INFINITY;
            let mut nearest_neighbour = None;
            
            for &neighbour in neighbours {
                if seen.contains(&neighbour) {
                    continue;
                }
                let distance = Self::euclidean_distance_3d(&points[curr], &points[neighbour]);
                if distance < min_distance {
                    min_distance = distance;
                    nearest_neighbour = Some(neighbour);
                }
            }
            
            let next_point = nearest_neighbour.expect("Failed to find nearest neighbour");
            curr = next_point;
            order.push(curr);
            seen.insert(curr);
        }
        
        order
    }
    
    pub fn upsample_grid(&self, point_cloud: &PointCloud<PointXyzRgba>, partition_k: usize) -> Vec<PointXyzRgba> {
        /*
        1. Partition the vertices
        2. Parallel iter upsampling each segment
        3. combining into a single point cloud
         */
        let start = Instant::now();
        let partitions = Self::partition(&point_cloud, (partition_k, partition_k, partition_k));
        let new_points = partitions.par_iter().filter(|vertices| !vertices.is_empty()).flat_map(|vertices| Self::upsample_grid_vertices(vertices.clone())).collect::<Vec<_>>();
        println!("{:?}", start.elapsed().as_micros());
        new_points
    }
    
    fn upsample_grid_vertices_dedup(vertices: Vec<PointXyzRgba>) -> Vec<PointXyzRgba> {
        /*
        Upsamples vertices and returns only the upsampled points
         */
        let mut vertices = vertices;
        vertices.sort_unstable();
        let mut kd_tree = KdTree::new();
        for (i, pt) in vertices.iter().enumerate() {
            kd_tree
                .add(&[pt.x, pt.y, pt.z], i)
                .expect("Failed to add to kd tree");
        }
        // let end_kd_init = start.elapsed();
        let mut visited: HashSet<(usize, usize)> = HashSet::new();
        let mut new_points: Vec<PointXyzRgba> = vec![];
        let mut visited_points: HashSet<usize> = HashSet::new();
        for source in 0..vertices.len() {
            if visited_points.contains(&source){
                continue;
            }
            let point = vertices[source];
            let x = point.x;
            let y = point.y;
            let z = point.z;
            match kd_tree.nearest(&[x, y, z], 9, &squared_euclidean){
                Ok(nearest) => {
                    let neighbours = nearest.iter().map(|(_, second)| **second).skip(1).collect::<Vec<_>>();
                    if neighbours.len() != 8 {
                        continue;
                    }
                    visited_points.extend(&neighbours);
                    
                    let order = Self::get_circumference_order(&neighbours, &vertices);
                    
                    for i in 0..order.len() {
                        let next_i = (i + 1) % order.len();
                        let circumference_pair = if order[i] < order[next_i] { (order[i], order[next_i]) } else { (order[next_i], order[i]) };
                        let source_pair = if order[i] < source { (order[i], source) } else { (source, order[i]) };
                        
                        for &pair in &[circumference_pair, source_pair] {
                            if visited.contains(&pair) {
                                continue;
                            }
                            let middlepoint  = Self::get_middlepoint(&vertices[pair.0], &vertices[pair.1]);
                            new_points.push(middlepoint);
                        }
                        visited.insert(source_pair);
                        visited.insert(circumference_pair);
                        
                        let next_next_i = (i + 2) % order.len();
                        let dup_pair = if order[next_next_i] < order[i] { (order[next_next_i], order[i]) } else { (order[i], order[next_next_i]) };
                        visited.insert(dup_pair);
                    }
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        };
        new_points
    }
    fn upsample_grid_vertices(vertices: Vec<PointXyzRgba>) -> Vec<PointXyzRgba> {
        let mut kd_tree = KdTree::new();
        for (i, pt) in vertices.iter().enumerate() {
            kd_tree
                .add(&[pt.x, pt.y, pt.z], i)
                .expect("Failed to add to kd tree");
        }
        // let end_kd_init = start.elapsed();
        let mut visited: HashSet<(usize, usize)> = HashSet::new();
        let mut new_points: Vec<PointXyzRgba> = vec![];
        for source in 0..vertices.len() {
            
            let point = vertices[source];
            let x = point.x;
            let y = point.y;
            let z = point.z;
            match kd_tree.nearest(&[x, y, z], 9, &squared_euclidean){
                Ok(nearest) => {
                    let neighbours = nearest.iter().map(|(_, second)| **second).skip(1).collect::<Vec<_>>();
                    if neighbours.len() != 8 {
                        continue;
                    }
                    let order = Self::get_circumference_order(&neighbours, &vertices);
                    
                    for i in 0..order.len() {
                        let next_i = (i + 1) % order.len();
                        let circumference_pair = if order[i] < order[next_i] { (order[i], order[next_i]) } else { (order[next_i], order[i]) };
                        let source_pair = if order[i] < source { (order[i], source) } else { (source, order[i]) };
                        
                        for &pair in &[circumference_pair, source_pair] {
                            if visited.contains(&pair) {
                                continue;
                            }
                            let middlepoint  = Self::get_middlepoint(&vertices[pair.0], &vertices[pair.1]);
                            new_points.push(middlepoint);
                        }
                        visited.insert(source_pair);
                        visited.insert(circumference_pair);
                        
                        let next_next_i = (i + 2) % order.len();
                        let dup_pair = if order[next_next_i] < order[i] { (order[next_next_i], order[i]) } else { (order[i], order[next_next_i]) };
                        visited.insert(dup_pair);
                    }
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        };
        new_points.extend(vertices);
        new_points
    }
    



}
