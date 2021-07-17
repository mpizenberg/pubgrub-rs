// SPDX-License-Identifier: MPL-2.0

use pubgrub::range_trait::Range;
use pubgrub::solver::{resolve, OfflineDependencyProvider};
use pubgrub::version_trait::{NumberInterval, NumberVersion};

// `root` depends on `menu` and `icons`
// `menu` depends on `dropdown`
// `dropdown` depends on `icons`
// `icons` has no dependency
#[rustfmt::skip]
fn main() {
    let mut dependency_provider = OfflineDependencyProvider::<&str, NumberInterval,  NumberVersion>::new();
    dependency_provider.add_dependencies(
        "root", 1, vec![("menu", Range::full()), ("icons", Range::full())],
    );
    dependency_provider.add_dependencies("menu", 1, vec![("dropdown", Range::full())]);
    dependency_provider.add_dependencies("dropdown", 1, vec![("icons", Range::full())]);
    dependency_provider.add_dependencies("icons", 1, vec![]);

    // Run the algorithm.
    let solution = resolve(&dependency_provider, "root", 1);
    println!("Solution: {:?}", solution);
}
