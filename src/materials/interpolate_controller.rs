use crate::params::Params;
use crate::point::Point;
use crate::points::*;
use std::sync::*;
use std::thread;

/// Spawns a single thread to compute the next "chunk" of closest points
pub fn setup_run_indiv_thread_closest_points(
    tx: mpsc::Sender<(Vec<Point>, Vec<Point>)>,
    slices: &mut std::slice::Chunks<Point>,
    kd_tree: Arc<kiddo::KdTree<f32, usize, 3_usize>>,
    options_for_nearest: usize,
    next_points: Arc<Points>,
    params: Arc<Params>,
    reference_frame: &mut Vec<Point>,
) -> std::thread::JoinHandle<()> {
    // let kd = kd_tree.clone();
    let slice = slices.next().unwrap().to_owned();

    // let now = Instant::now();
    let mut refer = reference_frame.clone();
    // println!("cloning time: {}", now.elapsed().as_millis());

    // let now = Instant::now();
    thread::spawn(move || {
        // let kd_arc_clone = kd_tree.clone();
        // let next_points_clone = next_points.clone();
        // let params_clone = params.clone();
        // let mut nearests: Vec<usize> = Vec::new();
        let mut closests: Vec<Point> = Vec::with_capacity(100);
        for s in &slice {
            let nearests =
                s.method_of_neighbour_query(&kd_tree, options_for_nearest, params.density_radius);
            let p = s.get_average_closest(&next_points, &nearests, &mut refer, &params);
            closests.push(p);
        }
        // println!("time for 1 thread to finish: {}", now.elapsed().as_millis());
        tx.send((closests, refer)).unwrap();
    })
}

/// Iteratively spawns threads to perform interpolation.
pub fn run_threads(
    threads: usize,
    slices: &mut std::slice::Chunks<Point>,
    kd_tree: &Arc<kiddo::KdTree<f32, usize, 3_usize>>,
    options_for_nearest: usize,
    next_points: Arc<Points>,
    params: &Arc<Params>,
    reference_frame: &mut Vec<Point>,
) -> Vec<Point> {
    let mut vrx: Vec<mpsc::Receiver<(Vec<Point>, Vec<Point>)>> = Vec::with_capacity(12);
    let mut vhandle: Vec<std::thread::JoinHandle<()>> = Vec::with_capacity(12);

    // let now = Instant::now();

    for _i in 0..threads {
        let (tx, rx): (
            mpsc::Sender<(Vec<Point>, Vec<Point>)>,
            mpsc::Receiver<(Vec<Point>, Vec<Point>)>,
        ) = mpsc::channel();
        vrx.push(rx);
        let handle = setup_run_indiv_thread_closest_points(
            tx,
            slices,
            kd_tree.clone(),
            options_for_nearest,
            next_points.clone(),
            params.clone(),
            reference_frame,
        );
        vhandle.push(handle);
    }

    for handle in vhandle {
        handle.join().unwrap();
    }

    let mut result: Vec<Point> = Vec::with_capacity(100000);

    for rx in vrx {
        let res = rx.recv().unwrap();
        result.extend(res.0);

        // for i in 0..reference_frame.len() {
        //     if reference_frame[i].mapping == 0 && res.1[i].mapping > 0 {
        //         reference_frame[i].mapping = res.1[i].mapping;
        //     }
        // }

        for (i, item) in reference_frame.iter_mut().enumerate() {
            if item.mapping == 0 && res.1[i].mapping > 0 {
                item.mapping = res.1[i].mapping;
            }
        }
    }
    result
}

/// Wrapper function for run_threads(). Slices the given Points into t chunks where t is the number of threads to be used.
pub fn parallel_query_closests(
    data_copy: &[Point],
    kd_tree: &Arc<kiddo::KdTree<f32, usize, 3_usize>>,
    next_points: Arc<Points>,
    params: &Arc<Params>,
    reference_frame: &mut Vec<Point>,
) -> Vec<Point> {
    let mut slices = data_copy.chunks((data_copy.len() as f32 / params.threads as f32).ceil() as usize);

    run_threads(
        params.threads,
        &mut slices,
        kd_tree,
        params.options_for_nearest,
        next_points,
        params,
        reference_frame,
    )
}
