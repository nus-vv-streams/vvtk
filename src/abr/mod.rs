pub mod quetra;

pub trait RateAdapter {
    /// Selects the bitrate to be used for the next segment download
    /// based on the current buffer occupancy and network throughput.
    /// Returns the index of the selected bitrate in the available_bitrates.
    ///
    /// # Arguments
    ///
    /// * `buffer_occupancy` - the current buffer occupancy in seconds of playback
    /// * `network_throughput` - the current network throughput in Kbps
    /// * `available_bitrates` - the vector of available bitrates
    fn select_quality(
        &self,
        buffer_occupancy: u64,
        network_throughput: f64,
        available_bitrates: &[f64],
    ) -> usize;
}
