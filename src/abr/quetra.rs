use cgmath::num_traits::Pow;
use std::f64::consts::E;

use super::RateAdapter;

/// Implementation of the Quetra algorithm.
///
/// See [Quetra: A Queueing Theory Approach to DASH Rate Adaptation](https://www.comp.nus.edu.sg/~ooiwt/papers/mm17-quetra.pdf)
pub struct Quetra {
    /// max buffer capacity measured in seconds of playback
    pub k: u64,
    /// how often is the bitrate adaptation done (selected as 1s for now)
    segment_frequency: u32,
    /// segment size param for granularity of each buffer (selected as 1 for now)
    segment_size: u32,
}

impl Quetra {
    pub fn new(buffer_capacity: u64) -> Self {
        Quetra {
            k: buffer_capacity,
            segment_frequency: 1,
            segment_size: 1,
        }
    }

    /// Get the x_i value for the given i, r and b
    fn x_i(i: u64, r: f64, b: f64) -> f64 {
        let mut x_i: f64 = 0.0f64;

        // (i - j).pow(j) / j!
        let first_term = |i: u64, j: u64| -> f64 {
            let mut result: f64 = 1.0f64;

            for k in 1..=j {
                result *= ((i - j) as f64) / k as f64;
            }

            result
        };

        for j in 0..=i {
            let x = first_term(i, j)
                * (b / r).powi(j.try_into().unwrap())
                * E.pow((i - j) as f64 * (b / r));
            if j % 2 == 0 {
                x_i += x;
            } else {
                x_i -= x;
            }
        }

        x_i
    }

    /// Get the buffer slack `P_{K,r,b}` value for the given k, r and b
    ///
    /// # Arguments
    ///
    /// * `k` - max buffer capacity measured in seconds of playback
    /// * `r` - downloaded segment bitrate
    /// * `b` - network throughput
    fn buffer_slack(k: u64, r: f64, b: f64) -> f64 {
        let mut pkrb: f64 = 0.0f64;
        let denominator = 1.0f64 + ((b / r) * Self::x_i(k - 1, r, b));

        for i in 0..k {
            pkrb += Self::x_i(i, r, b) / denominator;
        }

        pkrb
    }
}

impl RateAdapter for Quetra {
    fn select_quality(
        &self,
        buffer_occupancy: u64,
        network_throughput: f64,
        available_bitrates: &[u64],
    ) -> usize {
        let mut result: usize = 0;
        let mut min_diff_with_buffer_occupancy = f64::MAX;

        // Find a rate r_i where the buffer slack value (P_krb) has the smallest difference with Bt
        // In other words, we are looking for a rate that keeps the buffer occupancy at half-full.
        for (i, r) in available_bitrates.iter().enumerate() {
            let pkrb_r_i = Self::buffer_slack(self.k, *r as f64, network_throughput);
            let diff = (pkrb_r_i - buffer_occupancy as f64).abs();
            if diff < min_diff_with_buffer_occupancy {
                result = i;
                min_diff_with_buffer_occupancy = diff;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_quetra_formula_pkrb() {
        const EPSILON: f64 = 1.0e-8;
        assert!((Quetra::buffer_slack(2, 100.0, 500.0) - 0.20107662).abs() < EPSILON);
        assert!((Quetra::buffer_slack(3, 100.0, 300.0) - 0.353471).abs() < EPSILON);
        assert!((Quetra::buffer_slack(4, 100.0, 300.0) - 0.35434551).abs() < EPSILON);
        assert!((Quetra::buffer_slack(3, 125.0, 400.0) - 0.32755053).abs() < EPSILON);
        assert!((Quetra::buffer_slack(4, 125.0, 90.0) - 2.74465).abs() < EPSILON);
        assert!((Quetra::buffer_slack(4, 150.0, 70.0) - 3.34906349).abs() < EPSILON);
    }

    #[test]
    fn test_select_quality() {
        let quetra = Quetra::new(10);
        let available_bitrates = [100, 200, 300, 400, 500, 600, 700, 800, 900, 1000];
        assert_eq!(quetra.select_quality(0, 535.0, &available_bitrates), 0);
        assert_eq!(quetra.select_quality(1, 535.0, &available_bitrates), 2);
        assert_eq!(quetra.select_quality(3, 535.0, &available_bitrates), 4);
        assert_eq!(quetra.select_quality(6, 535.0, &available_bitrates), 5);
        assert_eq!(quetra.select_quality(9, 535.0, &available_bitrates), 8);
    }
}
