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
