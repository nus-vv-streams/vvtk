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
// QUETRA initialized alpha_0 to 0.1 by default
pub fn estimate_throughput_gaema(past_tp: Vec<f64>, alpha_last: f64, b_last_predicted: f64) -> f64 {
    let m_inst_i: f64 = (past_tp[past_tp.len() - 1] - past_tp[past_tp.len() - 2]).abs();
    let m_norm_i: f64 = past_tp.iter().sum::<f64>() / (past_tp.len().pow(2)) as f64;

    let alpha: f64 = alpha_last.powf(m_norm_i / m_inst_i);
    let result = (1.0 - alpha) * b_last_predicted + alpha * past_tp[past_tp.len() - 1];
    return result;
}

// estimate throughput from using low pass EMA (LPEMA)
// QUETRA initialized alpha_0 to 0.1 by default
pub fn estimate_throughput_lpema(past_tp: Vec<f64>, b_last_predicted: f64) -> f64 {
    let m_inst_i: f64 = (past_tp[past_tp.len() - 1] - past_tp[past_tp.len() - 2]).abs();
    let m_norm_i: f64 = past_tp.iter().sum::<f64>() / (past_tp.len().pow(2)) as f64;

    let alpha: f64 = 1.0 / (1.0 + (m_inst_i / m_norm_i));
    let result = (1.0 - alpha) * b_last_predicted + alpha * past_tp[past_tp.len() - 1];
    return result;
}

// estimate throughput from using Kaufman's Adaptive Moving Average (KAMA)
// uses a smoothing constant in a moving window of the last 10 values
// window_size reduced to len of past predicated values if < 10
pub fn estimate_throughput_kama(past_tp: Vec<f64>, past_predictions: Vec<f64>) -> f64 {
    let window_size = std::cmp::min(10, past_predictions.len());

    let numer = (past_predictions[past_predictions.len() - 1]
        - past_predictions[past_predictions.len() - window_size])
        .abs();
    let mut denom = 0.0;
    for x in past_predictions.len() - window_size..past_predictions.len() {
        denom += (past_predictions[x] - past_predictions[x - 1]).abs();
    }
    let e_i = numer / denom;
    let sc_i: f64 = (e_i * ((2 / 3) as f64 - (2 / 31) as f64) + (2 / 31) as f64).powf(2.0);
    let result = past_predictions[past_predictions.len() - 1]
        + sc_i * (past_tp[past_tp.len() - 1] - past_predictions[past_predictions.len() - 1]);
    return result;
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPSILON: f64 = 1.0e-8;

    #[test]
    fn test_estimate_throughput_avgtp() {
        let past_tp = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let n = 3;
        let result = estimate_throughput_avgtp(past_tp, n);
        assert_eq!(result, 4.0);
    }

    #[test]
    fn test_estimate_throughput_ema() {
        let past_tp = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let alpha = 0.1;
        let b_last_predicted = 4.0;
        let result = estimate_throughput_ema(past_tp, alpha, b_last_predicted);
        assert_eq!(result, 4.1);
    }

    #[test]
    fn test_estimate_throughput_gaema() {
        let past_tp = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let alpha_last = 0.1;
        let b_last_predicted = 4.0;
        let result = estimate_throughput_gaema(past_tp, alpha_last, b_last_predicted);
        assert!((result - 4.251188643).abs() < EPSILON);
    }

    #[test]
    fn test_estimate_throughput_lpema() {
        let past_tp = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b_last_predicted = 4.0;
        let result = estimate_throughput_lpema(past_tp, b_last_predicted);
        assert!((result - 4.375).abs() < EPSILON);
    }

    #[test]
    fn test_estimate_throughput_kama() {}
}
