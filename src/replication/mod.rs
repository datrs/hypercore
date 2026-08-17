//! Hypercore to Hypercore replication
pub mod events;
mod update;

use std::{
    collections::{BTreeSet, VecDeque},
    future::Future,
    io,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    task::{Context, Poll},
};

use futures::{
    Stream, StreamExt,
    stream::{FuturesUnordered, SelectAll},
};
use hypercore_handshake::CipherTrait;
use hypercore_protocol::{
    Channel, Message, Protocol, discovery_key,
    schema::{Data, Range, Request, Synchronize},
};
use tracing::{error, trace, warn};

use crate::{
    AppendOutcome, Hypercore, HypercoreError, Info, PartialKeypair,
    core::inner::{
        CreateProofFuture, HypercoreInner, MissingNodesFuture, VerifyAndApplyProofFuture,
    },
};
use hypercore_schema::{RequestBlock, RequestSeek, RequestUpgrade};

pub use events::Event;
pub use update::{UpdateFuture, UpdateOptions};

use async_broadcast::Receiver;

/// Methods related to just this core's information
pub trait CoreInfo {
    /// Get core info (see: [`crate::Hypercore::info`]
    fn info(&self) -> impl Future<Output = Info> + Send;
    /// Get the key_pair (see: [`crate::Hypercore::key_pair`]
    fn key_pair(&self) -> impl Future<Output = PartialKeypair> + Send;
}

