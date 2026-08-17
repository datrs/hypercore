//! Fetching the newest state of a core from its peers.
//!
//! See [`Hypercore::update`].

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use futures::Stream as _;

use crate::{
    Hypercore, HypercoreError, Info,
    core::inner::{BackgroundFuture, HypercoreInner},
    replication::{PeerSyncState, events},
};

/// Options for [`Hypercore::update`].
#[derive(Debug, Clone)]
pub struct UpdateOptions {
    /// Wait for peers to connect and tell us their length.
    ///
    /// When `false`, [`Hypercore::update`] answers from what currently-connected peers have
    /// already said and never blocks on a peer arriving — so with no peers it resolves
    /// `Ok(false)` immediately.
    ///
    /// Defaults to `true`.
    pub wait: bool,
    /// Run even on a writable core, which otherwise resolves `Ok(false)` without doing
    /// anything. Defaults to `false`.
    pub force: bool,
    /// Give up after this long and return [`HypercoreError::Timeout`].
    ///
    /// `None` (the default) means wait indefinitely. Note that `wait: true` with no timeout
    /// and no peers really does block forever, exactly like Javascript's
    /// `core.update({ wait: true })`.
    pub timeout: Option<Duration>,
}

impl Default for UpdateOptions {
    fn default() -> Self {
        Self {
            wait: true,
            force: false,
            timeout: None,
        }
    }
}

/// State captured on the first poll, against which "did we grow?" is judged.
struct Snapshot {
    fork: u64,
    length: u64,
}

/// Future returned by [`Hypercore::update`].
///
/// Resolves `Ok(true)` when the core's verified length (or fork) moved, `Ok(false)` when the
/// peers confirmed there is nothing newer, and [`HypercoreError::Timeout`] when
/// [`UpdateOptions::timeout`] elapsed with nobody having answered.
pub struct UpdateFuture {
    inner: HypercoreInner,
    opts: UpdateOptions,
    /// `None` until the first poll, which is also what makes the first poll special.
    snapshot: Option<Snapshot>,
    events: Option<async_broadcast::Receiver<events::Event>>,
    /// Replicator attached via [`Hypercore::attach_replicator`], if any. Driven in-band, the
    /// same way [`crate::Hypercore::get`] does it.
    background: BackgroundFuture,
    sleep: Option<Pin<Box<tokio::time::Sleep>>>,
}

impl std::fmt::Debug for UpdateFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateFuture")
            .field("opts", &self.opts)
            .finish_non_exhaustive()
    }
}

impl UpdateFuture {
    pub(crate) fn new(inner: HypercoreInner, opts: UpdateOptions) -> Self {
        let background = inner.background.clone();
        Self {
            inner,
            opts,
            snapshot: None,
            events: None,
            background,
            sleep: None,
        }
    }

    /// Drive an attached replicator, so that `update` works under
    /// [`Hypercore::attach_replicator`] as well as under an externally driven one.
    ///
    /// `Ok(None)` means no replicator is attached — replication, if any, is driven elsewhere
    /// and we can conclude nothing from that. `Ok(Some(true))` means one was attached and has
    /// now run to completion, so no further peer will ever appear through it.
    fn poll_background(&self, cx: &mut Context<'_>) -> Result<Option<bool>, HypercoreError> {
        let mut background = self.background.lock().unwrap();
        let Some(fut) = background.as_mut() else {
            return Ok(None);
        };
        match fut.as_mut().poll(cx) {
            Poll::Ready(Ok(())) => {
                *background = None;
                Ok(Some(true))
            }
            Poll::Ready(Err(e)) => Err(e),
            Poll::Pending => Ok(Some(false)),
        }
    }

    /// Drain pending events. Payloads are ignored on purpose: the event channel drops
    /// messages under load, so events are treated purely as "something changed, look again".
    fn drain_events(&mut self, cx: &mut Context<'_>) {
        let Some(events) = self.events.as_mut() else {
            return;
        };
        while let Poll::Ready(Some(_)) = Pin::new(&mut *events).poll_next(cx) {}
    }

}

