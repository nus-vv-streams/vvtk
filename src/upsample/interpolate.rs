use std::{collections::HashSet, time::Instant};

use cgmath::Matrix4;
use kiddo::{distance::squared_euclidean, KdTree};
use log::warn;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{formats::{bounds::Bounds, pointxyzrgba::PointXyzRgba, PointCloud}, render::wgpu::upsampler::Upsampler, utils::get_pc_bound};

pub fn upsample(point_cloud: PointCloud<PointXyzRgba>, factor: usize) -> PointCloud<PointXyzRgba> {
    if factor <= 1 {
        point_cloud
    } else {
        let points = point_cloud.points;
        let neighbour_radius = factor as f32 * 2.0 * 9.0;
        let mut kd_tree = KdTree::new();
        for (i, pt) in points.iter().enumerate() {
            kd_tree
                .add(&[pt.x, pt.y, pt.z], i)
                .expect("Failed to add to kd tree");
        }
        let mut new_points = vec![];
        let mut processed = HashSet::new();

        for i in 0..point_cloud.number_of_points {
            processed.insert(i);
            let point = points[i];
            let x = point.x;
            let y = point.y;
            let z = point.z;
            match kd_tree.within(&[x, y, z], neighbour_radius, &squared_euclidean) {
                Ok(nearest) => {
                    for (dist, &idx) in nearest {
                        if processed.contains(&idx) {
                            continue;
                        }

                        let point_n = points[idx];
                        let x_n = point_n.x;
                        let y_n = point_n.y;
                        let z_n = point_n.z;

                        let x_diff = x_n - x;
                        let y_diff = y_n - y;
                        let z_diff = z_n - z;

                        for k in 1..=(2 * factor) {
                            let k = k as f32;
                            let factor = factor as f32;
                            let scale = k / (2.0 * factor);

                            let geom_x = x + (x_diff * scale);
                            let geom_y = y + (y_diff * scale);
                            let geom_z = z + (z_diff * scale);

                            let pi_dist = dist * scale;
                            let ni_dist = dist - pi_dist;

                            let col_r = ((point.r as f32) * pi_dist + (point_n.r as f32) * ni_dist)
                                * (1.0 / dist);
                            let col_g = ((point.g as f32) * pi_dist + (point_n.g as f32) * ni_dist)
                                * (1.0 / dist);
                            let col_b = ((point.b as f32) * pi_dist + (point_n.b as f32) * ni_dist)
                                * (1.0 / dist);
                            let col_a = ((point.a as f32) * pi_dist + (point_n.a as f32) * ni_dist)
                                * (1.0 / dist);
                            new_points.push(PointXyzRgba {
                                x: geom_x,
                                y: geom_y,
                                z: geom_z,
                                r: col_r as u8,
                                g: col_g as u8,
                                b: col_b as u8,
                                a: col_a as u8,
                            })
                        }
                    }
                }
                Err(e) => {
                    warn!("{:?}", e);
                    continue;
                }
            }
        }
        new_points.extend(points);
        PointCloud::new(new_points.len(), new_points)
    }
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
        pc.points.iter().map(|point| point.clone()).filter(|point| contains(bound, point)).collect::<Vec<_>>()
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
            let distance = euclidean_distance_3d(&points[curr], &points[neighbour]);
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

pub fn upsample_grid(point_cloud: PointCloud<PointXyzRgba>, partition_k: usize) -> PointCloud<PointXyzRgba> {
    /*
    1. Partition the vertices
    2. Parallel iter upsampling each segment
    3. combining into a single point cloud
     */
    let start = Instant::now();
    let partitions = partition(&point_cloud, (partition_k, partition_k, partition_k));
    let new_points = partitions.par_iter().filter(|vertices| !vertices.is_empty()).flat_map(|vertices| upsample_grid_vertices_dedup(vertices.clone())).collect::<Vec<_>>();
    println!("{:?}", start.elapsed().as_micros());
    PointCloud::new(new_points.len(), new_points)
}

fn upsample_grid_vertices_dedup(vertices: Vec<PointXyzRgba>) -> Vec<PointXyzRgba> {
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
                
                let order = get_circumference_order(&neighbours, &vertices);
                for (index, value) in order.iter().enumerate() {
                    if index % 2 == 0 {
                        visited_points.insert(*value);
                    }
                }
                for i in 0..order.len() {
                    let next_i = (i + 1) % order.len();
                    let circumference_pair = if order[i] < order[next_i] { (order[i], order[next_i]) } else { (order[next_i], order[i]) };
                    let source_pair = if order[i] < source { (order[i], source) } else { (source, order[i]) };
                    
                    for &pair in &[circumference_pair, source_pair] {
                        if visited.contains(&pair) {
                            continue;
                        }
                        let middlepoint  = get_middlepoint(&vertices[pair.0], &vertices[pair.1]);
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
                let order = get_circumference_order(&neighbours, &vertices);

                for i in 0..order.len() {
                    let next_i = (i + 1) % order.len();
                    let circumference_pair = if order[i] < order[next_i] { (order[i], order[next_i]) } else { (order[next_i], order[i]) };
                    let source_pair = if order[i] < source { (order[i], source) } else { (source, order[i]) };
                    
                    for &pair in &[circumference_pair, source_pair] {
                        if visited.contains(&pair) {
                            continue;
                        }
                        let middlepoint  = get_middlepoint(&vertices[pair.0], &vertices[pair.1]);
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
    println!("Original count: {:?}", vertices.len());
    new_points.extend(vertices);
    println!("Upsampled count: {:?}", new_points.len());
    println!("Visited pairs count: {:?}", visited.len());
    new_points
}

#[cfg(test)]
mod test {
    use crate::{
        pcd::{create_pcd, write_pcd_file},
        utils::read_file_to_point_cloud,
    };

    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_upsample() {
        let pcd_path = PathBuf::from("./test_files/pcd_ascii/longdress_vox10_1213_short.pcd");
        let pcd = read_file_to_point_cloud(&pcd_path).unwrap();
        println!("before: {:?}", pcd);
        let upsampled = upsample(pcd, 2);
        println!("Upsampled: {:?}", upsampled);
        // write pcd
        let out_path =
            PathBuf::from("./test_files/pcd_ascii/longdress_vox10_1213_short_upsampled.pcd");
        let pcd = create_pcd(&upsampled);
        write_pcd_file(&pcd, crate::pcd::PCDDataType::Ascii, &out_path).unwrap();
    }
}