/// Error for ReplicationMethods trait
#[derive(thiserror::Error, Debug)]
pub enum ReplicationMethodsError {
    /// Error from hypercore
    #[error("Got a hypercore error: [{0}]")]
    HypercoreError(#[from] HypercoreError),
    /// Error from CoreMethods
    #[error("Got a CoreMethods error: [{0}]")]
    CoreMethodsError(#[from] CoreMethodsError),
}

/// Methods needed for replication
pub trait ReplicationMethods: CoreInfo + Send {
    /// ref Core::verify_and_apply_proof
    fn verify_and_apply_proof(
        &self,
        proof: hypercore_schema::Proof,
    ) -> impl Future<Output = Result<bool, ReplicationMethodsError>> + Send;
    /// ref Core::missing_nodes
    fn missing_nodes(
        &self,
        index: u64,
    ) -> impl Future<Output = Result<u64, ReplicationMethodsError>> + Send;
    /// ref Core::create_proof
    fn create_proof(
        &self,
        block: Option<RequestBlock>,
        hash: Option<RequestBlock>,
        seek: Option<RequestSeek>,
        upgrade: Option<RequestUpgrade>,
    ) -> impl Future<Output = Result<Option<hypercore_schema::Proof>, ReplicationMethodsError>> + Send;
    /// subscribe to core events
    fn event_subscribe(&self) -> impl Future<Output = Receiver<Event>>;
}

/// Error for CoreMethods trait
#[derive(thiserror::Error, Debug)]
pub enum CoreMethodsError {
    /// Error from hypercore
    #[error("Got a hypercore error [{0}]")]
    HypercoreError(#[from] HypercoreError),
}

/// Trait for things that consume [`crate::Hypercore`].
pub trait CoreMethods: CoreInfo {
    /// Check if the core has the block at the given index locally
    fn has(&self, index: u64) -> impl Future<Output = bool> + Send;

    /// get a block
    fn get(
        &self,
        index: u64,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, CoreMethodsError>> + Send;

    /// Append data to the core
    fn append(
        &self,
        data: &[u8],
    ) -> impl Future<Output = Result<AppendOutcome, CoreMethodsError>> + Send;

    /// Append a batch of data to the core
    fn append_batch<A: AsRef<[u8]>, B: AsRef<[A]> + Send>(
        &self,
        batch: B,
    ) -> impl Future<Output = Result<AppendOutcome, CoreMethodsError>> + Send;
}

// ── Remote bitfield ────────────────────────────────────────────────────────────

struct RemoteBitfield(BTreeSet<u64>);

impl RemoteBitfield {
    fn new() -> Self {
        Self(BTreeSet::new())
    }

    fn get(&self, index: u64) -> bool {
        self.0.contains(&index)
    }

    fn set_range(&mut self, start: u64, length: u64, value: bool) {
        for i in start..(start + length) {
            if value {
                self.0.insert(i);
            } else {
                self.0.remove(&i);
            }
        }
    }
}

// ── PeerSyncState ──────────────────────────────────────────────────────────────

/// Generate paired getter/setter methods over atomic fields.
macro_rules! atomic_accessors {
    ($($get:ident / $set:ident : $ty:ty),* $(,)?) => {
        $(
            pub(crate) fn $get(&self) -> $ty {
                self.$get.load(Ordering::Relaxed)
            }
            pub(crate) fn $set(&self, value: $ty) {
                self.$get.store(value, Ordering::Relaxed)
            }
        )*
    };
}

/// The slice of one peer's protocol state that the core itself needs, in order to answer
/// "could any peer still upgrade me?" — see [`crate::Hypercore::update`].
///
/// Owned by the [`ChannelState`] that writes it. The core holds only [`std::sync::Weak`]
/// observers, so when a channel goes away its entry simply becomes unreachable and is pruned
/// on the next read; there is no deregistration to get wrong.
#[derive(Debug, Default)]
pub(crate) struct PeerSyncState {
    /// The peer has sent us at least one [`Synchronize`].
    remote_synced: AtomicBool,
    remote_fork: AtomicU64,
    remote_length: AtomicU64,
    /// The peer says it can serve us an upgrade starting from the length we last told it.
    remote_can_upgrade: AtomicBool,
    /// The length of *ours* that the peer has echoed back to us.
    length_acked: AtomicU64,
    /// An upgrade [`Request`] is on the wire and its [`Data`] has not been applied yet.
    upgrade_inflight: AtomicBool,
}

impl PeerSyncState {
    atomic_accessors! {
        remote_synced / set_remote_synced: bool,
        remote_fork / set_remote_fork: u64,
        remote_length / set_remote_length: u64,
        remote_can_upgrade / set_remote_can_upgrade: bool,
        length_acked / set_length_acked: u64,
        upgrade_inflight / set_upgrade_inflight: bool,
    }
}

// ── PeerState ──────────────────────────────────────────────────────────────────

struct PeerState {
    can_upgrade: bool,
    remote_bitfield: RemoteBitfield,
    remote_uploading: bool,
    remote_downloading: bool,
    /// The subset of this state the core can observe. See [`PeerSyncState`].
    shared: Arc<PeerSyncState>,
}

impl Default for PeerState {
    fn default() -> Self {
        Self {
            can_upgrade: true,
            remote_bitfield: RemoteBitfield::new(),
            remote_uploading: true,
            remote_downloading: true,
            shared: Arc::new(PeerSyncState::default()),
        }
    }
}

// ── DataMeta ───────────────────────────────────────────────────────────────────

/// Saved from a Data message; used after verify_and_apply completes to decide
/// what to request next.
struct DataMeta {
    has_upgrade: bool,
    pre_length: u64,
    remote_length: u64,
    block_index: Option<u64>,
}

// ── ChannelState ───────────────────────────────────────────────────────────────

struct ChannelState {
    channel: Channel,
    state: PeerState,
    synced: bool,

    // Outgoing batches; drained one at a time through pending_send.
    outgoing: VecDeque<Vec<Message>>,
    pending_send: Option<Pin<Box<dyn Future<Output = io::Result<()>> + Send>>>,

    // Incoming Request → create_proof
    pending_create_proof: Option<CreateProofFuture>,
    pending_create_proof_id: u64,
    pending_create_proof_fork: u64,

    // Incoming Data → verify_and_apply_proof
    pending_verify_apply: Option<VerifyAndApplyProofFuture>,
    pending_data_meta: Option<DataMeta>,

    // Block request queue: each index needs a missing_nodes call before we
    // can send the Request message.
    pending_request_indices: VecDeque<u64>,
    pending_missing_nodes: Option<(u64, MissingNodesFuture)>,

    // (our fork, our length, their length) of the last upgrade Request we sent, so that
    // re-checking every poll does not re-send the same request.
    last_upgrade_requested: Option<(u64, u64, u64)>,

    // Core events (Get / Have / DataUpgrade / Upgrade)
    core_events: async_broadcast::Receiver<events::Event>,
}

impl ChannelState {
    /// Registers this peer with `inner`, so that [`crate::Hypercore::update`] can see its
    /// advertised length. The registry holds a `Weak`, so dropping this `ChannelState` is
    /// all the deregistration there is.
    fn new(channel: Channel, inner: &HypercoreInner) -> Self {
        let state = PeerState::default();
        inner.register_peer(&state.shared);
        inner.send_event(events::PeerSync {});
        Self {
            channel,
            state,
            synced: false,
            outgoing: VecDeque::new(),
            pending_send: None,
            pending_create_proof: None,
            pending_create_proof_id: 0,
            pending_create_proof_fork: 0,
            pending_verify_apply: None,
            pending_data_meta: None,
            pending_request_indices: VecDeque::new(),
            pending_missing_nodes: None,
            last_upgrade_requested: None,
            core_events: inner.event_subscribe(),
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        inner: &HypercoreInner,
    ) -> Poll<Result<(), HypercoreError>> {
        // ── Drive pending send ─────────────────────────────────────────────────
        if let Some(ref mut fut) = self.pending_send {
            match fut.as_mut().poll(cx) {
                Poll::Ready(Ok(())) => self.pending_send = None,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                Poll::Pending => {}
            }
        }
        if self.pending_send.is_none()
            && let Some(batch) = self.outgoing.pop_front()
        {
            let channel = self.channel.clone();
            self.pending_send = Some(Box::pin(async move { channel.send_batch(&batch).await }));
            cx.waker().wake_by_ref();
        }

        // ── Initial sync ───────────────────────────────────────────────────────
        if !self.synced {
            let info = inner.info();
            let remote_length = if info.fork == self.state.shared.remote_fork() {
                self.state.shared.remote_length()
            } else {
                0
            };
            let mut msgs = vec![Message::Synchronize(Synchronize {
                fork: info.fork,
                length: info.length,
                remote_length,
                can_upgrade: self.state.can_upgrade,
                uploading: true,
                downloading: true,
            })];
            if info.contiguous_length > 0 {
                msgs.push(Message::Range(Range {
                    drop: false,
                    start: 0,
                    length: info.contiguous_length,
                }));
            }
            self.outgoing.push_back(msgs);
            self.synced = true;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        // ── Poll core events ───────────────────────────────────────────────────
        loop {
            match Pin::new(&mut self.core_events).poll_next(cx) {
                Poll::Ready(Some(event)) => self.on_core_event(event, inner),
                Poll::Ready(None) => break,
                Poll::Pending => break,
            }
        }

        // ── Drive pending_missing_nodes ────────────────────────────────────────
        if let Some((index, ref mut fut)) = self.pending_missing_nodes {
            match Pin::new(fut).poll(cx) {
                Poll::Ready(Ok(nodes)) => {
                    self.pending_missing_nodes = None;
                    if self.state.remote_bitfield.get(index)
                        && self.state.shared.remote_length() > index
                    {
                        let info = inner.info();
                        self.outgoing.push_back(vec![Message::Request(Request {
                            id: index + 1,
                            fork: info.fork,
                            block: Some(RequestBlock { index, nodes }),
                            hash: None,
                            seek: None,
                            upgrade: None,
                            manifest: false,
                            priority: 42,
                        })]);
                        cx.waker().wake_by_ref();
                    }
                    if let Some(next) = self.pending_request_indices.pop_front() {
                        self.pending_missing_nodes =
                            Some((next, inner.missing_nodes_from_merkle_tree_index(next * 2)));
                        cx.waker().wake_by_ref();
                    }
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => {}
            }
        } else if let Some(index) = self.pending_request_indices.pop_front() {
            self.pending_missing_nodes =
                Some((index, inner.missing_nodes_from_merkle_tree_index(index * 2)));
            cx.waker().wake_by_ref();
        }

        // ── Drive pending_create_proof ─────────────────────────────────────────
        if let Some(ref mut fut) = self.pending_create_proof {
            match Pin::new(fut).poll(cx) {
                Poll::Ready(Ok(maybe_proof)) => {
                    let req_id = self.pending_create_proof_id;
                    let req_fork = self.pending_create_proof_fork;
                    self.pending_create_proof = None;
                    if let Some(proof) = maybe_proof {
                        self.outgoing.push_back(vec![Message::Data(Data {
                            request: req_id,
                            fork: req_fork,
                            hash: proof.hash,
                            block: proof.block,
                            seek: proof.seek,
                            upgrade: proof.upgrade,
                        })]);
                        cx.waker().wake_by_ref();
                    }
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => {}
            }
        }

        // ── Drive pending_verify_apply ─────────────────────────────────────────
        if let Some(ref mut fut) = self.pending_verify_apply {
            match Pin::new(fut).poll(cx) {
                Poll::Ready(Ok(_applied)) => {
                    self.pending_verify_apply = None;
                    if let Some(meta) = self.pending_data_meta.take() {
                        if meta.has_upgrade {
                            self.state.shared.set_upgrade_inflight(false);
                        }
                        let next_index = if meta.has_upgrade {
                            (meta.pre_length < meta.remote_length).then_some(meta.pre_length)
                        } else {
                            meta.block_index
                                .filter(|&i| i < meta.remote_length.saturating_sub(1))
                                .map(|i| i + 1)
                        };
                        if let Some(idx) = next_index {
                            self.pending_request_indices.push_front(idx);
                            cx.waker().wake_by_ref();
                        }
                    }
                }
                Poll::Ready(Err(e)) => {
                    if self
                        .pending_data_meta
                        .take()
                        .is_some_and(|meta| meta.has_upgrade)
                    {
                        self.state.shared.set_upgrade_inflight(false);
                    }
                    return Poll::Ready(Err(e));
                }
                Poll::Pending => {}
            }
        }

        // ── Re-check whether we should ask this peer for an upgrade ────────────
        // The peer's Synchronize may have arrived at a moment when we could not act on it
        // (e.g. it had not yet acked our length). This is the analogue of JS's `updateAll`.
        if self.maybe_request_upgrade(inner) {
            cx.waker().wake_by_ref();
        }

        // ── Poll channel for incoming messages ─────────────────────────────────
        match Pin::new(&mut self.channel).poll_next(cx) {
            Poll::Ready(Some(message)) => {
                self.on_message(message, inner);
                cx.waker().wake_by_ref();
            }
            Poll::Ready(None) => return Poll::Ready(Ok(())),
            Poll::Pending => {}
        }

        Poll::Pending
    }

    fn on_message(&mut self, message: Message, inner: &HypercoreInner) {
        match message {
            Message::Synchronize(msg) => self.on_synchronize(&msg, inner),
            Message::Request(msg) => {
                if self.pending_create_proof.is_none() {
                    self.pending_create_proof_id = msg.id;
                    self.pending_create_proof_fork = msg.fork;
                    self.pending_create_proof =
                        Some(inner.create_proof(msg.block, msg.hash, msg.seek, msg.upgrade));
                }
            }
            Message::Data(msg) => {
                if self.pending_verify_apply.is_none() {
                    let info = inner.info();
                    self.pending_data_meta = Some(DataMeta {
                        has_upgrade: msg.upgrade.is_some(),
                        pre_length: info.length,
                        remote_length: self.state.shared.remote_length(),
                        block_index: msg.block.as_ref().map(|b| b.index),
                    });
                    self.pending_verify_apply =
                        Some(inner.verify_and_apply_proof(msg.into_proof()));
                }
            }
            Message::Range(Range { start, length, .. }) => {
                self.state.remote_bitfield.set_range(start, length, true);
            }
            _ => {}
        }
    }

    fn on_synchronize(&mut self, msg: &Synchronize, inner: &HypercoreInner) {
        let info = inner.info();
        let first_sync = !self.state.shared.remote_synced();
        let same_fork = msg.fork == info.fork;

        let shared = &self.state.shared;
        shared.set_remote_fork(msg.fork);
        shared.set_remote_length(msg.length);
        shared.set_remote_can_upgrade(msg.can_upgrade);
        shared.set_remote_synced(true);
        shared.set_length_acked(if same_fork { msg.remote_length } else { 0 });
        self.state.remote_uploading = msg.uploading;
        self.state.remote_downloading = msg.downloading;

        if first_sync {
            self.outgoing
                .push_back(vec![Message::Synchronize(Synchronize {
                    fork: info.fork,
                    length: info.length,
                    remote_length: self.state.shared.remote_length(),
                    can_upgrade: self.state.can_upgrade,
                    uploading: true,
                    downloading: true,
                })]);
        }

        self.maybe_request_upgrade(inner);

        // Wake any `Hypercore::update` waiting on this peer's advertised length.
        inner.send_event(events::PeerSync {});
    }

    /// Ask this peer to grow our verified length, if it has advertised a longer one and we are
    /// not already waiting on an upgrade from it. Returns whether a request was queued.
    ///
    /// Called both when a [`Synchronize`] arrives and once per poll, because the conditions can
    /// become true after the fact — most often `length_acked` catching up to our own length.
    /// This is the analogue of JS's `_updatePeerNonPrimary` upgrade branch.
    fn maybe_request_upgrade(&mut self, inner: &HypercoreInner) -> bool {
        let info = inner.info();
        let shared = &self.state.shared;
        if !shared.remote_synced() || shared.upgrade_inflight() {
            return false;
        }
        let remote_length = shared.remote_length();
        if remote_length <= info.length || shared.length_acked() != info.length {
            return false;
        }
        // Don't re-send the same request on every poll.
        let request = (info.fork, info.length, remote_length);
        if self.last_upgrade_requested == Some(request) {
            return false;
        }
        self.last_upgrade_requested = Some(request);
        shared.set_upgrade_inflight(true);
        self.outgoing.push_back(vec![Message::Request(Request {
            id: 1,
            fork: info.fork,
            hash: None,
            block: None,
            seek: None,
            upgrade: Some(RequestUpgrade {
                start: info.length,
                length: remote_length - info.length,
            }),
            manifest: false,
            priority: 42,
        })]);
        true
    }

    fn on_core_event(&mut self, event: events::Event, inner: &HypercoreInner) {
        match event {
            events::Event::Get(evt) => {
                if self.state.shared.remote_length() > evt.index
                    && self.state.remote_bitfield.get(evt.index)
                {
                    self.pending_request_indices.push_back(evt.index);
                }
            }
            events::Event::Have(evt) => {
                self.outgoing.push_back(vec![Message::Range(Range {
                    drop: evt.drop,
                    start: evt.start,
                    length: evt.length,
                })]);
            }
            events::Event::DataUpgrade(_) => {
                let info = inner.info();
                self.outgoing
                    .push_back(vec![Message::Synchronize(Synchronize {
                        fork: info.fork,
                        length: info.length,
                        remote_length: self.state.shared.remote_length(),
                        downloading: true,
                        uploading: true,
                        can_upgrade: self.state.can_upgrade,
                    })]);
            }
            // Pure wake-ups. `Upgrade` is handled by the `maybe_request_upgrade` re-check that
            // runs every poll, and `PeerSync` is only ever of interest to `Hypercore::update`.
            events::Event::PeerSync(_) | events::Event::Upgrade(_) => {}
        }
    }
}

// ── ConnectionReplicator ───────────────────────────────────────────────────────

/// Drives replication for a single peer connection. Used internally by
/// [`Replicator`].
struct ConnectionReplicator {
    inner: HypercoreInner,
    protocol: Protocol,
    discovery_key: [u8; 32],
    public_key: [u8; 32],
    pending_open: Option<Pin<Box<dyn Future<Output = io::Result<()>> + Send>>>,
    channel_state: Option<ChannelState>,
}

impl ConnectionReplicator {
    fn new(inner: HypercoreInner, stream: impl CipherTrait + 'static) -> Self {
        let protocol = Protocol::new(Box::new(stream));
        let public_key = inner.key_pair().public.to_bytes();
        let discovery_key = discovery_key(&public_key);
        Self {
            inner,
            protocol,
            discovery_key,
            public_key,
            pending_open: None,
            channel_state: None,
        }
    }

    fn poll_inner(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), HypercoreError>> {
        if let Some(ref mut fut) = self.pending_open {
            match fut.as_mut().poll(cx) {
                Poll::Ready(Ok(())) => self.pending_open = None,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                Poll::Pending => return Poll::Pending,
            }
        }

        match Pin::new(&mut self.protocol).poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                self.on_protocol_event(event);
                cx.waker().wake_by_ref();
            }
            Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(e.into())),
            Poll::Ready(None) => return Poll::Ready(Ok(())),
            Poll::Pending => {}
        }

        if let Some(ref mut cs) = self.channel_state {
            match cs.poll(cx, &self.inner) {
                Poll::Ready(Ok(())) => self.channel_state = None,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => {}
            }
        }

        Poll::Pending
    }

    fn on_protocol_event(&mut self, event: hypercore_protocol::Event) {
        match event {
            hypercore_protocol::Event::Handshake(_) => {
                if self.protocol.is_initiator() {
                    self.pending_open = Some(Box::pin(self.protocol.open(self.public_key)));
                }
            }
            hypercore_protocol::Event::DiscoveryKey(dkey) => {
                if self.discovery_key == dkey {
                    self.pending_open = Some(Box::pin(self.protocol.open(self.public_key)));
                } else {
                    warn!("Got discovery key for different core: {dkey:?}");
                }
            }
            hypercore_protocol::Event::Channel(channel) => {
                if self.discovery_key == *channel.discovery_key() {
                    self.channel_state = Some(ChannelState::new(channel, &self.inner));
                } else {
                    error!("Wrong discovery key?");
                }
            }
            hypercore_protocol::Event::Close(_) => {}
            _ => {}
        }
    }
}

impl Future for ConnectionReplicator {
    type Output = Result<(), HypercoreError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll_inner(cx)
    }
}

// ── Replicator ─────────────────────────────────────────────────────────────────

type BoxReplicatorStream = Pin<Box<dyn Stream<Item = ConnectionReplicator> + Send>>;

/// Drives replication for one or more peer connections.
///
/// Created by [`Hypercore::replicator`]. Add connections via
/// [`with_connection`](Replicator::with_connection) or whole connection streams
/// via [`with_connection_stream`](Replicator::with_connection_stream), then
/// `.await` to drive all replication. Resolves when all connections have closed
/// and all connection streams have ended.
pub struct Replicator {
    inner: HypercoreInner,
    active: FuturesUnordered<ConnectionReplicator>,
    pending: SelectAll<BoxReplicatorStream>,
}

impl std::fmt::Debug for Replicator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Replicator")
            .field("active_count", &self.active.len())
            .finish_non_exhaustive()
    }
}

impl Replicator {
    fn new(inner: HypercoreInner) -> Self {
        Self {
            inner,
            active: FuturesUnordered::new(),
            pending: SelectAll::new(),
        }
    }

    /// Add a single connection to replicate over.
    pub fn with_connection(self, stream: impl CipherTrait + 'static) -> Self {
        self.active
            .push(ConnectionReplicator::new(self.inner.clone(), stream));
        self
    }

    /// Add a stream of connections. Each connection yielded by the stream will
    /// be replicated in parallel with all others.
    ///
    /// ```rust,ignore
    /// core.replicator()
    ///     .with_connection_stream(swarm.connections().filter_map(|r| async move {
    ///         r.ok().map(|e| e.connection)
    ///     }))
    ///     .await?;
    /// ```
    pub fn with_connection_stream<S, C>(mut self, stream: S) -> Self
    where
        S: Stream<Item = C> + Send + 'static,
        C: CipherTrait + 'static,
    {
        let inner = self.inner.clone();
        let boxed: BoxReplicatorStream =
            Box::pin(stream.map(move |conn| ConnectionReplicator::new(inner.clone(), conn)));
        self.pending.push(boxed);
        self
    }
}

impl Future for Replicator {
    type Output = Result<(), HypercoreError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // Drain pending connection streams → push new ConnectionReplicators into active.
        loop {
            let pending_result = Pin::new(&mut this.pending).poll_next(cx);
            trace!(
                "[replicator] pending.poll_next = {:?}",
                match &pending_result {
                    Poll::Ready(Some(_)) => "Ready(Some(conn))",
                    Poll::Ready(None) => "Ready(None)",
                    Poll::Pending => "Pending",
                }
            );
            match pending_result {
                Poll::Ready(Some(rep)) => this.active.push(rep),
                Poll::Ready(None) | Poll::Pending => break,
            }
        }

        // Drive all active connection replicators.
        loop {
            match Pin::new(&mut this.active).poll_next(cx) {
                Poll::Ready(Some(Ok(()))) => {}
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(e)),
                Poll::Ready(None) | Poll::Pending => break,
            }
        }

        if this.pending.is_empty() && this.active.is_empty() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}

// ── ChannelReplicator ──────────────────────────────────────────────────────────

/// Drives replication for a single core over a [`Channel`] already opened on a
/// [`Protocol`] someone else owns (e.g. a corestore-like multiplexer serving
/// several cores over one physical connection).
///
/// Unlike [`ConnectionReplicator`], this does not create or drive a `Protocol`
/// itself — it only runs the per-core wire protocol over an already-open
/// channel. Returned as an opaque `impl Future` from [`Hypercore::attach_channel`].
struct ChannelReplicator {
    inner: HypercoreInner,
    state: ChannelState,
}

impl ChannelReplicator {
    fn new(inner: HypercoreInner, channel: Channel) -> Self {
        Self {
            state: ChannelState::new(channel, &inner),
            inner,
        }
    }
}

impl Future for ChannelReplicator {
    type Output = Result<(), HypercoreError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.state.poll(cx, &this.inner)
    }
}