/// Whether some peer might still upgrade us, i.e. whether it is too early to answer `false`.
/// A port of Javascript's `_checkUpgradeIfAvailable` (`lib/replicator.js`).
///
/// Every peer gets a say: one peer having nothing newer says nothing about the others, so a
/// useless peer must `continue` rather than end the search. Javascript only consults the
/// first `MAX_PEERS_UPGRADE` (3) peers; we check them all, since a core here has a handful at
/// most.
///
/// Free-standing (rather than a method) so it can be unit tested against hand-built peer
/// state — the multi-peer cases are impractical to stage deterministically over real
/// connections, because which peer registers first is a race.
fn any_peer_may_upgrade(
    peers: &[Arc<PeerSyncState>],
    info: &Info,
    snapshot: &Snapshot,
    wait: bool,
) -> bool {
    for peer in peers {
        // Hasn't told us anything yet. Under `wait: false` we don't hang around for it.
        if !peer.remote_synced() {
            if wait {
                return true;
            }
            continue;
        }
        // We have nothing at all and the peer does: it can certainly upgrade us.
        if info.length == 0 && peer.remote_length() > 0 {
            return true;
        }
        // This peer has nothing we don't already have, or is on another fork.
        if peer.remote_length() <= snapshot.length || peer.remote_fork() != snapshot.fork {
            continue;
        }
        // It hasn't acked our current length, so its `can_upgrade` claim is stale.
        if peer.length_acked() != info.length && peer.remote_fork() == info.fork {
            return true;
        }
        if peer.remote_can_upgrade() || peer.upgrade_inflight() {
            return true;
        }
    }
    false
}

impl Future for UpdateFuture {
    type Output = Result<bool, HypercoreError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // ── First poll ─────────────────────────────────────────────────────────
        if this.snapshot.is_none() {
            let info = this.inner.info();
            if info.writeable && !this.opts.force {
                return Poll::Ready(Ok(false));
            }
            // Subscribe before reading any state, so no wake-up can slip between the two.
            this.events = Some(this.inner.event_subscribe());
            this.snapshot = Some(Snapshot {
                fork: info.fork,
                length: info.length,
            });
            // Nudge replicators that are connected but idle into re-checking whether they
            // should ask for an upgrade.
            this.inner.send_event(events::Upgrade {});
            this.sleep = this
                .opts
                .timeout
                .map(|timeout| Box::pin(tokio::time::sleep(timeout)));
        }

        let background = match this.poll_background(cx) {
            Ok(background) => background,
            Err(e) => return Poll::Ready(Err(e)),
        };

        this.drain_events(cx);

        // ── Did we grow? ───────────────────────────────────────────────────────
        let info = this.inner.info();
        let snapshot = this.snapshot.as_ref().expect("set above");
        if info.length != snapshot.length || info.fork != snapshot.fork {
            return Poll::Ready(Ok(true));
        }

        // ── Could anyone still grow us? ────────────────────────────────────────
        let peers = this.inner.peers();
        if peers.is_empty() && !this.opts.wait {
            return Poll::Ready(Ok(false));
        }
        let snapshot = this.snapshot.as_ref().expect("set above");
        if !peers.is_empty() && !any_peer_may_upgrade(&peers, &info, snapshot, this.opts.wait) {
            return Poll::Ready(Ok(false));
        }

        // An attached replicator that ran to completion means every connection it owned has
        // closed, so no peer is ever going to show up through it.
        if background == Some(true) && peers.is_empty() {
            return Poll::Ready(Ok(false));
        }

        // ── Give up? ───────────────────────────────────────────────────────────
        if let Some(sleep) = this.sleep.as_mut()
            && sleep.as_mut().poll(cx).is_ready()
        {
            return Poll::Ready(Err(HypercoreError::Timeout {
                context: "Hypercore::update timed out waiting for a peer".to_string(),
            }));
        }

        Poll::Pending
    }
}

impl Hypercore {
    /// Fetch the newest state of this core from its peers.
    ///
    /// Resolves `Ok(true)` if the core's verified length grew, `Ok(false)` if the peers
    /// confirmed there is nothing newer (or the core is writable — pass
    /// [`UpdateOptions::force`] to update one anyway), and [`HypercoreError::Timeout`] if
    /// [`UpdateOptions::timeout`] elapsed before any peer answered.
    ///
    /// This is the "is there anything new?" primitive: it asks peers to grow this core's
    /// *verified* length, which is a different operation from downloading blocks. Use
    /// [`Hypercore::get`] for the latter.
    ///
    /// Replication must be driven by someone: either a spawned [`Replicator`](super::Replicator)
    /// / channel future, or one attached with [`Hypercore::attach_replicator`], in which case
    /// this future drives it while it waits.
    ///
    /// ```rust,ignore
    /// // Block until a peer connects and tells us where it is, but no longer than 5s.
    /// let grew = core.update(UpdateOptions {
    ///     timeout: Some(Duration::from_secs(5)),
    ///     ..Default::default()
    /// }).await?;
    /// ```
    pub fn update(&self, opts: UpdateOptions) -> UpdateFuture {
        UpdateFuture::new(self.inner.clone(), opts)
    }

