mod coordinate {
    /// Structure representing the 3D-coordinate of one point.
    #[derive(Debug, Clone)]
    pub struct Coordinate {
        /// x-coordinate
        pub x: f32,
        /// y-coordinate
        pub y: f32,
        /// z-coordinate
        pub z: f32,
    }

    impl PartialEq for Coordinate {
        fn eq(&self, other: &Self) -> bool {
            self.x == other.x && self.y == other.y && self.z == other.z
        }
    }

    impl Coordinate {
        /// Return the original
        pub fn new_default() -> Self {
            Coordinate {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }
        }

        /// Return a `Coordinate` with specific coordinates
        pub fn new(x: f32, y: f32, z: f32) -> Self {
            Coordinate { x, y, z }
        }

        /// Return the `Point3` type of the `Coordinate` for rendering
        pub fn get_point3(&self) -> nalgebra::Point3<f32> {
            nalgebra::Point3::new(self.x, self.y, self.z)
        }

        /// Return a midpoint of two `Coordinate`s
        pub fn get_weighted_average(&self, p: &Coordinate, alpha: f32) -> Coordinate {
            Coordinate::new(
                (self.x * alpha) + (p.x * (1.0 - alpha)),
                (self.y * alpha) + (p.y * (1.0 - alpha)),
                (self.z * alpha) + (p.z * (1.0 - alpha)),
            )
        }

        /// Return a midpoint of two `Coordinate`s
        pub fn get_average(&self, p: &Coordinate) -> Coordinate {
            Coordinate::new(
                (self.x + p.x) * 0.5,
                (self.y + p.y) * 0.5,
                (self.z + p.z) * 0.5,
            )
        }

        /// Return the distance between two `Coordinate`s
        pub fn get_coord_delta(&self, another_point: &Coordinate) -> f32 {
            (self.x - another_point.x)
                .hypot(self.y - another_point.y)
                .hypot(self.z - another_point.z)
        }
    }
}

mod color {
    /// Structure representing the colors (in RGB) of one point.
    #[derive(Debug, Clone)]
    pub struct Color {
        /// red
        pub r: u8,
        /// green
        pub g: u8,
        /// blue
        pub b: u8,
    }

    impl PartialEq for Color {
        fn eq(&self, other: &Self) -> bool {
            self.r == other.r && self.g == other.g && self.b == other.b
        }
    }

    impl Color {
        /// Return a white `Color` (R = G = B = 0)
        pub fn new_default() -> Self {
            Color { r: 0, g: 0, b: 0 }
        }

        /// Return a `Color` with specific RGB
        pub fn new(r: u8, g: u8, b: u8) -> Self {
            Color { r, g, b }
        }

        /// Return the `Point3` type of the `Color` for rendering
        pub fn get_point3(&self) -> nalgebra::Point3<f32> {
            nalgebra::Point3::new(
                self.r as f32 / 256.0,
                self.g as f32 / 256.0,
                self.b as f32 / 256.0,
            )
        }

        /// Return a average `Color` of two `Color`s
        pub fn get_weighted_average(&self, p: &Color, alpha: f32) -> Color {
            // let lab_of_self = self.to_lab();
            // let lab_of_another = another_point.to_lab();
            // let lab_of_average = Lab {
            //     l: (lab_of_self.l + lab_of_another.l) / 2.0,
            //     a: (lab_of_self.a + lab_of_another.a) / 2.0,
            //     b: (lab_of_self.b + lab_of_another.b) / 2.0,
            // };

            // Color::new_with_array(lab_of_average.to_rgb())

            // Color::new(
            //     ((self.r as usize + another_point.r as usize) / 2) as u8,
            //     ((self.g as usize + another_point.g as usize) / 2) as u8,
            //     ((self.b as usize + another_point.b as usize) / 2) as u8,
            // )

            Color::new(
                ((self.r as f32 * alpha) + (p.r as f32 * (1.0 - alpha))) as u8,
                ((self.g as f32 * alpha) + (p.g as f32 * (1.0 - alpha))) as u8,
                ((self.b as f32 * alpha) + (p.b as f32 * (1.0 - alpha))) as u8,
            )
        }

