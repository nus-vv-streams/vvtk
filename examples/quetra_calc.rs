use cgmath::num_traits::Pow;
use clap::Parser;
use std::f64::consts::E;

// take binary files from input folder and a simulated network condition,
// then output binary files of varying qualities into output folder (should decoding be done here?)
#[derive(Parser)]
struct Args {
    // #[clap(parse(from_os_str))]
    k_input: i64,  // buffer capacity
    r_input: f64,  // bitrate of segment being downloaded
    b_input: f64,  // network throughput
    sf_input: i32, // segment frequency for bitrate adaptation
    ss_input: i32, // segment size param for granularity of each buffer
}

fn factorial(i: i64) -> i64 {
    assert!(i >= 0); // panics if i is negative

    if i == 0 {
        return 1; // factorial(0) = 1 by definition
    }

    i * factorial(i - 1)
}

fn get_quetra_formula_x_i(i: i64, r: f64, b: f64) -> f64 {
    let mut x_i: f64 = 0.0f64;
    let mut j: i64 = 0;

    while j < i + 1 {
        x_i += ((-1_i64).pow(j as u32) / factorial(j)) as f64
            * (i - j).pow(j as u32) as f64
            * (b / r).powi(j.try_into().unwrap())
            * E.pow((i - j) as f64 * (b / r));

        // * pow(i - j, j) // (i - j).pow(j)
        // * pow(b / r, j) // (b / r).pow(j)
        // * pow(E, (i - j) as f32 * (b / r)); // E.pow((i - j) * (b / r))

        j += 1;
    }

    x_i
}

fn get_quetra_formula_pkrb(k: i64, r: f64, b: f64) -> f64 {
    let mut i: i64 = 0;
    let mut pkrb_numerator: f64 = 0.0f64;

    while i < k {
        pkrb_numerator += get_quetra_formula_x_i(i, r, b);
        println!("pkrb_numerator: {pkrb_numerator}");
        i += 1;
    }

    pkrb_numerator / (1.0f64 + ((b / r) * get_quetra_formula_x_i(k - 1, r, b)))
}

fn main() {
    let args: Args = Args::parse();
    let k = args.k_input;
    let r = args.r_input;
    let b = args.b_input;

    // call for pkrb, temporary hardcoded values
    // let x_i: f64 = get_quetra_formula_x_i(1, r, b);
    let pkrb: f64 = get_quetra_formula_pkrb(k, r, b);
    // println!("x_i: {}", x_i);
    println!("pkrb: {pkrb}");
}
