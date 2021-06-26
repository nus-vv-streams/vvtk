use nalgebra::Point3;
use lab::{ Lab };

pub struct Color {
    data: Vec<PointColor>
}

impl Color {
    pub fn new(data: Vec<PointColor>) -> Self {
        Color {
            data: data
        }
    }

    pub fn get(self) -> Vec<Point3<f32>> {
        self.data.into_iter().map(|point_color| point_color.get_point3()).collect()
    }
}

#[derive(Debug)]
#[derive(Clone)]
pub struct PointColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8
}

impl PointColor {
    pub fn new_default() -> Self {
        PointColor {
            red: 0,
            green: 0,
            blue: 0
        }
    }

    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        PointColor {
            red: red,
            green: green,
            blue: blue,
        }
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
       Point3::new(self.red as f32 /256.0, self.green as f32 /256.0, self.blue as f32 /256.0)
    }

    pub fn get_average(self, another_point: PointColor) -> PointColor {
        let lab_of_self = self.to_lab();
        let lab_of_another = another_point.to_lab();
        let lab_of_average = Lab { l: (lab_of_self.l + lab_of_another.l)/2.0,
                                    a: (lab_of_self.a + lab_of_another.a)/2.0,
                                    b: (lab_of_self.b + lab_of_another.b)/2.0 };

        PointColor::new_with_array(lab_of_average.to_rgb())
        
    }

        // PointColor::new((self.red + another_point.red)/2
        //                 ,(self.green + another_point.green)/2
        //                 ,(self.blue + another_point.blue)/2)
    

    pub fn get_color_delta(&self, another_point: &PointColor) -> f32 {
        let lab_of_self = self.to_lab();
        let lab_of_another = another_point.to_lab();

        (lab_of_self.l - lab_of_another.l)
            .hypot(lab_of_self.a - lab_of_another.a)
            .hypot(lab_of_self.b - lab_of_another.b)
    }
}