        /// Return the difference between two `Color`s
        pub fn get_color_delta(&self, p: &Color) -> f32 {
            // let lab_of_self = self.to_lab();
            // let lab_of_another = another_point.to_lab();

            // (lab_of_self.l - lab_of_another.l)
            //     .hypot(lab_of_self.a - lab_of_another.a)
            //     .hypot(lab_of_self.b - lab_of_another.b)
            (self.r as f32 - p.r as f32)
                .hypot(self.g as f32 - p.g as f32)
                .hypot(self.b as f32 - p.b as f32)
        }
    }
}

#[derive(Debug, Clone)]
/// Structure presenting a point
pub struct Point {
    pub(crate) coord: coordinate::Coordinate,
    pub(crate) color: color::Color,
    pub(crate) mapping: u16,
    pub(crate) index: usize,
    pub(crate) point_size: f32,
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.coord == other.coord && self.color == other.color
    }
}

impl Point {
    pub(crate) fn new(
        coord: coordinate::Coordinate,
        color: color::Color,
        mapping: u16,
        index: usize,
        point_size: f32,
    ) -> Self {
        Point {
            coord,
            color,
            mapping,
            index,
            point_size,
        }
    }

    pub(crate) fn new_default() -> Self {
        Point {
            coord: coordinate::Coordinate::new_default(),
            color: color::Color::new_default(),
            mapping: 0,
            index: 0,
            point_size: 1.0,
        }
    }

    pub(crate) fn get_coord(&self) -> &coordinate::Coordinate {
        &self.coord
    }

    pub(crate) fn get_color(&self) -> &color::Color {
        &self.color
    }

    pub(crate) fn set_index(&mut self, idx: usize) {
        self.index = idx;
    }

    pub fn get_point(&self) -> [f32; 3] {
        [self.coord.x, self.coord.y, self.coord.z]
    }

    pub fn get_6dpoint(&self) -> [f32; 6] {
        [
            self.coord.x,
            self.coord.y,
            self.coord.z,
            self.color.r as f32,
            self.color.g as f32,
            self.color.b as f32,
        ]
    }

    /// Returns a Point whose coordinates and colours are the average of 2 given points
    pub fn get_weighted_average(&self, p: &Point, alpha: f32) -> Point {
        Point::new(
            self.coord.get_weighted_average(&p.coord, alpha),
            self.color.get_weighted_average(&p.color, alpha),
            0,
            p.index,
            (self.point_size + p.point_size) / 2.0,
        )
    }

    pub fn get_coord_delta(&self, another_point: &Point) -> f32 {
        self.coord.get_coord_delta(&another_point.coord)
    }

    pub fn get_color_delta(&self, another_point: &Point) -> f32 {
        self.color.get_color_delta(&another_point.color)
    }

    /// Add `Color` and `index` to create a `Point`
    pub fn set_color(&self, r: u8, g: u8, b: u8) -> Point {
        Point::new(
            self.coord.clone(),
            color::Color::new(r, g, b),
            0,
            self.index,
            self.point_size,
        )
    }

    /// Add `Color` and `index` to create a `Point`
    pub fn set_size(&self, size: f32) -> Point {
        Point::new(self.coord.clone(), self.color.clone(), 0, self.index, size)
    }

    pub fn coord(&self) -> [f32; 3] {
        [self.coord.x, self.coord.y, self.coord.z]
    }

    pub fn coord_and_colors(&self) -> [f32; 6] {
        [
            self.coord.x,
            self.coord.y,
            self.coord.z,
            self.color.r as f32,
            self.color.g as f32,
            self.color.b as f32,
        ]
    }
}
