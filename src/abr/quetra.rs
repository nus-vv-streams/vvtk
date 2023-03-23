use cgmath::num_traits::Pow;
use std::f64::consts::E;

use super::RateAdapter;

/// Implementation of the Quetra algorithm.
///
/// See [Quetra: A Queueing Theory Approach to DASH Rate Adaptation](https://www.comp.nus.edu.sg/~ooiwt/papers/mm17-quetra.pdf)
pub struct Quetra {
    /// max buffer capacity measured in seconds of playback
    pub k: u64,
    /// playback speed: how many seconds of video is consumed and played in 1 second
    p: f64,
    /// how often is the bitrate adaptation done (selected as 1s for now)
    segment_frequency: u32,
    /// segment size param for granularity of each buffer (selected as 1 for now)
    segment_size: u32,
}

impl Quetra {
    pub fn new(buffer_capacity: u64, fps: f32) -> Self {
        Quetra {
            k: buffer_capacity,
            segment_frequency: 1,
            segment_size: 1,
            p: fps as f64 / 30.0,
        }
    }

    /// Get the x_i value for the given i, r and b
    fn x_i(&self, i: u64, r: f64, b: f64) -> f64 {
        let mut x_i: f64 = 0.0f64;

        // (i - j).pow(j) / j!
        let first_term = |i: u64, j: u64| -> f64 {
            let mut result: f64 = 1.0f64;

            for k in 1..=j {
                result *= ((i - j) as f64) / k as f64;
            }

            result
        };

        // rho is utilization: lambda / mu
        // lambda is arrival rate of segment: b / rd (b: throughput, r: bitrate, d: segment duration)
        // mu is service rate: p / d (p: playback rate, d: segment duration)
        let rho = b / (self.p * r);
        for j in 0..=i {
            let x =
                first_term(i, j) * rho.powi(j.try_into().unwrap()) * E.pow((i - j) as f64 * rho);
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
    fn buffer_slack(&self, r: f64, b: f64) -> f64 {
        let mut pkrb: f64 = 0.0f64;

        let denominator = 1.0f64 + ((b / (r * self.p)) * self.x_i(self.k - 1, r, b));
        for i in 0..self.k {
            pkrb += self.x_i(i, r, b) / denominator;
        }

        pkrb
    }
}

impl RateAdapter for Quetra {
    fn select_quality(
        &self,
        buffer_occupancy: u64,
        network_throughput: f64,
        available_bitrates: &[Vec<u64>],
        _cosines: &[f32],
    ) -> Vec<usize> {
        let mut result: usize = 0;
        let mut min_diff_with_buffer_occupancy = f64::MAX;

        // Find a rate r_i where the buffer slack value (P_krb) has the smallest difference with Bt
        // In other words, we are looking for a rate that keeps the buffer occupancy at half-full.
        for (i, r) in available_bitrates[0].iter().enumerate() {
            let pkrb_r_i = self.buffer_slack(*r as f64, network_throughput);
            let diff = (pkrb_r_i - buffer_occupancy as f64).abs();
            if diff < min_diff_with_buffer_occupancy {
                result = i;
                min_diff_with_buffer_occupancy = diff;
            }
        }

        vec![result]
    }
}

/// An adaptation of Quetra to support multiview video.
pub struct QuetraMultiview {
    mckp: super::MCKP,
    quetra: Quetra,
    /// number of views
    v: usize,
}

impl QuetraMultiview {
    /// Create a new QuetraMultiview instance.
    ///
    /// # Arguments
    /// - `v` - number of views
    /// - `fps` - playback speed: how many frames is played in 1 second
    /// - `buffer_capacity` - max buffer capacity measured in seconds of playback
    /// - `qualities` - qualities vector of the video
    pub fn new(buffer_capacity: u64, fps: f32, v: usize, qualities: Vec<f32>) -> Self {
        QuetraMultiview {
            mckp: super::MCKP::new(v, qualities),
            quetra: Quetra::new(buffer_capacity, fps),
            v,
        }
    }
}

impl RateAdapter for QuetraMultiview {
    fn select_quality(
        &self,
        buffer_occupancy: u64,
        network_throughput: f64,
        available_bitrates: &[Vec<u64>],
        cosines: &[f32],
    ) -> Vec<usize> {
        let mut combined_bitrates = available_bitrates[0].clone();
        for i in 1..self.v {
            for j in 0..combined_bitrates.len() {
                combined_bitrates[j] += available_bitrates[i][j];
            }
        }
        // Based on the network throughput and buffer occupancy, Quetra gives us the quality to download
        let quality = self.quetra.select_quality(
            buffer_occupancy,
            network_throughput,
            &[combined_bitrates],
            cosines,
        )[0];
        // this is the total bits that Quetra suggested us to download based on the network throughput and buffer occupancy
        let target_bitrate: u64 = available_bitrates.iter().map(|v| v[quality]).sum();
        // now we use MCKP to decide how to distribute the bits among the views
        self.mckp.select_quality(
            buffer_occupancy,
            target_bitrate as f64,
            available_bitrates,
            cosines,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quetra_buffer_slack() {
        const EPSILON: f64 = 1.0e-8;
        assert!((Quetra::new(2, 30.0).buffer_slack(100.0, 500.0) - 0.20107662).abs() < EPSILON);
        assert!((Quetra::new(3, 30.0).buffer_slack(100.0, 300.0) - 0.353471).abs() < EPSILON);
        assert!((Quetra::new(4, 30.0).buffer_slack(100.0, 300.0) - 0.35434551).abs() < EPSILON);
        assert!((Quetra::new(3, 30.0).buffer_slack(125.0, 400.0) - 0.32755053).abs() < EPSILON);
        assert!((Quetra::new(4, 30.0).buffer_slack(125.0, 90.0) - 2.74465).abs() < EPSILON);
        assert!((Quetra::new(4, 30.0).buffer_slack(150.0, 70.0) - 3.34906349).abs() < EPSILON);
    }

    #[test]
    fn test_quetra_select_quality() {
        let quetra = Quetra::new(10, 30.0);
        let available_bitrates = [vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]];
        assert_eq!(
            quetra.select_quality(0, 535.0, &available_bitrates, &[])[0],
            0
        );
        assert_eq!(
            quetra.select_quality(1, 535.0, &available_bitrates, &[])[0],
            2
        );
        assert_eq!(
            quetra.select_quality(3, 535.0, &available_bitrates, &[])[0],
            4
        );
        assert_eq!(
            quetra.select_quality(6, 535.0, &available_bitrates, &[])[0],
            5
        );
        assert_eq!(
            quetra.select_quality(9, 535.0, &available_bitrates, &[])[0],
            8
        );
    }
}
