#[cfg(feature = "async-std")]
use async_std::main as async_main;
use hypercore::{HypercoreBuilder, Storage};
use tempfile::Builder;
#[cfg(feature = "tokio")]
use tokio::main as async_main;

#[async_main]
async fn main() {
    // For the purposes of this example, first create a
    // temporary directory to hold hypercore.
    let dir = Builder::new()
        .prefix("examples_disk")
        .tempdir()
        .unwrap()
        .into_path();

    // Create a disk storage, overwriting existing values.
    let overwrite = true;
    let storage = Storage::new_disk(&dir, overwrite)
        .await
        .expect("Could not create disk storage");

    // Build a new disk hypercore
    let mut hypercore = HypercoreBuilder::new(storage)
        .build()
        .await
        .expect("Could not create disk hypercore");

    // Append values to the hypercore
    hypercore.append(b"Hello, ").await.unwrap();
    hypercore.append(b"from ").await.unwrap();

    // Close hypercore
    drop(hypercore);

    // Open hypercore again from same directory, not
    // overwriting.
    let overwrite = false;
    let storage = Storage::new_disk(&dir, overwrite)
        .await
        .expect("Could not open existing disk storage");
    let mut hypercore = HypercoreBuilder::new(storage)
        .open(true)
        .build()
        .await
        .expect("Could not open disk hypercore");

    // Append new values to the hypercore
    hypercore.append(b"disk hypercore!").await.unwrap();

    // Add three values and clear the first two
    let batch: &[&[u8]] = &[
        b"first value to clear",
        b"second value to clear",
        b"third value to keep",
    ];
    let new_length = hypercore.append_batch(batch).await.unwrap().length;
    hypercore
        .clear(new_length - 3, new_length - 1)
        .await
        .unwrap();

    // The two values return None, but the last one returns correctly
    assert!(hypercore.get(3).await.unwrap().is_none());
    assert!(hypercore.get(4).await.unwrap().is_none());
    assert_eq!(
        hypercore.get(5).await.unwrap().unwrap(),
        b"third value to keep"
    );

    // Print the first three values, converting binary back to string
    println!(
        "{}{}{}",
        String::from_utf8(hypercore.get(0).await.unwrap().unwrap()).unwrap(),
        String::from_utf8(hypercore.get(1).await.unwrap().unwrap()).unwrap(),
        String::from_utf8(hypercore.get(2).await.unwrap().unwrap()).unwrap()
    ); // prints "Hello, from disk hypercore!"
}
