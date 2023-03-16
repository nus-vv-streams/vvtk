use cgmath::num_traits::Pow;
use std::f64::consts::E;

use crate::abr::MCKP;

use super::RateAdapter;

/// Implementation of the Quetra algorithm for Multiplane Decoder.
///
/// See [Quetra: A Queueing Theory Approach to DASH Rate Adaptation](https://www.comp.nus.edu.sg/~ooiwt/papers/mm17-quetra.pdf)
pub struct QuetraMulti {
    /// max buffer capacity measured in seconds of playback
    pub k: u64,
    /// how often is the bitrate adaptation done (selected as 1s for now)
    segment_frequency: u32,
    /// segment size param for granularity of each buffer (selected as 1 for now)
    segment_size: u32,
}

impl QuetraMulti {
    pub fn new(buffer_capacity: u64) -> Self {
        QuetraMulti {
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
        // for large k (say k > 10), P_{K,r,b} can be approximated with K/2
        if k > 10 {
            pkrb = k as f64 / 2.0f64;
        } else {
            let denominator = 1.0f64 + ((b / r) * Self::x_i(k - 1, r, b));

            for i in 0..k {
                pkrb += Self::x_i(i, r, b) / denominator;
            }
        }

        pkrb
    }
}

impl RateAdapter for QuetraMulti {
    // the selected qualities vectors must follow the same order as the qualities in the vector of vectors
    fn select_quality(
        &self,
        buffer_occupancy: u64,
        network_throughput: f64,
        available_bitrates: &[Vec<u64>],
        cosines: &[f32],
    ) -> Vec<usize> {
        let mut results: Vec<usize> = vec![];
        let mut min_diff_with_buffer_occupancy = f64::MAX;

        // first use MCKP to get a specific set of qualities for each plane
        let mckp = MCKP::new(6, vec![1.72, 2.69, 3.61, 4.26, 4.47, 4.5]);
        let selected_qualities = mckp.select_quality(
            buffer_occupancy,
            network_throughput,
            available_bitrates,
            cosines,
        );

        // after MCKP selects a combination of qualities, it has not yet taken into account the conditions of the buffer slack yet
        // we will then need to check for the buffer slack under this specific combination of qualities
        // this part could also be done in select_quality() of MCKP?
        let mut total_bitrate: u64 = 0;
        // for each i in selected_qualities, i is the element in available_bitrates
        for plane in 0..selected_qualities.len() {
            total_bitrate += available_bitrates[plane][selected_qualities[plane]];
        }

        let pkrb_r_i = Self::buffer_slack(self.k, total_bitrate as f64, network_throughput);
        let diff = (pkrb_r_i - buffer_occupancy as f64).abs();
        if diff < min_diff_with_buffer_occupancy {
            results = selected_qualities.clone();
            min_diff_with_buffer_occupancy = diff;
        }

        // after buffer_slack is checked for original combination of qualities, check for improvements by swapping qualities to see if buffer slack is improved
        // aim to preserve higher quality for planes with the most negative cosines
        let mut cosines_sorted = cosines.to_vec();
        cosines_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let cosine_cutoff = cosines_sorted[3];

        for (plane, i) in selected_qualities.iter().enumerate() {
            if cosines[plane] < cosine_cutoff {
                continue;
            }
            for j in (0..available_bitrates[plane].len()).rev() {
                if j < *i {
                    let mut new_selected_qualities = selected_qualities.clone();
                    new_selected_qualities[plane] = j;
                    let mut new_total_bitrate: u64 = 0;
                    for (plane, i) in new_selected_qualities.iter().enumerate() {
                        new_total_bitrate += available_bitrates[plane][*i];
                    }
                    let new_pkrb_r_i =
                        Self::buffer_slack(self.k, new_total_bitrate as f64, network_throughput);
                    let diff = (new_pkrb_r_i - buffer_occupancy as f64).abs();
                    if diff < min_diff_with_buffer_occupancy {
                        results = new_selected_qualities;
                        min_diff_with_buffer_occupancy = diff;
                    }
                }
            }
        }

        results
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_quetra_buffer_slack() {
        const EPSILON: f64 = 1.0e-8;
        assert!((QuetraMulti::buffer_slack(2, 100.0, 500.0) - 0.20107662).abs() < EPSILON);
        assert!((QuetraMulti::buffer_slack(3, 100.0, 300.0) - 0.353471).abs() < EPSILON);
        assert!((QuetraMulti::buffer_slack(4, 100.0, 300.0) - 0.35434551).abs() < EPSILON);
        assert!((QuetraMulti::buffer_slack(3, 125.0, 400.0) - 0.32755053).abs() < EPSILON);
        assert!((QuetraMulti::buffer_slack(4, 125.0, 90.0) - 2.74465).abs() < EPSILON);
        assert!((QuetraMulti::buffer_slack(4, 150.0, 70.0) - 3.34906349).abs() < EPSILON);
    }

    #[test]
    fn test_quetra_multi_select_quality() {
        let quetra_multi = QuetraMulti::new(5);
        let available_bitrates = vec![
            vec![133, 182, 323, 607, 990],
            vec![45, 45, 65, 96, 89],
            vec![122, 179, 317, 582, 896],
            vec![128, 179, 311, 572, 961],
            vec![37, 39, 54, 86, 83],
            vec![125, 192, 347, 653, 931],
        ];
        let cosines = [0.88, 0.17, 0.44, -0.94, 0.25, -0.17];

        assert_eq!(
            quetra_multi.select_quality(0, 500.0, &available_bitrates, &cosines),
            vec![0, 2, 0, 0, 0, 0]
        );

        assert_eq!(
            quetra_multi.select_quality(3, 500.0, &available_bitrates, &cosines),
            vec![0, 2, 0, 0, 0, 0]
        );

        assert_eq!(
            quetra_multi.select_quality(0, 750.0, &available_bitrates, &cosines),
            vec![0, 2, 0, 2, 0, 1]
        );

        assert_eq!(
            quetra_multi.select_quality(3, 750.0, &available_bitrates, &cosines),
            vec![0, 2, 0, 2, 1, 1]
        );

        assert_eq!(
            quetra_multi.select_quality(0, 1000.0, &available_bitrates, &cosines),
            vec![0, 2, 0, 3, 0, 1]
        );

        assert_eq!(
            quetra_multi.select_quality(3, 1000.0, &available_bitrates, &cosines),
            vec![0, 2, 0, 3, 1, 1]
        );

        assert_eq!(
            quetra_multi.select_quality(0, 1500.0, &available_bitrates, &cosines),
            vec![0, 4, 0, 3, 0, 3]
        );

        assert_eq!(
            quetra_multi.select_quality(3, 1500.0, &available_bitrates, &cosines),
            vec![0, 4, 0, 3, 2, 3]
        );
    }
}
