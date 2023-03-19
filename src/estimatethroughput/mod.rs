// estimate throughput from the 'n' last throughputs
pub fn estimate_throughput_avgtp(past_tp: Vec<f64>, n: i32) -> f64 {
    let mut result: f64 = 0.0;
    for i in (past_tp.len() - n as usize)..past_tp.len() {
        result += past_tp[i];
    }
    return result / n as f64;
}

// estimate throughput from using an exponential moving average (EMA)
// QUETRA set alpha to 0.1 by default but can change it in the parameter
pub fn estimate_throughput_ema(past_tp: Vec<f64>, alpha: f64, b_last_predicted: f64) -> f64 {
    let result = (1.0 - alpha) * b_last_predicted + alpha * past_tp[past_tp.len() - 1];
    return result;
}

// estimate throughput from using a gradient adaptive EMA (GAEMA)
// QUETRA initialized alpha to 0.1 by default but can change it in the parameter
pub fn estimate_throughput_gaema(past_tp: Vec<f64>, alpha_last: f64, b_last_predicted: f64) -> f64 {
    let m_inst_i: f64 = (past_tp[past_tp.len() - 1] - past_tp[past_tp.len() - 2]).abs();
    let m_norm_i: f64 = past_tp.iter().sum::<f64>() / (past_tp.len() ^ 2) as f64;

    let alpha: f64 = alpha_last.powf(m_norm_i / m_inst_i);
    let result = (1.0 - alpha) * b_last_predicted + alpha * past_tp[past_tp.len() - 1];
    return result;
}

// estimate throughput from using low pass EMA (LPEMA)
// QUETRA initialized alpha to 0.1 by default but can change it in the parameter
pub fn estimate_throughput_lpema(past_tp: Vec<f64>, b_last_predicted: f64) -> f64 {
    let m_inst_i: f64 = (past_tp[past_tp.len() - 1] - past_tp[past_tp.len() - 2]).abs();
    let m_norm_i: f64 = past_tp.iter().sum::<f64>() / (past_tp.len() ^ 2) as f64;

    let alpha: f64 = 1.0 / (1.0 + (m_inst_i / m_norm_i));
    let result = (1.0 - alpha) * b_last_predicted + alpha * past_tp[past_tp.len() - 1];
    return result;
}

// estimate throughput from using Kaufman's Adaptive Moving Average (KAMA)
// TODO: implement KAMA
