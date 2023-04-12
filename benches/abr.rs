use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vivotk::abr::quetra::{Quetra, QuetraMultiview};
use vivotk::abr::RateAdapter;

fn bench_quetra_multiview(c: &mut Criterion) {
    c.bench_function("quetra-multiview", |b| {
        let abr = QuetraMultiview::new(4, 30.0, 6, vec![1.72, 2.69, 3.61, 4.26, 4.47, 4.5]);
        b.iter(|| {
            abr.select_quality(
                black_box(2),
                black_box(1728450.56),
                black_box(&[
                    vec![827392, 1089536, 1490944, 2646016, 4972544, 8110080],
                    vec![335872, 368640, 368640, 532480, 786432, 729088],
                    vec![729088, 999424, 1466368, 2596864, 4767744, 7340032],
                    vec![794624, 1048576, 1466368, 2547712, 4685824, 7872512],
                    vec![270336, 303104, 319488, 442368, 704512, 679936],
                    vec![802816, 1105920, 1572864, 2842624, 5349376, 7626752],
                ]),
                black_box(&[
                    -0.07874092,
                    -0.054728724,
                    0.9953917,
                    0.07874092,
                    0.054728724,
                    -0.9953917,
                ]),
            );
        })
    });
}

fn bench_quetra(c: &mut Criterion) {
    c.bench_function("quetra", |b| {
        let abr = Quetra::new(4, 30.0);
        b.iter(|| {
            abr.select_quality(
                black_box(2),
                black_box(1728450.56),
                black_box(&[
                    vec![827392, 1089536, 1490944, 2646016, 4972544, 8110080],
                    vec![335872, 368640, 368640, 532480, 786432, 729088],
                    vec![729088, 999424, 1466368, 2596864, 4767744, 7340032],
                    vec![794624, 1048576, 1466368, 2547712, 4685824, 7872512],
                    vec![270336, 303104, 319488, 442368, 704512, 679936],
                    vec![802816, 1105920, 1572864, 2842624, 5349376, 7626752],
                ]),
                black_box(&[
                    -0.07874092,
                    -0.054728724,
                    0.9953917,
                    0.07874092,
                    0.054728724,
                    -0.9953917,
                ]),
            );
        })
    });
}

criterion_group!(benches, bench_quetra_multiview, bench_quetra);
criterion_main!(benches);