// ── Hypercore::replicator / replicate ─────────────────────────────────────────
impl Hypercore {
    /// Create a [`Replicator`] for this core. Add connections or connection
    /// streams, then `.await` to drive all replication.
    ///
    /// ```rust,ignore
    /// // Single connection
    /// core.replicator().with_connection(stream).await?;
    ///
    /// // Stream of connections (e.g. from hyperswarm)
    /// core.replicator()
    ///     .with_connection_stream(swarm.connections().filter_map(|r| async move {
    ///         r.ok().map(|e| e.connection)
    ///     }))
    ///     .await?;
    /// ```
    pub fn replicator(&self) -> Replicator {
        Replicator::new(self.inner.clone())
    }

    /// Shorthand for `self.replicator().with_connection(stream)`.
    pub fn replicate(&self, stream: impl CipherTrait + 'static) -> Replicator {
        self.replicator().with_connection(stream)
    }

    /// Attach a replicator to this core so that [`Hypercore::get`] automatically
    /// drives replication while waiting for missing blocks.
    ///
    /// Once attached, a `get` for a block that is not yet locally available will
    /// block (without spinning) until replication delivers it, rather than
    /// returning `None` immediately.
    ///
    /// Only one replicator can be attached at a time; calling this again replaces
    /// the previous one.
    pub fn attach_replicator(&self, replicator: Replicator) {
        *self.inner.background.lock().unwrap() = Some(Box::pin(replicator));
    }

    /// Drive this core's replication over a [`Channel`] already opened on a
    /// [`Protocol`] the caller owns and drives itself.
    ///
    /// Use this (instead of [`Hypercore::replicate`]) when multiplexing several
    /// cores over one physical connection: the caller owns a single `Protocol`
    /// for the connection, opens a channel per core's discovery key, and calls
    /// this once per core with the resulting channel. Resolves when the channel
    /// closes.
    pub fn attach_channel(
        &self,
        channel: Channel,
    ) -> impl Future<Output = Result<(), HypercoreError>> + Send + 'static {
        ChannelReplicator::new(self.inner.clone(), channel)
    }
}
