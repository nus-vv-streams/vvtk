use std::fmt::Debug;

use crate::pcd::PointCloudData;
use crate::velodyne::{VelodynPoint, VelodyneBinData};

pub mod bounds;
pub mod metadata;
pub mod pointxyzrgba;
pub mod pointxyzrgbanormal;

#[derive(Clone)]
pub struct PointCloud<T>
where
    T: Clone,
{
    pub number_of_points: usize,
    pub segments: Vec<PointCloudSegment<T>>,
    pub points: Vec<T>,
}

#[derive(Clone)]
pub struct PointCloudSegment<T> {
    pub number_of_points: usize,
    pub points: Vec<T>,
}

impl<T> PointCloud<T>
where
    T: Clone,
{
    pub fn new(number_of_points: usize, points: Vec<T>) -> Self {
        let segments = vec![PointCloudSegment {
            number_of_points,
            points: points.clone(),
        }];
        Self {
            number_of_points,
            segments,
            points,
        }
    }

    pub fn new_with_segments(segments: Vec<Vec<T>>) -> Self {
        let points: Vec<T> = segments.iter().flatten().cloned().collect();
        let number_of_points = points.len();
        let segments = segments
            .into_iter()
            .map(|segment| PointCloudSegment {
                number_of_points: segment.len(),
                points: segment,
            })
            .collect();
        Self {
            number_of_points,
            segments,
            points,
        }
    }

    pub fn is_partitioned(&self) -> bool {
        self.segments.len() > 1
    }

    pub fn merge_points(&self, points: Vec<T>) -> Self {
        let number_of_points = self.number_of_points + points.len();
        let segments = vec![PointCloudSegment {
            number_of_points,
            points: points.clone(),
        }];
        let mut all_points = self.points.clone();
        all_points.extend(points);
        Self {
            number_of_points,
            segments,
            points: all_points,
        }
    }
}

impl Debug for PointCloud<pointxyzrgba::PointXyzRgba> {
    // first print the number of points in one line
    // then for each T in the Vec, print in a new line
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "PointCloud<PointXyzRgba> {{")?;
        writeln!(f, "   number_of_points: {}", self.number_of_points)?;
        for point in &self.points {
            writeln!(f, "   {:?}", point)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl Debug for PointCloud<pointxyzrgbanormal::PointXyzRgbaNormal> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "PointCloud<PointXyzRgbaNormal> {{")?;
        writeln!(f, "   number_of_points: {}", self.number_of_points)?;
        for point in &self.points {
            writeln!(f, "   {:?}", point)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl<T> From<PointCloudData> for PointCloud<T>
where
    T: Clone,
{
    fn from(pcd: PointCloudData) -> Self {
        let number_of_points = pcd.header.points() as usize;

        let mut v_clone = std::mem::ManuallyDrop::new(pcd.data);
        let points = unsafe {
            let factor = v_clone.len() / number_of_points;
            let capacity = v_clone.capacity() / factor;
            Vec::from_raw_parts(v_clone.as_mut_ptr() as *mut T, number_of_points, capacity)
        };
        Self::new(number_of_points, points)
    }
}

impl From<VelodyneBinData> for PointCloud<pointxyzrgba::PointXyzRgba> {
    // type T: pointxyzrgba::PointXyzRgba;
    fn from(value: VelodyneBinData) -> Self {
        let number_of_points = value.data.len();
        let points = value.data.into_iter().map(|point| point.into()).collect();
        Self::new(number_of_points, points)
    }
}

impl From<VelodynPoint> for pointxyzrgba::PointXyzRgba {
    fn from(value: VelodynPoint) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
            r: (value.intensity * 255.0) as u8,
            g: (value.intensity * 255.0) as u8,
            b: (value.intensity * 255.0) as u8,
            a: 255,
        }
    }
}
