use cgmath::{Matrix4, Point3, Transform};
use kiddo::{distance::squared_euclidean, KdTree};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{
    formats::{bounds::Bounds, pointxyzrgba::PointXyzRgba, PointCloud},
    utils::get_pc_bound,
};
use std::{cmp::min, collections::HashSet};

use super::{camera::CameraState, renderable::Renderable};

pub struct Upsampler {}

impl Upsampler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn upsample(
        pc: &PointCloud<PointXyzRgba>,
        camera_state: &CameraState,
        partition_k: usize,
    ) -> Option<PointCloud<PointXyzRgba>> {
        let should_upsample = Self::should_upsample(pc, camera_state);

        if should_upsample {
            let upsampled_points = Self::upsample_grid(pc, partition_k);
            Some(PointCloud::new(
                upsampled_points.len(),
                upsampled_points.clone(),
            ))
        } else {
            None
        }
    }

    pub fn get_visible_points(
        point_cloud: PointCloud<PointXyzRgba>,
        camera_state: &CameraState,
    ) -> PointCloud<PointXyzRgba> {
        let view_proj_matrix = Matrix4::from(camera_state.camera_uniform.view_proj);
        let antialias = point_cloud.antialias();

        let visible_points = point_cloud
            .points
            .into_par_iter()
            .filter(|point| {
                let point_vec = Point3::new(
                    point.x - antialias.x,
                    point.y - antialias.y,
                    point.z - antialias.z,
                ) / antialias.scale;
                let point_in_view = view_proj_matrix * point_vec.to_homogeneous();
                (point_in_view.x / point_in_view.w).abs() <= 1.0
                    && (point_in_view.y / point_in_view.w).abs() <= 1.0
                    && (point_in_view.z / point_in_view.w).abs() <= 1.0
            })
            .collect::<Vec<_>>();
        PointCloud::new(visible_points.len(), visible_points)
    }

    pub fn should_upsample(
        point_cloud: &PointCloud<PointXyzRgba>,
        camera_state: &CameraState,
    ) -> bool {
        if point_cloud.points.is_empty() || point_cloud.points.len() > 300_000 {
            return false;
        }
        const RANGE_PIXEL_THRESHOLD: i32 = 1;
        const PERCENTAGE_THRESHOLD: f32 = 0.8;

        let view_proj_matrix = Matrix4::from(camera_state.camera_uniform.view_proj);
        let antialias = point_cloud.antialias();
        let width = camera_state.get_window_size().width as usize;
        let height = camera_state.get_window_size().height as usize;
        let points_viewport = point_cloud
            .points
            .par_iter()
            .map(|point| {
                let point_vec = Point3::new(
                    point.x - antialias.x,
                    point.y - antialias.y,
                    point.z - antialias.z,
                ) / antialias.scale;
                let point_ndc = view_proj_matrix.transform_point(point_vec);
                let x = min(((point_ndc.x + 1.0) * (width as f32) / 2.0) as usize, width);
                let y = min(
                    ((point_ndc.y + 1.0) * (height as f32) / 2.0) as usize,
                    height,
                );
                (x, y)
            })
            .collect::<Vec<_>>();

        let mut viewport_is_filled = vec![false; (height + 1) * (width + 1)];

        points_viewport.iter().for_each(|&coords| {
            let (x, y) = coords;
            viewport_is_filled[y * (width + 1) + x] = true;
        });

        let number_pixels_with_close_neighbours = (0..(viewport_is_filled.len()))
            .into_par_iter()
            .filter(|&index| viewport_is_filled[index])
            .map(|val| (val % (width + 1), val / (width + 1)))
            .filter(|(x, y)| {
                let x = *x;
                let y = *y;
                let mut filled_neighbours_count = 0;
                for x_curr in
                    ((x as i32) - RANGE_PIXEL_THRESHOLD)..((x as i32) + RANGE_PIXEL_THRESHOLD + 1)
                {
                    for y_curr in ((y as i32) - RANGE_PIXEL_THRESHOLD)
                        ..((y as i32) + RANGE_PIXEL_THRESHOLD + 1)
                    {
                        if 0 > x_curr
                            || x_curr > (width as i32)
                            || 0 > y_curr
                            || y_curr > (height as i32)
                            || (x_curr, y_curr) == (x as i32, y as i32)
                        {
                            continue;
                        }
                        let x_curr = x_curr as usize;
                        let y_curr = y_curr as usize;
                        if viewport_is_filled[y_curr * (width + 1) + x_curr] {
                            filled_neighbours_count += 1;
                        }
                    }
                }
                return filled_neighbours_count >= 4;
            })
            .count();
        let filled_pixels: usize = viewport_is_filled
            .par_iter()
            .filter(|&&is_filled| is_filled)
            .count();
        let percentage_pixels_close_enough =
            (number_pixels_with_close_neighbours as f32) / (filled_pixels as f32);

        percentage_pixels_close_enough < PERCENTAGE_THRESHOLD
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

        child_bounds
            .par_iter()
            .map(|bound| {
                pc.points
                    .iter()
                    .map(|point| point.clone())
                    .filter(|point| Self::contains(bound, point))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
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

    pub fn upsample_grid(
        point_cloud: &PointCloud<PointXyzRgba>,
        partition_k: usize,
    ) -> Vec<PointXyzRgba> {
        /*
        1. Partition the vertices
        2. Parallel iter upsampling each segment
        3. combining into a single point cloud
         */
        // let start = Instant::now();
        let partitions = Self::partition(&point_cloud, (partition_k, partition_k, partition_k));
        let new_points = partitions
            .par_iter()
            .filter(|vertices| !vertices.is_empty())
            .flat_map(|vertices| Self::upsample_grid_vertices(vertices.clone()))
            .collect::<Vec<_>>();
        new_points
    }

    fn upsample_grid_vertices(vertices: Vec<PointXyzRgba>) -> Vec<PointXyzRgba> {
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
            if visited_points.contains(&source) {
                continue;
            }
            let point = vertices[source];
            let x = point.x;
            let y = point.y;
            let z = point.z;
            match kd_tree.nearest(&[x, y, z], 9, &squared_euclidean) {
                Ok(nearest) => {
                    let neighbours = nearest
                        .iter()
                        .map(|(_, second)| **second)
                        .skip(1)
                        .collect::<Vec<_>>();
                    // visited_points.extend(neighbours.clone());

                    if neighbours.len() != 8 {
                        continue;
                    }

                    let order = Self::get_circumference_order(&neighbours, &vertices);
                    for (index, value) in order.iter().enumerate() {
                        if index % 2 == 0 {
                            visited_points.insert(*value);
                        }
                    }
                    for i in 0..order.len() {
                        let next_i = (i + 1) % order.len();
                        let circumference_pair = if order[i] < order[next_i] {
                            (order[i], order[next_i])
                        } else {
                            (order[next_i], order[i])
                        };
                        let source_pair = if order[i] < source {
                            (order[i], source)
                        } else {
                            (source, order[i])
                        };

                        for &pair in &[circumference_pair, source_pair] {
                            if visited.contains(&pair) {
                                continue;
                            }
                            let middlepoint =
                                Self::get_middlepoint(&vertices[pair.0], &vertices[pair.1]);
                            new_points.push(middlepoint);
                        }
                        visited.insert(source_pair);
                        visited.insert(circumference_pair);

                        let next_next_i = (i + 2) % order.len();
                        let dup_pair = if order[next_next_i] < order[i] {
                            (order[next_next_i], order[i])
                        } else {
                            (order[i], order[next_next_i])
                        };
                        visited.insert(dup_pair);
                    }
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        }
        new_points.extend(vertices);

        new_points
    }
}