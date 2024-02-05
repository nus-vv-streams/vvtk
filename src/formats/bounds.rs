use super::pointxyzrgba::PointXyzRgba;

const DELTA: f32 = 1e-4;

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

    pub fn contains(&self, point: &PointXyzRgba) -> bool {
        point.x >= self.min_x
            && point.x <= self.max_x
            && point.y >= self.min_y
            && point.y <= self.max_y
            && point.z >= self.min_z
            && point.z <= self.max_z
    }
}
