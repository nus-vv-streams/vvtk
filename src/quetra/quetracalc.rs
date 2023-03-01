use cgmath::num_traits::Pow;
use clap::Parser;
use std::f64::consts::E;

pub struct QuetraCalc {
    pub name: String,
    pub k: i64,                 // buffer capacity
    pub r_vec: Vec<f64>, // a vector of different bitrates of segment being downloaded (i.e. low, mid, high)
    pub b: f64,          // network throughput
    pub buffer_occupancy: f64, // buffer occupancy (Bt)
    pub segment_frequency: i32, // segment frequency for bitrate adaptation
    pub segment_size: i32, // segment size param for granularity of each buffer
}

impl QuetraCalc {
    fn new(
        name: String,
        k: i64,
        r_vec: Vec<f64>,
        b: f64,
        buffer_occupancy: f64,
        segment_frequency: i32,
        segment_size: i32,
    ) -> Self {
        QuetraCalc {
            name,
            k,
            r_vec,
            b,
            buffer_occupancy,
            segment_frequency,
            segment_size,
        }
    }

    fn factorial(i: i64) -> i64 {
        assert!(i >= 0); // panics if i is negative
        if i == 0 {
            return 1; // factorial(0) = 1 by definition
        }
        return i * Self::factorial(i - 1);
    }

    fn get_quetra_formula_x_i(i: i64, r: f64, b: f64) -> f64 {
        let mut x_i: f64 = 0.0f64;
        let mut j: i64 = 0;

        while j < i + 1 {
            x_i += ((-1 as i64).pow(j as u32) / Self::factorial(j)) as f64
                * (i - j).pow(j as u32) as f64
                * (b / r).powi(j.try_into().unwrap())
                * E.pow((i - j) as f64 * (b / r));
            // * pow(i - j, j) // (i - j).pow(j)
            // * pow(b / r, j) // (b / r).pow(j)
            // * pow(E, (i - j) as f32 * (b / r)); // E.pow((i - j) * (b / r))
            j += 1;
        }

        return x_i;
    }

    // TODO: handle cases where k is too large inside this fn?
    fn get_quetra_formula_pkrb(k: i64, r: f64, b: f64) -> f64 {
        let mut i: i64 = 0;
        let mut pkrb_numerator: f64 = 0.0f64;

        while i < k {
            pkrb_numerator += Self::get_quetra_formula_x_i(i, r, b);
            // println!("pkrb_numerator: {}", pkrb_numerator);
            i += 1;
        }

        return pkrb_numerator / (1.0f64 + ((b / r) * Self::get_quetra_formula_x_i(k - 1, r, b)));
    }

    fn select_bitrate(&self) -> f64 {
        let mut result: f64 = 0.0f64;
        let mut min_diff_with_buffer_occupancy: f64 = f64::MAX;

        // for each r_i inside r_vec, calculate its pkrb value (pkrb_r_i)
        // then replace result with r_i if its pkrb value has a smaller difference with buffer occupancy Bt (pkrb_r_i - Bt)
        for r in &self.r_vec {
            let pkrb_r_i: f64 = Self::get_quetra_formula_pkrb(self.k, *r, self.b);
            if (pkrb_r_i - self.buffer_occupancy) < min_diff_with_buffer_occupancy {
                result = *r;
                min_diff_with_buffer_occupancy = pkrb_r_i - self.buffer_occupancy;
            }
        }

        return result;
    }
}

trait RateAdaptation {
    fn description(&self) -> String;
    fn get_buffer_occupancy(&self) -> f64;
    fn get_bitrate(&self) -> f64;
}

impl RateAdaptation for QuetraCalc {
    fn description(&self) -> String {
        return format!("This algorithm uses {}, with buffer_occupancy {} resulting in a selected_bitrate of {}.", self.name, self.buffer_occupancy, self.select_bitrate());
    }

    fn get_buffer_occupancy(&self) -> f64 {
        return self.buffer_occupancy;
    }

    fn get_bitrate(&self) -> f64 {
        return self.select_bitrate();
    }
}
