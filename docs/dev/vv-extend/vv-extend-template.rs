use std::io::{self, BufRead};
use serde::{Serialize, Deserialize};
use std::fmt::Debug;

// Template code for PointXyzRgba struct
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointXyzRgba {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

// Template code for PointCloud
#[derive(Clone, Deserialize, Serialize)]
pub struct PointCloud<T> {
    pub number_of_points: usize,
    pub points: Vec<T>,
}

impl Debug for PointCloud<PointXyzRgba> {
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

// Template code for SubcommandObject
#[derive(Debug, Serialize, Deserialize)]
pub struct SubcommandObject<T:Clone + Serialize> {
    content: Box<T>,
}

impl<T:Clone + Serialize> SubcommandObject<T> {
    pub fn new(content: T) -> Self {
        Self {
            content: Box::new(content),
        }
    }
}

impl<T:Clone + Serialize> Clone for SubcommandObject<T> {
    fn clone(&self) -> Self {
        Self {
            content: self.content.clone(),
        }
    }
}

// Here's the actual implementation of the program
fn main() {
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = line.expect("Could not read the piped stdin");
        let deserialised_input = line.clone();
        // Regenerate the SubcommandObject from input 
        let deserialized: SubcommandObject<PointCloud<PointXyzRgba>> = serde_json::from_str(&deserialised_input).unwrap();
        let mut deserialized_pc = *deserialized.content;
        // Do something here to transform the deserialized_pc
        deserialized_pc = new_pc_transform_function(deserialized_pc);
        // Serialize the point cloud
        let new_subcommand_object = SubcommandObject::new(deserialized_pc);
        // Pass serialized SubcommandObject to the parent process
        let serialized: String = serde_json::to_string(&new_subcommand_object).unwrap();
        print!("{}", serialized);
    }
}

// Custom function to transform the point cloud input
fn new_pc_transform_function(pc:PointCloud<PointXyzRgba>) -> PointCloud<PointXyzRgba> {
    pc
}
