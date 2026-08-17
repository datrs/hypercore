#![cfg(feature = "replication")]

use hypercore::{
    Hypercore, HypercoreBuilder, HypercoreError, PartialKeypair, Storage,
    replication::UpdateOptions,
};
use hypercore_handshake::{
    Cipher, CipherTrait,
    state_machine::{SecStream, hc_specific::generate_keypair},
};
use std::time::Duration;
use tokio_util::compat::TokioAsyncReadCompatExt;
use uint24le_framing::Uint24LELengthPrefixedFraming;

/// Create a pair of connected in-memory encrypted streams.
fn connected_pair() -> (impl CipherTrait + 'static, impl CipherTrait + 'static) {
    let (a_b, b_a) = tokio::io::duplex(64 * 1024);
    let a_b = Uint24LELengthPrefixedFraming::new(a_b.compat());
    let b_a = Uint24LELengthPrefixedFraming::new(b_a.compat());
    let keypair = generate_keypair().unwrap();
    let initiator = Cipher::new(
        Some(Box::new(a_b)),
        SecStream::new_initiator_xx(&[]).unwrap().into(),
    );
    let responder = Cipher::new(
        Some(Box::new(b_a)),
        SecStream::new_responder_xx(&keypair, &[]).unwrap().into(),
    );
    (initiator, responder)
}

/// Create a writer (with data) and a blank reader sharing the same public key.
async fn make_writer_reader(data: &[&[u8]]) -> (Hypercore, Hypercore) {
    let writer = HypercoreBuilder::new(Storage::new_memory().await.unwrap())
        .build()
        .await
        .unwrap();
    for chunk in data {
        writer.append(chunk).await.unwrap();
    }
    let public_key = writer.key_pair().public;
    let reader = HypercoreBuilder::new(Storage::new_memory().await.unwrap())
        .key_pair(PartialKeypair {
            public: public_key,
            secret: None,
        })
        .build()
        .await
        .unwrap();
    (writer, reader)
}

/// Another read-only replica of `core`'s core, for building multi-peer setups.
async fn make_replica(core: &Hypercore) -> Hypercore {
    HypercoreBuilder::new(Storage::new_memory().await.unwrap())
        .key_pair(PartialKeypair {
            public: core.key_pair().public,
            secret: None,
        })
        .build()
        .await
        .unwrap()
}

/// Get block 0 from `reader`, with `writer`'s replicator spawned to drive the other end.
/// Uses `attach_replicator` so that `reader.get()` itself drives replication (structured
/// concurrency path).
async fn get_via_attached_replicator(writer: &Hypercore, reader: &Hypercore) -> Option<Vec<u8>> {
    let (writer_stream, reader_stream) = connected_pair();
    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    reader.attach_replicator(reader.replicate(reader_stream));
    let block = tokio::time::timeout(Duration::from_secs(5), reader.get(0))
        .await
        .expect("timed out waiting for attach_replicator get")
        .unwrap();
    writer_rep.abort();
    block
}

