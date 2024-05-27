use serde::{Deserialize, Serialize};

use super::pointxyzrgba::PointXyzRgba;

pub const DELTA: f32 = 1e-4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl Bounds {
    fn new(min_x: f32, max_x: f32, min_y: f32, max_y: f32, min_z: f32, max_z: f32) -> Self {
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z,
        }
    }

    pub fn split(&self) -> Vec<Bounds> {
        let &Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z,
        } = self;

        let bisect_x = (max_x + min_x) / 2f32;
        let bisect_y = (max_y + min_y) / 2f32;
        let bisect_z = (max_z + min_z) / 2f32;

        vec![
            Bounds::new(min_x, bisect_x, min_y, bisect_y, min_z, bisect_z),
            Bounds::new(min_x, bisect_x, min_y, bisect_y, bisect_z + DELTA, max_z),
            Bounds::new(min_x, bisect_x, bisect_y + DELTA, max_y, min_z, bisect_z),
            Bounds::new(
                min_x,
                bisect_x,
                bisect_y + DELTA,
                max_y,
                bisect_z + DELTA,
                max_z,
            ),
            Bounds::new(bisect_x + DELTA, max_x, min_y, bisect_y, min_z, bisect_z),
            Bounds::new(
                bisect_x + DELTA,
                max_x,
                min_y,
                bisect_y,
                bisect_z + DELTA,
                max_z,
            ),
            Bounds::new(
                bisect_x + DELTA,
                max_x,
                bisect_y + DELTA,
                max_y,
                min_z,
                bisect_z,
            ),
            Bounds::new(
                bisect_x + DELTA,
                max_x,
                bisect_y + DELTA,
                max_y,
                bisect_z + DELTA,
                max_z,
            ),
        ]
    }

    pub fn partition(&self, partitions: (usize, usize, usize)) -> Vec<Bounds> {
        let x_step = (self.max_x - self.min_x + DELTA) / partitions.0 as f32;
        let y_step = (self.max_y - self.min_y + DELTA) / partitions.1 as f32;
        let z_step = (self.max_z - self.min_z + DELTA) / partitions.2 as f32;

        let mut bounds = vec![];

        for z in 0..partitions.2 {
            for y in 0..partitions.1 {
                for x in 0..partitions.0 {
                    bounds.push(Bounds::new(
                        self.min_x + x as f32 * x_step,
                        self.min_x + (x + 1) as f32 * x_step,
                        self.min_y + y as f32 * y_step,
                        self.min_y + (y + 1) as f32 * y_step,
                        self.min_z + z as f32 * z_step,
                        self.min_z + (z + 1) as f32 * z_step,
                    ));
                }
            }
        }

        bounds
    }

    pub fn get_vertexes(&self) -> Vec<[f32; 3]> {
        vec![
            [self.min_x, self.min_y, self.min_z],
            [self.max_x, self.min_y, self.min_z],
            [self.min_x, self.max_y, self.min_z],
            [self.max_x, self.max_y, self.min_z],
            [self.min_x, self.min_y, self.max_z],
            [self.max_x, self.min_y, self.max_z],
            [self.min_x, self.max_y, self.max_z],
            [self.max_x, self.max_y, self.max_z],
        ]
    }

    pub fn contains(&self, point: &PointXyzRgba) -> bool {
        point.x >= self.min_x
            && point.x <= self.max_x
            && point.y >= self.min_y
            && point.y <= self.max_y
            && point.z >= self.min_z
            && point.z <= self.max_z
    }
}
