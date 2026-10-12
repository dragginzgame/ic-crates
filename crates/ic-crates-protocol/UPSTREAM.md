# crates.io source attribution

Selected source: [rust-lang/crates.io at
8ac99b2b31af2efd2ab53035c8636f3c1895654c](https://github.com/rust-lang/crates.io/tree/8ac99b2b31af2efd2ab53035c8636f3c1895654c).
The adapted code uses the upstream MIT license, reproduced in
[LICENSE-CRATES-IO](LICENSE-CRATES-IO). Our original code remains covered by
[LICENSE](LICENSE).

| Upstream source | Local adaptation |
| --- | --- |
| [crates_io_index/src/features.rs](https://github.com/rust-lang/crates.io/blob/8ac99b2b31af2efd2ab53035c8636f3c1895654c/crates/crates_io_index/src/features.rs) | [Cargo feature splitting](src/cargo/features/mod.rs): direct extended syntax and transitive aliases move into `features2`. A reverse traversal replaces repeated scanning and the fixed iteration cutoff, preserving cycles and deep chains. |
| [crates_io_validation/src/lib.rs](https://github.com/rust-lang/crates.io/blob/8ac99b2b31af2efd2ab53035c8636f3c1895654c/crates/crates_io_validation/src/lib.rs) | [Dependency alias validation](src/index/mod.rs): permit an initial underscore for aliases, retain bounded ASCII identifiers and map failures to the existing passive registry error. Actual crate/package names keep their separate rules. |

No upstream database, web framework, worker or runtime was copied. The existing
public DTOs, persisted records and IC lifecycle/storage owners remain the local
integration boundaries. Protocol regressions cover transitive aliases, cycles,
long chains and alias/name distinctions; the native bridge's unmodified Cargo
fixture publishes two crates and resolves a renamed optional dependency through
default, namespaced and weak feature activation.
