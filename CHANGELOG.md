## 2020-02-19, Version 0.11.0
### Commits
- [[`f2baf805d5`](https://github.com/datrs/hypercore/commit/f2baf805d5477c768f32ca2cf7faae4d9d284686)] (cargo-release) version 0.11.0 (Bruno Tavares)
- [[`31dfdd15f2`](https://github.com/datrs/hypercore/commit/31dfdd15f27356780d75fa126bd8a8d464fefc39)] Merge pull request #95 from bltavares/send (Bruno Tavares)
- [[`46be5197a2`](https://github.com/datrs/hypercore/commit/46be5197a2398e04d413ebfa65fcb6f830dedf0f)] Use published version (Bruno Tavares)
- [[`d4905b11cf`](https://github.com/datrs/hypercore/commit/d4905b11cf83871db98c118c373d52626e6b1c78)] Point to merkle-tree-stream that is Send while new version is to be released (Bruno Tavares)
- [[`40caf92ec2`](https://github.com/datrs/hypercore/commit/40caf92ec2c357a08ddeec03f9d4ba34a723eeaf)] Replace all Rc with Arc in code. Needs to update dependencies (Bruno Tavares)
- [[`2dc8008a55`](https://github.com/datrs/hypercore/commit/2dc8008a5542713a2569cfb115a006dee34bbca6)] example to ensure structs are send (Bruno Tavares)
- [[`f77fe7b025`](https://github.com/datrs/hypercore/commit/f77fe7b0257bd5f0e7007c012bc68bc1d75eda05)] fix readme link (#88) (nasa)
- [[`82e48f0c7d`](https://github.com/datrs/hypercore/commit/82e48f0c7d2330f0ed845dac30db46a02d5f7c48)] Update memory-pager requirement from 0.8.0 to 0.9.0 (dependabot-preview[bot])
- [[`580dff64c5`](https://github.com/datrs/hypercore/commit/580dff64c50377e6fc51dbed701c2dc26a2693a2)] Update sparse-bitfield requirement from 0.8.1 to 0.10.0 (dependabot-preview[bot])
- [[`7eda3504d6`](https://github.com/datrs/hypercore/commit/7eda3504d61de0f1423d0efa272587fe8b0a1650)] Merge pull request #81 from bltavares/discovery-key-hash (Szabolcs Berecz)
- [[`1edf42f790`](https://github.com/datrs/hypercore/commit/1edf42f79007924b79e7b1b99a7e9d66abc3b4e9)] Implements discoveryKey from hypercore-crypto (Bruno Tavares)
- [[`aedef0b149`](https://github.com/datrs/hypercore/commit/aedef0b149de042313245c2baab0948da3390aef)] Update changelog (Yoshua Wuyts)

### Stats
```diff
 CHANGELOG.md         | 26 ++++++++++++++++++++++++++
 Cargo.toml           |  9 +++++----
 README.md            |  2 +-
 examples/async.rs    | 30 ++++++++++++++++++++++++++++++
 src/crypto/hash.rs   | 42 +++++++++++++++++++++++++++++-------------
 src/crypto/merkle.rs | 10 +++++-----
 src/feed.rs          |  4 ++--
 7 files changed, 98 insertions(+), 25 deletions(-)
```


## 2018-12-22, Version 0.9.0
### Commits
- [[`9c2b07fca6`](https://github.com/datrs/hypercore/commit/9c2b07fca68bb34046551f0fd152aa7f97a33fb6)] (cargo-release) version 0.9.0 (Yoshua Wuyts)
- [[`86e241f9e0`](https://github.com/datrs/hypercore/commit/86e241f9e02e3583445fcb43fcc28295eae1cd31)] 🙋 Implement feed auditing (#55) (Tim Deeb-Swihart)
- [[`5840a3a6a9`](https://github.com/datrs/hypercore/commit/5840a3a6a90f47ba89662687a374f070f3172c69)] Update rand requirement from 0.5.5 to 0.6.0 (#49) (dependabot[bot])
- [[`1628057868`](https://github.com/datrs/hypercore/commit/162805786831866ea611cfe97e85def690614fa6)] use tree_index functions (#48) (Yoshua Wuyts)
- [[`f66fbb3543`](https://github.com/datrs/hypercore/commit/f66fbb354376681062697ffd2be18da2224cb1b9)] Update merkle-tree-stream requirement from 0.7.0 to 0.8.0 (#46) (dependabot[bot])
- [[`343df6f991`](https://github.com/datrs/hypercore/commit/343df6f991b0fbe5f50a7d95b632b3c60e5dfa54)] Update changelog (Yoshua Wuyts)

### Stats
```diff
 CHANGELOG.md           | 26 +++++++++++++++++++-
 Cargo.toml             |  6 ++--
 src/audit.rs           | 20 +++++++++++++++-
 src/bitfield/mod.rs    | 21 ++++++++++------
 src/crypto/key_pair.rs |  2 +-
 src/crypto/merkle.rs   | 14 ++++++----
 src/feed.rs            | 46 ++++++++++++++++++++++++++++------
 src/lib.rs             |  1 +-
 src/storage/mod.rs     | 10 +++++--
 src/storage/node.rs    |  2 +-
 tests/feed.rs          | 69 +++++++++++++++++++++++++++++++++++++++++++++++++++-
 11 files changed, 191 insertions(+), 26 deletions(-)
```


## 2018-12-22, Version 0.9.0
### Commits
- [[`9c2b07fca6`](https://github.com/datrs/hypercore/commit/9c2b07fca68bb34046551f0fd152aa7f97a33fb6)] (cargo-release) version 0.9.0 (Yoshua Wuyts)
- [[`86e241f9e0`](https://github.com/datrs/hypercore/commit/86e241f9e02e3583445fcb43fcc28295eae1cd31)] 🙋 Implement feed auditing (#55) (Tim Deeb-Swihart)
- [[`5840a3a6a9`](https://github.com/datrs/hypercore/commit/5840a3a6a90f47ba89662687a374f070f3172c69)] Update rand requirement from 0.5.5 to 0.6.0 (#49) (dependabot[bot])
- [[`1628057868`](https://github.com/datrs/hypercore/commit/162805786831866ea611cfe97e85def690614fa6)] use tree_index functions (#48) (Yoshua Wuyts)
- [[`f66fbb3543`](https://github.com/datrs/hypercore/commit/f66fbb354376681062697ffd2be18da2224cb1b9)] Update merkle-tree-stream requirement from 0.7.0 to 0.8.0 (#46) (dependabot[bot])
- [[`343df6f991`](https://github.com/datrs/hypercore/commit/343df6f991b0fbe5f50a7d95b632b3c60e5dfa54)] Update changelog (Yoshua Wuyts)

### Stats
```diff
 CHANGELOG.md           | 26 +++++++++++++++++++-
 Cargo.toml             |  6 ++--
 src/audit.rs           | 20 +++++++++++++++-
 src/bitfield/mod.rs    | 21 ++++++++++------
 src/crypto/key_pair.rs |  2 +-
 src/crypto/merkle.rs   | 14 ++++++----
 src/feed.rs            | 46 ++++++++++++++++++++++++++++------
 src/lib.rs             |  1 +-
 src/storage/mod.rs     | 10 +++++--
 src/storage/node.rs    |  2 +-
 tests/feed.rs          | 69 +++++++++++++++++++++++++++++++++++++++++++++++++++-
 11 files changed, 191 insertions(+), 26 deletions(-)
```


## 2018-10-28, Version 0.8.1
### Commits
- [[`938d2816cc`](https://github.com/datrs/hypercore/commit/938d2816cc63e4dd8964139baa56be2dd28e72d5)] (cargo-release) version 0.8.1 (Yoshua Wuyts)
- [[`79fd7a8141`](https://github.com/datrs/hypercore/commit/79fd7a8141096606b4124c7d59dede2a4021b3fb)] Stricter lints (#45) (Yoshua Wuyts)
- [[`96b3af825d`](https://github.com/datrs/hypercore/commit/96b3af825ddc5c69364fe92c71d8498f4a00a2dc)] use spec compatible constants (#44) (Yoshua Wuyts)
- [[`ac8ef53b0c`](https://github.com/datrs/hypercore/commit/ac8ef53b0cd45f0b935ab83dde4f750eb91a07e8)] Update changelog (Yoshua Wuyts)

### Stats
```diff
 CHANGELOG.md           | 33 +++++++++++++++++++++++++++++++++
 Cargo.toml             | 10 +++++++++-
 src/crypto/hash.rs     |  8 ++++----
 src/crypto/key_pair.rs |  2 +-
 src/crypto/merkle.rs   |  4 ++--
 src/feed.rs            | 20 ++++++++++----------
 src/feed_builder.rs    | 10 +++++-----
 src/lib.rs             | 19 +++++++++++--------
 src/prelude.rs         |  4 ++--
 src/proof.rs           |  4 ++--
 src/storage/mod.rs     |  2 +-
 src/storage/node.rs    |  4 ++--
 src/storage/persist.rs |  2 +-
 13 files changed, 83 insertions(+), 39 deletions(-)
```


## 2018-10-18, Version 0.8.0
### Commits
- [[`048921b077`](https://github.com/datrs/hypercore/commit/048921b077d02963e70a881fa780e6e96c347d50)] (cargo-release) version 0.8.0 (Yoshua Wuyts)
- [[`54ceb55e7b`](https://github.com/datrs/hypercore/commit/54ceb55e7bf6c5c037b3849c53bc082bc57e0ee4)] travis master only builds (Yoshua Wuyts)
- [[`1a06b5862d`](https://github.com/datrs/hypercore/commit/1a06b5862d371120dc2e1695e5d1764721707e29)] upgrade (#43) (Yoshua Wuyts)
- [[`2fda376767`](https://github.com/datrs/hypercore/commit/2fda376767efe3b61fe2f3bc46a431340cf984a2)] tests/helpers -> tests/common (#38) (Yoshua Wuyts)
- [[`d48e5570fa`](https://github.com/datrs/hypercore/commit/d48e5570fa659b38519a54288b6019205cb48276)] Keep up with modern times in clippy invocation (#35) (Szabolcs Berecz)
- [[`a62a21b249`](https://github.com/datrs/hypercore/commit/a62a21b24953f6b1da5cfc902abef6914f0b7950)] Update quickcheck requirement from 0.6.2 to 0.7.1 (#33) (Szabolcs Berecz)
- [[`3bbe87db8d`](https://github.com/datrs/hypercore/commit/3bbe87db8d448e8fbc7a73a99b07ff39ec09c1e9)] Update changelog (Yoshua Wuyts)

### Stats
```diff
 .github/ISSUE_TEMPLATE.md                 | 40 +++---------------------------
 .github/ISSUE_TEMPLATE/bug_report.md      | 23 +++++++++++++++++-
 .github/ISSUE_TEMPLATE/feature_request.md | 43 ++++++++++++++++++++++++++++++++-
 .github/ISSUE_TEMPLATE/question.md        | 18 +++++++++++++-
 .travis.yml                               | 24 +++++++++---------
 CHANGELOG.md                              | 25 +++++++++++++++++++-
 Cargo.toml                                | 28 ++++++++++-----------
 README.md                                 | 23 +++++++++++++++--
 src/feed.rs                               |  2 +-
 src/lib.rs                                | 23 +++++++++++++----
 src/replicate/peer.rs                     |  2 +-
 src/storage/mod.rs                        |  2 +-
 tests/common/mod.rs                       | 15 +++++++++++-
 tests/feed.rs                             | 29 +++++++++++++++++++---
 tests/helpers.rs                          | 34 +-------------------------
 tests/model.rs                            |  6 ++--
 tests/regression.rs                       |  4 +--
 17 files changed, 232 insertions(+), 109 deletions(-)
```


## 2018-09-03, Version 0.7.1
### Commits
- [[`43ad5d3c9a`](https://github.com/datrs/hypercore/commit/43ad5d3c9accd9e4faa63fc5fe35b5c74997d503)] (cargo-release) version 0.7.1 (Yoshua Wuyts)
- [[`cb2cfac275`](https://github.com/datrs/hypercore/commit/cb2cfac2757a50600886251b608ab349bdc6daf4)] Update ed25519_dalek to 0.8 and rand to 0.5 (#30) (Luiz Irber)
- [[`ade97ddfe3`](https://github.com/datrs/hypercore/commit/ade97ddfe3310edbff11057740ebd03ed73075b4)] Update memory-pager requirement from 0.7.0 to 0.8.0 (dependabot[bot])
- [[`420a3b19b0`](https://github.com/datrs/hypercore/commit/420a3b19b0daa7d32d96c3c67045adab10c0f38d)] Upgrade random-access-storage (#26) (Szabolcs Berecz)
- [[`7421f677eb`](https://github.com/datrs/hypercore/commit/7421f677eb200cfa2cceb98c027408e29cc526ee)] update changelog (Yoshua Wuyts)

### Stats
```diff
 CHANGELOG.md           | 26 ++++++++++++++++++++++++++
 Cargo.toml             | 14 +++++++-------
 benches/bench.rs       |  8 +++-----
 src/crypto/key_pair.rs |  4 ++--
 src/feed.rs            | 16 ++++++++--------
 src/feed_builder.rs    |  6 +++---
 src/storage/mod.rs     | 40 ++++++++++++++++++++--------------------
 src/storage/persist.rs |  4 ++--
 tests/compat.rs        |  6 +++---
 tests/feed.rs          | 10 +++++-----
 tests/helpers.rs       |  8 ++++----
 11 files changed, 83 insertions(+), 59 deletions(-)
```


## 2018-08-25, Version 0.7.0
### Commits
- [[`c4c5986191`](https://github.com/datrs/hypercore/commits/c4c5986191ab9dc07443264c65d0f2edc6971439)] (cargo-release) version 0.7.0 (Yoshua Wuyts)
- [[`7d6bde061c`](https://github.com/datrs/hypercore/commits/7d6bde061c6724a216f59ecd90970722b0c0f118)] Storage: implement keypair read/write (#18)
- [[`d027f37ed8`](https://github.com/datrs/hypercore/commits/d027f37ed8aa5c9a487a7e0260fa1ca0cd089011)] Update sparse-bitfield requirement from 0.4.0 to 0.8.0 (#20)
- [[`5d9b05f029`](https://github.com/datrs/hypercore/commits/5d9b05f029f2e1427770c4169794ce1cccd70ec5)] Update memory-pager requirement from 0.4.5 to 0.7.0
- [[`73a3f28e26`](https://github.com/datrs/hypercore/commits/73a3f28e26957c627254ed024092df7ae057d277)] Update sleep-parser requirement from 0.4.0 to 0.6.0
- [[`566b7a1021`](https://github.com/datrs/hypercore/commits/566b7a1021a36e7dc82ca22091ee21df88870d57)] Upgrade to latest random-access-storage (#17)
- [[`e086e60942`](https://github.com/datrs/hypercore/commits/e086e609428d015bc831384ff3e16a8c9a295bc7)] Add rustfmt back to travis (#19)
- [[`eb5edfba43`](https://github.com/datrs/hypercore/commits/eb5edfba438f8617d076f3a3f95636dfd3cc29ad)] (cargo-release) start next development iteration 0.6.1-alpha.0 (Yoshua Wuyts)

### Stats
```diff
 .travis.yml         |  1 +-
 Cargo.toml          | 14 ++++++------
 src/bitfield/mod.rs |  9 +++-----
 src/feed.rs         | 49 +++++++++++++++++++++++++++++--------------
 src/feed_builder.rs |  3 ++-
 src/lib.rs          |  2 +-
 src/storage/mod.rs  | 62 +++++++++++++++++++++++++++++++++++++++++++++++++-----
 tests/compat.rs     |  7 +++---
 tests/feed.rs       | 32 ++++++++++++++++++++++++++++-
 tests/helpers.rs    |  2 +-
 tests/storage.rs    | 54 +++++++++++++++++++++++++++++++++++++++++++++++-
 11 files changed, 197 insertions(+), 38 deletions(-)
```
