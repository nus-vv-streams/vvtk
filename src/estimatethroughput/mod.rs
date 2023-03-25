// estimate throughput from the 'n' last throughputs
pub fn estimate_throughput_avgtp(past_tp: Vec<f64>, n: i32) -> f64 {
    let mut result: f64 = 0.0;
    for item in past_tp.iter().skip(past_tp.len() - n as usize) {
        result += item;
    }
    result / n as f64
}

// estimate throughput from using an exponential moving average (EMA)
// QUETRA set alpha to 0.1 by default but can change it in the parameter
pub fn estimate_throughput_ema(past_tp: Vec<f64>, alpha: f64, b_last_predicted: f64) -> f64 {
    (1.0 - alpha) * b_last_predicted + alpha * past_tp[past_tp.len() - 1]
}

// estimate throughput from using a gradient adaptive EMA (GAEMA)
// QUETRA initialized alpha_0 to 0.1 by default
pub fn estimate_throughput_gaema(past_tp: Vec<f64>, alpha_last: f64, b_last_predicted: f64) -> f64 {
    let m_inst_i: f64 = (past_tp[past_tp.len() - 1] - past_tp[past_tp.len() - 2]).abs();
    let m_norm_i: f64 = past_tp.iter().sum::<f64>() / (past_tp.len().pow(2)) as f64;

    let alpha: f64 = alpha_last.powf(m_norm_i / m_inst_i);

    (1.0 - alpha) * b_last_predicted + alpha * past_tp[past_tp.len() - 1]
}

// estimate throughput from using low pass EMA (LPEMA)
// QUETRA initialized alpha_0 to 0.1 by default
pub fn estimate_throughput_lpema(past_tp: Vec<f64>, b_last_predicted: f64) -> f64 {
    let m_inst_i: f64 = (past_tp[past_tp.len() - 1] - past_tp[past_tp.len() - 2]).abs();
    let m_norm_i: f64 = past_tp.iter().sum::<f64>() / (past_tp.len().pow(2)) as f64;

    let alpha: f64 = 1.0 / (1.0 + (m_inst_i / m_norm_i));

    (1.0 - alpha) * b_last_predicted + alpha * past_tp[past_tp.len() - 1]
}

// estimate throughput from using Kaufman's Adaptive Moving Average (KAMA)
// uses a smoothing constant in a moving window of the last 10 values
// window_size reduced to len of past predicated values if < 10
pub fn estimate_throughput_kama(past_tp: Vec<f64>, past_predictions: Vec<f64>) -> f64 {
    let mut past_predictions_copy = past_predictions.clone();
    let window_size = std::cmp::min(10, past_predictions.len());

    if window_size == 0 || window_size == 1 {
        past_predictions_copy = vec![0.0, 0.0];
    }

    let mut x = window_size - 1;

    let numer = (past_tp[past_tp.len() - 1] - past_tp[past_tp.len() - window_size]).abs();
    let mut denom = 0.0;

    // while loop for when x is between 0 and window_size
    while x <= window_size && x > 0 {
        denom += (past_tp[x] - past_tp[x - 1]).abs();
        x -= 1;
    }

    let e_i = numer / denom;
    let sc_i: f64 = (e_i * ((2.0 / 3.0) - (2.0 / 31.0)) + (2.0 / 31.0)).powf(2.0);

    past_predictions_copy[past_predictions_copy.len() - 1]
        + sc_i
            * (past_tp[past_tp.len() - 1] - past_predictions_copy[past_predictions_copy.len() - 1])
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
    fn test_estimate_throughput_kama() {
        let past_tp = vec![15.0, 20.0, 110.0, 60.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
        let past_predictions = vec![
            15.0,
            15.0,
            17.22222222,
            58.45679012,
            58.55431659,
            58.2104803,
            58.30411071,
            59.05727784,
            60.65356491,
            63.22673683,
        ];
        let result = estimate_throughput_kama(past_tp, past_predictions);
        assert!((result - 66.85678339).abs() < EPSILON);
    }
}