    /// The number of peers currently replicating this core.
    pub fn peer_count(&self) -> usize {
        self.inner.peers().len()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Peer state as it would be after a `Synchronize` from a peer at `remote_length`.
    fn synced_peer(remote_length: u64, our_length: u64) -> Arc<PeerSyncState> {
        let peer = Arc::new(PeerSyncState::default());
        peer.set_remote_synced(true);
        peer.set_remote_length(remote_length);
        peer.set_length_acked(our_length);
        peer.set_remote_can_upgrade(true);
        peer
    }

    fn unsynced_peer() -> Arc<PeerSyncState> {
        Arc::new(PeerSyncState::default())
    }

    fn info(length: u64) -> Info {
        Info {
            length,
            byte_length: 0,
            contiguous_length: length,
            fork: 0,
            writeable: false,
        }
    }

    fn snapshot(length: u64) -> Snapshot {
        Snapshot { fork: 0, length }
    }

    /// The case the peer registry exists for: a peer with nothing newer must not end the
    /// search, because a later peer may still have more. Consulting only `peers[0]` here
    /// would wrongly answer "nobody can upgrade us".
    #[test]
    fn useless_peer_does_not_mask_a_useful_later_one() {
        let peers = vec![synced_peer(2, 2), synced_peer(5, 2)];
        assert!(any_peer_may_upgrade(&peers, &info(2), &snapshot(2), true));
        // ...and the same two peers in the other order, so this cannot pass by luck.
        let peers = vec![synced_peer(5, 2), synced_peer(2, 2)];
        assert!(any_peer_may_upgrade(&peers, &info(2), &snapshot(2), true));
    }

    #[test]
    fn all_peers_in_sync_means_nothing_newer() {
        let peers = vec![synced_peer(2, 2), synced_peer(2, 2), synced_peer(1, 2)];
        assert!(!any_peer_may_upgrade(&peers, &info(2), &snapshot(2), true));
    }

    #[test]
    fn no_peers_means_nothing_newer() {
        assert!(!any_peer_may_upgrade(&[], &info(2), &snapshot(2), true));
    }

    /// An unsynced peer is worth waiting for, but only when the caller asked to wait.
    #[test]
    fn unsynced_peer_is_awaited_only_when_waiting() {
        let peers = vec![synced_peer(2, 2), unsynced_peer()];
        assert!(any_peer_may_upgrade(&peers, &info(2), &snapshot(2), true));
        assert!(!any_peer_may_upgrade(&peers, &info(2), &snapshot(2), false));
    }

    /// An empty core with a peer that has anything at all is always worth waiting for.
    #[test]
    fn empty_core_waits_for_any_peer_with_data() {
        let peers = vec![synced_peer(1, 0)];
        assert!(any_peer_may_upgrade(&peers, &info(0), &snapshot(0), true));
    }

    /// A peer on another fork cannot help us, even though it looks longer.
    #[test]
    fn peer_on_another_fork_cannot_help() {
        let peer = synced_peer(9, 2);
        peer.set_remote_fork(7);
        assert!(!any_peer_may_upgrade(&[peer], &info(2), &snapshot(2), true));
    }

    /// A peer that has more but has not acked our current length: its `can_upgrade` claim is
    /// stale, so we wait for the next sync rather than concluding either way.
    #[test]
    fn peer_with_stale_ack_is_awaited() {
        let peer = synced_peer(5, 2);
        peer.set_length_acked(1); // has not caught up to our length of 2
        peer.set_remote_can_upgrade(false);
        assert!(any_peer_may_upgrade(&[peer], &info(2), &snapshot(2), true));
    }

    /// A peer that has more, has acked us, but says it cannot serve the upgrade, and has
    /// nothing in flight, is a dead end.
    #[test]
    fn peer_that_cannot_upgrade_and_has_nothing_inflight_is_a_dead_end() {
        let peer = synced_peer(5, 2);
        peer.set_remote_can_upgrade(false);
        assert!(!any_peer_may_upgrade(
            std::slice::from_ref(&peer),
            &info(2),
            &snapshot(2),
            true
        ));
        // ...unless we are already waiting on a response from it.
        peer.set_upgrade_inflight(true);
        assert!(any_peer_may_upgrade(
            std::slice::from_ref(&peer),
            &info(2),
            &snapshot(2),
            true
        ));
    }
}
