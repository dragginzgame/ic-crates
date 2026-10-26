# IC Crates Protocol

Cargo sparse-index formats and publication contracts for
[IC Crates](https://github.com/dragginzgame/ic-crates), a registry foundation
on the Internet Computer.

This library validates bounded Cargo index entries, computes Cargo's sparse
index paths, normalizes crate namespace keys and carries Candid configuration
and publication requests. `CargoMetadata::into_index` translates publish API
dependencies and extended features. Shared HTTP DTOs carry the gateway Candid
contract. It exports no endpoints, storage or upload runtime.

```rust
use ic_crates_protocol::{canonical_name, index_path};

assert_eq!(index_path("serde_json")?, "se/rd/serde_json");
assert_eq!(canonical_name("Example_Crate")?, "example-crate");
# Ok::<(), ic_crates_protocol::RegistryError>(())
```

`IndexEntry` uses Cargo's index dependency fields. A null dependency registry
means the current registry; crates.io dependencies need their explicit index URL.
This is distinct from Cargo publish-API metadata.

Dependency aliases may start with `_`; crate and package names must start with an
ASCII letter. Extended features and every transitive alias referring to them
are placed in `features2`, including aliases used by `default`. This behavior is
adapted from crates.io; [source revisions and notices](UPSTREAM.md) are included
in the package.

Cargo's checksum is lowercase hexadecimal SHA-256 of raw `.crate` bytes, not the
provider tree root. Configuration names an HTTPS provider origin, blob service,
namespace, project and completion verifier. Parsing validates representation;
the application authenticates delivery and verifies content before publication.

Current dependency adoption is pending compatible Canic/IC Wire integration;
see the [qualification status](https://github.com/dragginzgame/ic-crates/blob/main/docs/status/current.md).

Early pre-1.0 API. Minimum required Rust: 1.91.0 (required by Blob contracts 0.47). License: MIT.