/// Poll until `core.info().contiguous_length >= expected`, with a 5-second timeout.
///
/// Note this cannot be replaced by `Hypercore::update`: `update` resolves once the *verified
/// length* has caught up, which says nothing about whether the blocks themselves have been
/// downloaded yet. These tests read blocks back, so they need `contiguous_length`.
async fn wait_for_length(core: &Hypercore, expected: u64) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut last_reported = u64::MAX;
    loop {
        let current = core.info().contiguous_length;
        if current >= expected {
            return;
        }
        if current != last_reported {
            eprintln!("contiguous_length = {current} (waiting for {expected})");
            last_reported = current;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out: contiguous_length stuck at {current}, never reached {expected}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// `UpdateOptions` with a deadline, so a broken `update` fails the test instead of hanging it.
fn update_opts(timeout_ms: u64) -> UpdateOptions {
    UpdateOptions {
        timeout: Some(Duration::from_millis(timeout_ms)),
        ..Default::default()
    }
}

/// `update` drives an attached replicator in-band, exactly as `get` does: nothing else is
/// polling the reader's side of the connection here.
#[tokio::test]
async fn update_drives_attached_replicator() {
    let (writer, reader) = make_writer_reader(&[b"hello", b"world"]).await;
    let (writer_stream, reader_stream) = connected_pair();
    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    reader.attach_replicator(reader.replicate(reader_stream));

    assert!(reader.update(update_opts(5000)).await.unwrap());
    assert_eq!(reader.info().length, 2);

    writer_rep.abort();
}

/// The reader is at length 0 and stays there until `update` is polled, so the append is
/// guaranteed to be news by the time `update` takes its snapshot.
#[tokio::test]
async fn update_reports_growth_for_late_append() {
    let (writer, reader) = make_writer_reader(&[]).await;
    let (writer_stream, reader_stream) = connected_pair();
    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    reader.attach_replicator(reader.replicate(reader_stream));

    writer.append(b"late").await.unwrap();

    assert!(reader.update(update_opts(5000)).await.unwrap());
    assert_eq!(reader.info().length, 1);

    writer_rep.abort();
}

/// Once caught up, a second `update` reports that the peer has nothing newer.
#[tokio::test]
async fn update_returns_false_when_in_sync() {
    let (writer, reader) = make_writer_reader(&[b"hello", b"world"]).await;
    let (writer_stream, reader_stream) = connected_pair();
    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    reader.attach_replicator(reader.replicate(reader_stream));

    assert!(reader.update(update_opts(5000)).await.unwrap());
    assert!(!reader.update(update_opts(5000)).await.unwrap());
    assert_eq!(reader.info().length, 2);

    writer_rep.abort();
}

/// `wait: false` answers from what is already known rather than waiting for a peer.
#[tokio::test]
async fn update_no_wait_returns_false_without_peers() {
    let (_, reader) = make_writer_reader(&[b"hello"]).await;
    assert_eq!(reader.peer_count(), 0);

    let updated = reader
        .update(UpdateOptions {
            wait: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(!updated);
}

/// `wait: true` with no peers blocks, and the timeout is what ends it.
#[tokio::test]
async fn update_times_out_without_peers() {
    let (_, reader) = make_writer_reader(&[b"hello"]).await;

    let err = reader.update(update_opts(100)).await.unwrap_err();
    assert!(
        matches!(err, HypercoreError::Timeout { .. }),
        "expected Timeout, got {err:?}"
    );
}

/// A writable core has nothing to fetch, so `update` short-circuits unless forced.
#[tokio::test]
async fn update_on_writable_core_returns_false() {
    let (writer, _) = make_writer_reader(&[b"hello"]).await;
    assert!(writer.info().writeable);

    // Not forced: returns immediately without waiting for the (nonexistent) peers.
    assert!(!writer.update(update_opts(5000)).await.unwrap());

    // Forced: actually runs, and so hits the timeout waiting for a peer.
    let err = writer
        .update(UpdateOptions {
            force: true,
            timeout: Some(Duration::from_millis(100)),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(
        matches!(err, HypercoreError::Timeout { .. }),
        "expected Timeout, got {err:?}"
    );
}

/// Two peers where only one has anything, so the per-peer loop has to cope with a mix.
///
/// This deliberately does *not* assert the returned bool. `update` answers from the peers
/// connected at the moment it decides — matching Javascript's `_checkUpgradeIfAvailable`,
/// which likewise iterates only `this.peers` — so if the empty peer happens to register and
/// sync before the writer's channel opens, `Ok(false)` is the correct answer. What is
/// guaranteed either way is the postcondition: `update` does not return while a peer it can
/// see is known to have more.
#[tokio::test]
async fn update_with_two_peers_only_one_useful() {
    let (writer, reader) = make_writer_reader(&[b"hello", b"world"]).await;
    let empty_peer = make_replica(&writer).await;

    let (writer_stream, reader_stream_a) = connected_pair();
    let (empty_stream, reader_stream_b) = connected_pair();
    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    let empty_rep = tokio::spawn(empty_peer.replicate(empty_stream));

    reader.attach_replicator(
        reader
            .replicator()
            .with_connection(reader_stream_a)
            .with_connection(reader_stream_b),
    );

    // Retry to absorb the registration race described above: whichever peer wins, the reader
    // must end up at the writer's length rather than stalling on the empty peer.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while reader.info().length < 2 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "reader stuck at length {} with 2 peers attached",
            reader.info().length
        );
        reader.update(update_opts(1000)).await.unwrap();
    }
    assert_eq!(reader.info().length, 2);
    assert_eq!(reader.peer_count(), 2, "both peers should be registered");

    writer_rep.abort();
    empty_rep.abort();
}

/// With two peers and nothing newer anywhere, `update` reports false — but only after both
/// have been heard from.
#[tokio::test]
async fn update_with_two_peers_all_in_sync() {
    let (writer, reader) = make_writer_reader(&[b"hello", b"world"]).await;
    let other = make_replica(&writer).await;

    let (writer_stream, reader_stream_a) = connected_pair();
    let (other_stream, reader_stream_b) = connected_pair();
    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    let other_rep = tokio::spawn(other.replicate(other_stream));

    reader.attach_replicator(
        reader
            .replicator()
            .with_connection(reader_stream_a)
            .with_connection(reader_stream_b),
    );

    assert!(reader.update(update_opts(5000)).await.unwrap());
    assert!(!reader.update(update_opts(5000)).await.unwrap());
    assert_eq!(reader.info().length, 2);
    assert_eq!(reader.peer_count(), 2);

    writer_rep.abort();
    other_rep.abort();
}

/// `update` also works when replication is driven by someone else entirely (a spawned
/// replicator, as corestore does). Whether it reports growth is a race against the
/// background task, but it must not return before the reader has caught up.
#[tokio::test]
async fn update_with_externally_driven_replication() {
    let (writer, reader) = make_writer_reader(&[b"a", b"b", b"c"]).await;
    let (writer_stream, reader_stream) = connected_pair();
    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    let reader_rep = tokio::spawn(reader.replicate(reader_stream));

    reader.update(update_opts(5000)).await.unwrap();
    assert_eq!(reader.info().length, 3);
    assert_eq!(reader.peer_count(), 1);

    writer_rep.abort();
    reader_rep.abort();
}

#[tokio::test]
async fn replicate_data_before_connect() {
    let (writer, reader) = make_writer_reader(&[b"hello", b"world"]).await;
    let (writer_stream, reader_stream) = connected_pair();

    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    let reader_rep = tokio::spawn(reader.replicate(reader_stream));

    wait_for_length(&reader, 2).await;

    assert_eq!(reader.get(0).await.unwrap(), Some(b"hello".to_vec()));
    assert_eq!(reader.get(1).await.unwrap(), Some(b"world".to_vec()));

    writer_rep.abort();
    reader_rep.abort();
}

#[tokio::test]
async fn replicate_data_after_connect() {
    let (writer, reader) = make_writer_reader(&[]).await;
    let (writer_stream, reader_stream) = connected_pair();

    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    let reader_rep = tokio::spawn(reader.replicate(reader_stream));

    writer.append(b"late data").await.unwrap();

    wait_for_length(&reader, 1).await;

    assert_eq!(reader.get(0).await.unwrap(), Some(b"late data".to_vec()));

    writer_rep.abort();
    reader_rep.abort();
}

/// Without an attached replicator, `get()` returns `None` immediately for a missing block.
#[tokio::test]
async fn get_returns_none_without_replicator() {
    let (_, reader) = make_writer_reader(&[b"hello"]).await;
    assert_eq!(reader.get(0).await.unwrap(), None);
}

/// `attach_replicator` makes `reader.get()` drive replication itself — no spawn needed for
/// the reader side.
#[tokio::test]
async fn attach_replicator_drives_get() {
    let (writer, reader) = make_writer_reader(&[b"hello", b"world"]).await;
    assert_eq!(
        get_via_attached_replicator(&writer, &reader).await,
        Some(b"hello".to_vec())
    );
}

/// `attach_replicator` still works when the writer appends the block *after* the connection
/// is established.
#[tokio::test]
async fn attach_replicator_drives_get_late_data() {
    let (writer, reader) = make_writer_reader(&[]).await;
    let (writer_stream, reader_stream) = connected_pair();
    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    reader.attach_replicator(reader.replicate(reader_stream));

    writer.append(b"late").await.unwrap();

    let block = tokio::time::timeout(Duration::from_secs(5), reader.get(0))
        .await
        .expect("timed out")
        .unwrap();
    assert_eq!(block, Some(b"late".to_vec()));
    writer_rep.abort();
}

/// Sequential gets — after `get(0)` resolves, `get(1)` and `get(2)` must also work.
#[tokio::test]
async fn attach_replicator_gets_sequential_blocks() {
    let (writer, reader) = make_writer_reader(&[b"a", b"b", b"c"]).await;
    let (writer_stream, reader_stream) = connected_pair();
    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    reader.attach_replicator(reader.replicate(reader_stream));

    for (i, expected) in [b"a".as_slice(), b"b", b"c"].iter().enumerate() {
        let block = tokio::time::timeout(Duration::from_secs(5), reader.get(i as u64))
            .await
            .unwrap_or_else(|_| panic!("timed out on block {i}"))
            .unwrap();
        assert_eq!(block.as_deref(), Some(*expected), "block {i}");
    }
    writer_rep.abort();
}

/// Get a non-zero block directly without fetching earlier indices first.
/// Verifies that `Event::Get` fires with the requested index, not always 0.
#[tokio::test]
async fn attach_replicator_get_by_index() {
    let data: Vec<Vec<u8>> = (0u8..5).map(|i| vec![i]).collect();
    let slices: Vec<&[u8]> = data.iter().map(|v| v.as_slice()).collect();
    let (writer, reader) = make_writer_reader(&slices).await;
    let (writer_stream, reader_stream) = connected_pair();
    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    reader.attach_replicator(reader.replicate(reader_stream));

    let block = tokio::time::timeout(Duration::from_secs(5), reader.get(4))
        .await
        .expect("timed out")
        .unwrap();
    assert_eq!(block, Some(vec![4u8]));
    writer_rep.abort();
}

#[tokio::test]
async fn replicate_many_blocks() {
    let data: Vec<Vec<u8>> = (0u8..10).map(|i| vec![i]).collect();
    let slices: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();

    let (writer, reader) = make_writer_reader(&slices).await;
    let (writer_stream, reader_stream) = connected_pair();

    let writer_rep = tokio::spawn(writer.replicate(writer_stream));
    let reader_rep = tokio::spawn(reader.replicate(reader_stream));

    wait_for_length(&reader, 10).await;

    for (i, expected) in data.iter().enumerate() {
        assert_eq!(
            reader.get(i as u64).await.unwrap().as_ref(),
            Some(expected),
            "block {i} mismatch"
        );
    }

    writer_rep.abort();
    reader_rep.abort();
}

