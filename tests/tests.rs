// SPDX-License-Identifier: MPL-2.0

use pubgrub::error::PubGrubError;
use pubgrub::range_trait::Range;
use pubgrub::solver::{resolve, OfflineDependencyProvider};
use pubgrub::version_trait::{NumberInterval, NumberVersion};

#[test]
fn same_result_on_repeated_runs() {
    let mut dependency_provider =
        OfflineDependencyProvider::<_, NumberInterval, NumberVersion>::new();

    dependency_provider.add_dependencies("c", 0, vec![]);
    dependency_provider.add_dependencies("c", 2, vec![]);
    dependency_provider.add_dependencies("b", 0, vec![]);
    dependency_provider.add_dependencies("b", 1, vec![("c", Range::between(0, 1))]);

    dependency_provider.add_dependencies("a", 0, vec![("b", Range::full()), ("c", Range::full())]);

    let name = "a";
    let ver = NumberVersion(0);
    let one = resolve(&dependency_provider, name, ver);
    for _ in 0..10 {
        match (&one, &resolve(&dependency_provider, name, ver)) {
            (Ok(l), Ok(r)) => assert_eq!(l, r),
            _ => panic!("not the same result"),
        }
    }
}

#[test]
fn should_always_find_a_satisfier() {
    let mut dependency_provider =
        OfflineDependencyProvider::<_, NumberInterval, NumberVersion>::new();
    dependency_provider.add_dependencies("a", 0, vec![("b", Range::empty())]);
    assert!(matches!(
        resolve(&dependency_provider, "a", 0),
        Err(PubGrubError::DependencyOnTheEmptySet { .. })
    ));

    dependency_provider.add_dependencies("c", 0, vec![("a", Range::full())]);
    assert!(matches!(
        resolve(&dependency_provider, "c", 0),
        Err(PubGrubError::DependencyOnTheEmptySet { .. })
    ));
}

#[test]
fn cannot_depend_on_self() {
    let mut dependency_provider =
        OfflineDependencyProvider::<_, NumberInterval, NumberVersion>::new();
    dependency_provider.add_dependencies("a", 0, vec![("a", Range::full())]);
    assert!(matches!(
        resolve(&dependency_provider, "a", 0),
        Err(PubGrubError::SelfDependency { .. })
    ));
}
