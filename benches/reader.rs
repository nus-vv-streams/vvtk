use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::Path;
use vivotk::pcd::read_pcd_file;
use vivotk::ply::read_ply;

fn bench_read_ply(c: &mut Criterion) {
    c.bench_function("read_ply", |b| {
        let p = Path::new("../test/longdress_vox10_1051.ply");
        b.iter(|| {
            read_ply(black_box(p));
        })
    });
}

fn bench_read_pcd(c: &mut Criterion) {
    c.bench_function("read_pcd", |b| {
        let p = Path::new("../test/0.pcd");
        b.iter(|| {
            _ = read_pcd_file(black_box(p));
        })
    });
}

criterion_group!(benches, bench_read_ply, bench_read_pcd);
criterion_main!(benches);
