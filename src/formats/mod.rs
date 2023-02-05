use serde::Serialize;

use crate::pcd::PointCloudData;

use self::pointxyzrgba::PointXyzRgba;

pub mod pointxyzrgba;

#[derive(Clone, Debug)]
pub struct PointCloud<T>
where
    T: Clone + Serialize,
{
    pub number_of_points: usize,
    pub points: Vec<T>,
}

impl<T> PointCloud<T>
where
    T: Clone + Serialize,
{
    pub(crate) fn combine(&mut self, other: &Self) {
        self.points.extend_from_slice(&other.points);
        self.number_of_points += other.number_of_points;
    }
}

impl<T> From<PointCloudData> for PointCloud<T>
where
    T: Clone + Serialize,
{
    fn from(pcd: PointCloudData) -> Self {
        let number_of_points = pcd.header.points() as usize;

        let mut v_clone = std::mem::ManuallyDrop::new(pcd.data);
        let points = unsafe {
            let factor = v_clone.len() / number_of_points;
            let capacity = v_clone.capacity() / factor;
            Vec::from_raw_parts(v_clone.as_mut_ptr() as *mut T, number_of_points, capacity)
        };
        Self {
            number_of_points,
            points,
        }
    }
}

impl From<tmc2rs::codec::PointSet3> for PointCloud<PointXyzRgba> {
    fn from(point_set: tmc2rs::codec::PointSet3) -> Self {
        let number_of_points = point_set.len();
        let points = (0..number_of_points)
            .map(|i| {
                let pos = point_set.positions[i];
                let color = point_set.colors[i];
                PointXyzRgba {
                    x: pos.x as f32,
                    y: pos.y as f32,
                    z: pos.z as f32,
                    r: color.x,
                    g: color.y,
                    b: color.z,
                    a: 0,
                }
            })
            .collect();
        Self {
            number_of_points,
            points,
        }
    }
}
