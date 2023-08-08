pub mod quetra;

pub trait RateAdapter: Send {
    /// Selects the bitrate to be used for the next segment download
    /// based on the current buffer occupancy and network throughput.
    /// Returns the index of the selected bitrate in the available_bitrates for each view.
    ///
    /// # Arguments
    ///
    /// * `buffer_occupancy` - the current buffer occupancy in seconds of playback
    /// * `network_throughput` - the current network throughput in Kbps
    /// * `available_bitrates` - the vector of available bitrates. The inner vector supplies bitrate information (in bps) for each view
    /// * `cosines` - the vector of cosines between the camera to each views. Not used in single-view ABR algorithms
    fn select_quality(
        &self,
        buffer_occupancy: u64,
        network_throughput: f64,
        available_bitrates: &[Vec<u64>],
        cosines: &[f32],
    ) -> Vec<usize>;
}

/// Multiple-Choice Knapsack Problem
pub struct MCKP {
    /// v: number of views
    v: usize,
    // acts as value/profit in knapsack problem
    qualities: Vec<f32>,
}

impl MCKP {
    pub fn new(v: usize, qualities: Vec<f32>) -> Self {
        MCKP { v, qualities }
    }

    fn select_quality_helper(
        &self,
        views_left: usize,
        network_throughput: f64,
        available_bitrates: &[Vec<u64>],
        cosines: &[f32],
        quality: f32,
        qualities_chosen: &mut Vec<usize>,
    ) -> (f32, Vec<usize>) {
        if network_throughput < 1e-4 {
            return (f32::MIN, vec![]);
        }

        if views_left == 0 {
            return (quality, qualities_chosen.iter().rev().cloned().collect());
        }

        let mut result = (0.0, vec![0; available_bitrates.len()]);
        for (i, r) in available_bitrates[views_left - 1].iter().enumerate() {
            qualities_chosen.push(i);
            let (q, chosen) = self.select_quality_helper(
                views_left - 1,
                network_throughput - *r as f64,
                available_bitrates,
                cosines,
                // 0.2588 ~ cos(75), i.e. if the view is > 75 degrees, we assume that it's hard to see it
                // and thus cosines[views_left - 1] will be positive and will always get the lowest quality
                quality - self.qualities[i] * (cosines[views_left - 1] - 0.2588),
                qualities_chosen,
            );

            if result.0 < q {
                result = (q, chosen);
            }
            qualities_chosen.pop();
        }
        result
    }
}

impl RateAdapter for MCKP {
    /// Maximize sum of -q_i * cos(theta_i) * x_i
    /// subject to sum of b_i * x_i <= network bandwidth and sum of x_i = 1
    fn select_quality(
        &self,
        _buffer_occupancy: u64,
        network_throughput: f64,
        available_bitrates: &[Vec<u64>],
        cosines: &[f32],
    ) -> Vec<usize> {
        let mut v = vec![];
        let (_quality, qualities_chosen) = self.select_quality_helper(
            self.v,
            network_throughput,
            available_bitrates,
            cosines,
            0.0,
            &mut v,
        );
        qualities_chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mckp_select_quality() {
        let mckp = MCKP::new(6, vec![1.72, 2.69, 3.61, 4.26, 4.47, 4.5]);
        let bitrate_sum = |qualities_chosen: &[usize], available_bitrates: &[Vec<u64>]| {
            qualities_chosen
                .iter()
                .zip(available_bitrates)
                .map(|(i, v)| v[*i])
                .sum::<u64>() as f64
        };
        let available_bitrates = vec![
            vec![133, 182, 323, 607, 990],
            vec![45, 45, 65, 96, 89],
            vec![122, 179, 317, 582, 896],
            vec![128, 179, 311, 572, 961],
            vec![37, 39, 54, 86, 83],
            vec![125, 192, 347, 653, 931],
        ];
        let qualities = mckp.select_quality(
            0,
            750.0,
            &available_bitrates,
            &[0.88, 0.17, 0.44, -0.94, 0.25, -0.17],
        );
        assert_eq!(qualities, vec![0, 2, 0, 1, 2, 1]);
        assert!(bitrate_sum(&qualities, &available_bitrates) <= 750.0);

        let qualities = mckp.select_quality(
            0,
            1000.0,
            &available_bitrates,
            &[0.88, 0.17, 0.44, -0.94, 0.25, -0.17],
        );
        assert_eq!(qualities, vec![0, 1, 0, 2, 1, 2]);
        assert!(bitrate_sum(&qualities, &available_bitrates) <= 1000.0);

        let qualities = mckp.select_quality(
            0,
            1500.0,
            &available_bitrates,
            &[0.88, 0.17, 0.44, -0.94, 0.25, -0.17],
        );
        assert_eq!(qualities, vec![0, 4, 0, 3, 4, 2]);
        assert!(bitrate_sum(&qualities, &available_bitrates) <= 1500.0);

        assert_eq!(
            mckp.select_quality(
                0,
                500.0,
                &available_bitrates,
                &[-0.18, 0.82, 0.53, 0.96, -0.20, 0.14]
            ),
            vec![0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            mckp.select_quality(
                0,
                1000.0,
                &available_bitrates,
                &[-0.18, 0.82, 0.53, 0.96, -0.20, 0.14]
            ),
            vec![2, 0, 0, 0, 4, 1]
        );
        assert_eq!(
            mckp.select_quality(
                0,
                1500.0,
                &available_bitrates,
                &[-0.18, 0.82, 0.53, 0.96, -0.20, 0.14]
            ),
            vec![3, 0, 0, 0, 4, 2]
        );
    }
}
