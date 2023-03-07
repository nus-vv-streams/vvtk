use cgmath::num_traits::Pow;
use std::f64::consts::E;

use super::RateAdapter;

pub struct Quetra {
    pub name: String,
    /// max buffer capacity measured in seconds of playback
    pub k: u64,
    /// how often is the bitrate adaptation done (selected as 1s for now)
    segment_frequency: u32,
    /// segment size param for granularity of each buffer (selected as 1 for now)
    segment_size: u32,
}

impl Quetra {
    pub fn new(name: String, buffer_capacity: u64) -> Self {
        Quetra {
            name,
            k: buffer_capacity,
            segment_frequency: 1,
            segment_size: 1,
        }
    }

    fn factorial(i: u64) -> u64 {
        match i {
            0 => 1,
            1 => 1,
            2 => 2,
            3 => 6,
            4 => 24,
            5 => 120,
            _ => i * (i - 1) * (i - 2) * (i - 3) * (i - 4) * Self::factorial(i - 5),
        }
    }

    fn get_quetra_formula_x_i(i: u64, r: f64, b: f64) -> f64 {
        let mut x_i: f64 = 0.0f64;

        for j in 0..=i {
            let x = (i - j).pow(j as u32) as f64 / Self::factorial(j) as f64
                * (b / r).powi(j.try_into().unwrap())
                * E.pow((i - j) as f64 * (b / r));
            // * pow(i - j, j) // (i - j).pow(j)
            // * pow(b / r, j) // (b / r).pow(j)
            // * pow(E, (i - j) as f32 * (b / r)); // E.pow((i - j) * (b / r))
            if j % 2 == 0 {
                x_i += x;
            } else {
                x_i -= x;
            }
        }

        x_i
    }

    // TODO: handle cases where k is too large inside this fn?
    fn get_quetra_formula_pkrb(k: u64, r: f64, b: f64) -> f64 {
        let mut pkrb_numerator: f64 = 0.0f64;

        for i in 0..k {
            pkrb_numerator += Self::get_quetra_formula_x_i(i, r, b);
            // println!("pkrb_numerator: {}", pkrb_numerator);
        }

        pkrb_numerator / (1.0f64 + ((b / r) * Self::get_quetra_formula_x_i(k - 1, r, b)))
    }
}

impl RateAdapter for Quetra {
    fn select_quality(
        &self,
        buffer_occupancy: u64,
        network_throughput: f64,
        available_bitrates: &[f64],
    ) -> usize {
        let mut result: usize = 0;
        let mut min_diff_with_buffer_occupancy = f64::MAX;

        // for each r_i inside r_vec, calculate its pkrb value (pkrb_r_i)
        // then replace result with r_i if its pkrb value has a smaller difference with buffer occupancy Bt (pkrb_r_i - Bt)
        for (i, r) in available_bitrates.iter().enumerate() {
            let pkrb_r_i = Self::get_quetra_formula_pkrb(buffer_occupancy, *r, network_throughput);
            if (pkrb_r_i - buffer_occupancy as f64) < min_diff_with_buffer_occupancy {
                result = i;
                min_diff_with_buffer_occupancy = pkrb_r_i - buffer_occupancy as f64;
            }
        }

        result
    }
}
