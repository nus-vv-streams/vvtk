use std::cell::RefCell;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
/**
 * Network trace for vvplay_async
 */
pub struct NetworkTrace {
    data: Vec<f64>,
    index: RefCell<usize>,
}

impl NetworkTrace {
    /// The network trace file to contain the network bandwidth in Kbps, each line representing 1 bandwidth sample.
    /// # Arguments
    ///
    /// * `path` - The path to the network trace file.
    pub fn new(path: &Path) -> Self {
        use std::io::BufRead;

        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let data = reader
            .lines()
            .map(|line| line.unwrap().trim().parse::<f64>().unwrap())
            .collect();
        NetworkTrace {
            data,
            index: RefCell::new(0),
        }
    }

    // Get the next bandwidth sample
    pub fn next(&self) -> f64 {
        let idx = *self.index.borrow();
        let next_idx = (idx + 1) % self.data.len();
        *self.index.borrow_mut() = next_idx;
        self.data[idx]
    }
}
