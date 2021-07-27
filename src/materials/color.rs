use lab::Lab;
use nalgebra::Point3;

pub struct Color {
    data: Vec<PointColor>,
}

impl Color {
    pub fn new(data: Vec<PointColor>) -> Self {
        Color { data }
    }

    pub fn get_point_col_vec(&self) -> &Vec<PointColor> {
        &self.data //self.data.into_iter().map(|coord| coord.get_point3()).collect()
    }
}

#[derive(Debug, Clone)]
pub struct PointColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl PartialEq for PointColor {
    fn eq(&self, other: &Self) -> bool {
        self.red == other.red && self.green == other.green && self.blue == other.blue
    }
}

impl PointColor {
    pub fn new_default() -> Self {
        PointColor {
            red: 0,
            green: 0,
            blue: 0,
        }
    }

    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        PointColor { red, green, blue }
    }

    fn new_with_array(array: [u8; 3]) -> PointColor {
        PointColor::new(array[0], array[1], array[2])
    }

    fn to_array(&self) -> [u8; 3] {
        [self.red, self.green, self.blue]
    }

    fn to_lab(&self) -> Lab {
        Lab::from_rgb(&self.to_array())
    }

    pub fn get_point3(&self) -> Point3<f32> {
        Point3::new(
            self.red as f32 / 256.0,
            self.green as f32 / 256.0,
            self.blue as f32 / 256.0,
        )
    }

    pub fn get_average(&self, another_point: &PointColor) -> PointColor {
        let lab_of_self = self.to_lab();
        let lab_of_another = another_point.to_lab();
        let lab_of_average = Lab {
            l: (lab_of_self.l + lab_of_another.l) / 2.0,
            a: (lab_of_self.a + lab_of_another.a) / 2.0,
            b: (lab_of_self.b + lab_of_another.b) / 2.0,
        };

        PointColor::new_with_array(lab_of_average.to_rgb())
    }

    pub fn get_color_delta(&self, another_point: &PointColor) -> f32 {
        let lab_of_self = self.to_lab();
        let lab_of_another = another_point.to_lab();

        (lab_of_self.l - lab_of_another.l)
            .hypot(lab_of_self.a - lab_of_another.a)
            .hypot(lab_of_self.b - lab_of_another.b)
    }
}
