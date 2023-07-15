#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]

pub struct TriangleFace {
    pub v1: i32,
    pub v2: i32,
    pub v3: i32,
}

impl TriangleFace {
    pub fn get_default_mesh(num_points: i32) -> Vec<TriangleFace> {
        assert!(
            num_points % 3 == 0,
            "points from poissonRecon must be divisible by 3"
        );
        // Create a vector to store the TriangleFace instances
        let mut mesh = Vec::with_capacity(num_points as usize / 3);

        // Generate the default mesh
        for i in (0..num_points).step_by(3) {
            let face = TriangleFace {
                v1: i,
                v2: i + 1,
                v3: i + 2,
            };
            mesh.push(face);
        }

        mesh
    }
}
