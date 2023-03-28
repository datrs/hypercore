use std::time::Instant;

use criterion::async_executor::AsyncStdExecutor;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hypercore::{Hypercore, HypercoreBuilder, HypercoreError, Storage};
use random_access_memory::RandomAccessMemory;

#[cfg(feature = "cache")]
async fn create_hypercore(
    page_size: usize,
) -> Result<Hypercore<RandomAccessMemory>, HypercoreError> {
    let storage = Storage::open(
        |_| Box::pin(async move { Ok(RandomAccessMemory::new(page_size)) }),
        false,
    )
    .await?;
    HypercoreBuilder::new(storage)
        .node_cache_options(hypercore::CacheOptionsBuilder::new())
        .build()
        .await
}

#[cfg(not(feature = "cache"))]
async fn create_hypercore(
    page_size: usize,
) -> Result<Hypercore<RandomAccessMemory>, HypercoreError> {
    let storage = Storage::open(
        |_| Box::pin(async move { Ok(RandomAccessMemory::new(page_size)) }),
        false,
    )
    .await?;
    HypercoreBuilder::new(storage).build().await
}

fn create_memory(c: &mut Criterion) {
    c.bench_function("create_memory", move |b| {
        b.to_async(AsyncStdExecutor).iter(|| create_hypercore(1024));
    });
}

fn write_memory(c: &mut Criterion) {
    c.bench_function("write_memory", move |b| {
        b.to_async(AsyncStdExecutor)
            .iter_custom(|iters| async move {
                let mut hypercore = create_hypercore(1024).await.unwrap();
                let data = Vec::from("hello");
                let start = Instant::now();
                for _ in 0..iters {
                    black_box(hypercore.append(&data).await.unwrap());
                }
                start.elapsed()
            });
    });
}

fn read_memory(c: &mut Criterion) {
    c.bench_function("read_memory", move |b| {
        b.to_async(AsyncStdExecutor)
            .iter_custom(|iters| async move {
                let mut hypercore = create_hypercore(1024).await.unwrap();
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

fn clear_memory(c: &mut Criterion) {
    c.bench_function("clear_memory", move |b| {
        b.to_async(AsyncStdExecutor)
            .iter_custom(|iters| async move {
                let mut hypercore = create_hypercore(1024).await.unwrap();
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

criterion_group!(
    benches,
    create_memory,
    write_memory,
    read_memory,
    clear_memory
);
criterion_main!(benches);
