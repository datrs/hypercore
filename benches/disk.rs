use std::time::{Duration, Instant};

use criterion::async_executor::AsyncStdExecutor;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hypercore::{Hypercore, HypercoreBuilder, HypercoreError, Storage};
use random_access_disk::RandomAccessDisk;
use tempfile::Builder as TempfileBuilder;

#[cfg(feature = "cache")]
async fn create_hypercore(name: &str) -> Result<Hypercore<RandomAccessDisk>, HypercoreError> {
    let dir = TempfileBuilder::new()
        .prefix(name)
        .tempdir()
        .unwrap()
        .into_path();
    let storage = Storage::new_disk(&dir, true).await?;
    HypercoreBuilder::new(storage)
        .node_cache_options(hypercore::CacheOptionsBuilder::new())
        .build()
        .await
}

#[cfg(not(feature = "cache"))]
async fn create_hypercore(name: &str) -> Result<Hypercore<RandomAccessDisk>, HypercoreError> {
    let dir = TempfileBuilder::new()
        .prefix(name)
        .tempdir()
        .unwrap()
        .into_path();
    let storage = Storage::new_disk(&dir, true).await?;
    HypercoreBuilder::new(storage).build().await
}

fn create_disk(c: &mut Criterion) {
    let mut group = c.benchmark_group("slow_call");
    group.measurement_time(Duration::from_secs(20));
    group.bench_function("create_disk", move |b| {
        b.to_async(AsyncStdExecutor)
            .iter(|| create_hypercore("create"));
    });
}

fn write_disk(c: &mut Criterion) {
    let mut group = c.benchmark_group("slow_call");
    group.measurement_time(Duration::from_secs(20));
    group.bench_function("write_disk", move |b| {
        b.to_async(AsyncStdExecutor)
            .iter_custom(|iters| async move {
                let mut hypercore = create_hypercore("write").await.unwrap();
                let data = Vec::from("hello");
                let start = Instant::now();
                for _ in 0..iters {
                    black_box(hypercore.append(&data).await.unwrap());
                }
                start.elapsed()
            });
    });
}

fn read_disk(c: &mut Criterion) {
    let mut group = c.benchmark_group("slow_call");
    group.measurement_time(Duration::from_secs(20));
    group.bench_function("read_disk", move |b| {
        b.to_async(AsyncStdExecutor)
            .iter_custom(|iters| async move {
                let mut hypercore = create_hypercore("read").await.unwrap();
                let data = Vec::from("hello");
                for _ in 0..iters {
                    hypercore.append(&data).await.unwrap();
                }
                let start = Instant::now();
                for i in 0..iters {
                    black_box(hypercore.get(i).await.unwrap());
                }
                start.elapsed()
            });
    });
}

fn clear_disk(c: &mut Criterion) {
    let mut group = c.benchmark_group("slow_call");
    group.measurement_time(Duration::from_secs(20));
    group.bench_function("clear_disk", move |b| {
        b.to_async(AsyncStdExecutor)
            .iter_custom(|iters| async move {
                let mut hypercore = create_hypercore("clear").await.unwrap();
                let data = Vec::from("hello");
                for _ in 0..iters {
                    hypercore.append(&data).await.unwrap();
                }
                let start = Instant::now();
                for i in 0..iters {
                    black_box(hypercore.clear(i, 1).await.unwrap());
                }
                start.elapsed()
            });
    });
}

criterion_group!(benches, create_disk, write_disk, read_disk, clear_disk);
criterion_main!(benches);
