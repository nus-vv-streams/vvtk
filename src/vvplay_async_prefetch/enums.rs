/**
 * This file contains all the enums that is used by vvplay_async_prefetch.rs
 */


//Noop for operation that will not use a decoder
#[derive(clap::ValueEnum, Clone, Copy)]
pub enum DecoderType {
    Noop,
    Draco,
    Tmc2rs,
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum AbrType {
    Quetra,
    QuetraMultiview,
    Mckp,
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum ThroughputPredictionType {
    /// Last throughput
    Last,
    /// Average of last 3 throughput
    Avg,
    /// ExponentialMovingAverage,
    Ema,
    /// Gradient Adaptive Exponential Moving Average
    Gaema,
    /// Low Pass Exponential Moving Average
    Lpema,
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum ViewportPredictionType {
    /// Last viewport
    Last,
}