//! Cargo feature partitioning adapted from rust-lang/crates.io.
//!
//! Source: crates/crates_io_index/src/features.rs at
//! 8ac99b2b31af2efd2ab53035c8636f3c1895654c.
//! Copyright (c) 2014 The Rust Project Developers.
//! Used under MIT; see LICENSE-CRATES-IO and UPSTREAM.md in this package.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) fn split_features(
    features: BTreeMap<String, Vec<String>>,
) -> (BTreeMap<String, Vec<String>>, BTreeMap<String, Vec<String>>) {
    let mut dependents = BTreeMap::<&str, Vec<&str>>::new();
    let mut extended = BTreeSet::new();
    let mut pending = VecDeque::new();
    for (name, values) in &features {
        if values.iter().any(|value| has_features2_syntax(value)) {
            extended.insert(name.clone());
            pending.push_back(name.as_str());
        }
        for value in values {
            if features.contains_key(value) {
                dependents.entry(value).or_default().push(name);
            }
        }
    }
    // Upstream propagates the partition by repeated scans. Reverse edges let
    // us visit each affected feature once, including cycles and deep aliases,
    // without its fixed iteration cap or a partially split index.
    while let Some(name) = pending.pop_front() {
        if let Some(aliases) = dependents.get(name) {
            for &alias in aliases {
                if extended.insert(alias.to_owned()) {
                    pending.push_back(alias);
                }
            }
        }
    }
    features
        .into_iter()
        .partition(|(name, _)| !extended.contains(name))
}

fn has_features2_syntax(value: &str) -> bool {
    value.starts_with("dep:") || value.contains("?/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_cycles_remain_legacy_and_extended_chains_have_no_iteration_cutoff() {
        let legacy = BTreeMap::from([
            ("a".into(), vec!["b".into()]),
            ("b".into(), vec!["a".into()]),
        ]);
        let (features, extended) = split_features(legacy.clone());
        assert_eq!(features, legacy);
        assert!(extended.is_empty());

        let mut chain = BTreeMap::new();
        chain.insert("f150".into(), vec!["dep:codec".into()]);
        for index in 0..150 {
            chain.insert(format!("f{index:03}"), vec![format!("f{:03}", index + 1)]);
        }
        let (features, extended) = split_features(chain.clone());
        assert!(features.is_empty());
        assert_eq!(extended, chain);
    }
}
