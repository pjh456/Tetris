use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tetris_core::{Board, Engine};

fn bench_engine_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine_tick");

    for seed in [42u32, 100, 9999] {
        group.bench_with_input(BenchmarkId::new("single_tick", seed), &seed, |b, &s| {
            let mut engine = Engine::<10, 20>::new();
            engine.reset(s);
            b.iter(|| {
                black_box(engine.tick());
            });
        });
    }

    group.bench_function("100_ticks_seed42", |b| {
        b.iter(|| {
            let mut engine = Engine::<10, 20>::new();
            engine.reset(42);
            for _ in 0..100 {
                black_box(engine.tick());
            }
        });
    });

    group.finish();
}

fn bench_board_clear(c: &mut Criterion) {
    let mut group = c.benchmark_group("board_clear");

    group.bench_function("empty_board", |b| {
        let mut board = Board::<10, 20>::new();
        b.iter(|| {
            black_box(board.clear_lines());
        });
    });

    group.bench_function("one_full_row", |b| {
        b.iter_batched(
            || {
                let mut board = Board::<10, 20>::new();
                board.rows[19] = Board::<10, 20>::FULL;
                board
            },
            |mut board| {
                black_box(board.clear_lines());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("four_full_rows", |b| {
        b.iter_batched(
            || {
                let mut board = Board::<10, 20>::new();
                board.rows[16] = Board::<10, 20>::FULL;
                board.rows[17] = Board::<10, 20>::FULL;
                board.rows[18] = Board::<10, 20>::FULL;
                board.rows[19] = Board::<10, 20>::FULL;
                board
            },
            |mut board| {
                black_box(board.clear_lines());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_bincode_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("bincode");

    let board_rows: Vec<u64> = vec![0u64; 20];

    group.bench_function("serialize_board_rows", |b| {
        b.iter(|| {
            black_box(bincode::serialize(&board_rows).unwrap());
        });
    });

    let serialized = bincode::serialize(&board_rows).unwrap();
    group.bench_function("deserialize_board_rows", |b| {
        b.iter(|| {
            black_box(bincode::deserialize::<Vec<u64>>(&serialized).unwrap());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_engine_tick, bench_board_clear, bench_bincode_roundtrip);
criterion_main!(benches);
