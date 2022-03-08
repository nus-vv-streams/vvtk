use crate::point::Point;
use crate::pointcloud::PointCloud;
use crate::processing::conceal::interpolate_params::InterpolateParams;
use kiddo::KdTree;
use std::slice::Chunks;
use std::sync::*;
use std::thread;
use std::thread::JoinHandle;

#[allow(dead_code)]
type PairVecPoint = (Vec<Point>, Vec<Point>);

/// Spawns a single thread to compute the next "chunk" of closest points
fn spawn_thread_to_interpolate(
    tx: mpsc::Sender<Vec<Point>>,
    slices: &mut Chunks<Point>,
    kd_tree: Arc<KdTree<f32, usize, 3>>,
    points: Arc<PointCloud>,
    params: Arc<InterpolateParams>,
) -> JoinHandle<()> {
    let slice = slices.next().unwrap().to_owned();

    thread::spawn(move || {
        let mut interpolated_points: Vec<Point> = Vec::with_capacity(100);
        let size = params.neighborhood_size;
        for s in &slice {
            let neighbourhood: Vec<usize> = kd_tree
                .nearest(&s.get_point(), size, &params.dist_func)
                .unwrap()
                .into_iter()
                .map(|found| *found.1)
                .collect();
            let p = s.interpolate_with_closest(&points, &neighbourhood, &params);
            interpolated_points.push(p);
        }
        tx.send(interpolated_points).unwrap();
    })
}

/// Iteratively spawns threads to perform interpolation.
pub fn interpolate_in_parallel(
    threads: usize,
    slices: &mut std::slice::Chunks<Point>,
    kd_tree: Arc<kiddo::KdTree<f32, usize, 3>>,
    next_points: Arc<PointCloud>,
    params: Arc<InterpolateParams>,
) -> Vec<Point> {
    let mut vrx: Vec<mpsc::Receiver<Vec<Point>>> = Vec::with_capacity(12);
    let mut vhandle: Vec<std::thread::JoinHandle<()>> = Vec::with_capacity(12);

    // let now = Instant::now();

    for _i in 0..threads {
        let (tx, rx): (mpsc::Sender<Vec<Point>>, mpsc::Receiver<Vec<Point>>) = mpsc::channel();
        vrx.push(rx);
        let handle = spawn_thread_to_interpolate(
            tx,
            slices,
            kd_tree.clone(),
            next_points.clone(),
            params.clone(),
        );
        vhandle.push(handle);
    }

    for handle in vhandle {
        handle.join().unwrap();
    }

    let mut result: Vec<Point> = Vec::with_capacity(100000);

    for rx in vrx {
        let res = rx.recv().unwrap();
        result.extend(res);
    }
    result
}

/// Returns dimension of kdtree in use for interpolation
pub const fn kdtree_dim() -> usize {
    3_usize
}

#[cfg(feature = "dim_6")]
/// Returns dimension of kdtree in use for interpolation
pub const fn kdtree_dim() -> usize {
    6_usize
}

/*
/// input: an array of target_points.  For each point, look for the
/// closest point in search_points.
pub fn find_closest(
    targets: &[Point],
    kd_tree: &Arc<kiddo::KdTree<f32, usize, 3>>,
    references: &Arc<PointCloud>,
    params: &Arc<InterpolateParams>,
) -> Vec<Point> {
    let mut slices =
        targets.chunks((targets.len() as f32 / params.threads as f32).ceil() as usize);

    run_threads(
        params.threads,
        &mut slices,
        kd_tree,
        references,
        params,
    )
}
*/
