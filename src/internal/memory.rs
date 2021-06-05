// SPDX-License-Identifier: MPL-2.0

//! A Memory acts like a structured partial solution
//! where terms are regrouped by package in a [Map](crate::type_aliases::Map).

use crate::internal::arena::Arena;
use crate::internal::assignment::Assignment::{self, Decision, Derivation};
use crate::internal::incompatibility::Incompatibility;
use crate::package::Package;
use crate::range::Range;
use crate::term::Term;
use crate::type_aliases::{Map, SelectedDependencies};
use crate::version::Version;

/// A memory is the set of all assignments in the partial solution,
/// organized by package instead of historically ordered.
///
/// Contrary to PartialSolution, Memory does not store derivations causes, only the terms.
#[derive(Clone)]
pub struct Memory<P: Package, V: Version> {
    assignments: Map<P, PackageAssignments<V>>,
}

/// A package memory contains the potential decision and derivations
/// that have already been made for a given package.
#[derive(Clone)]
enum PackageAssignments<V: Version> {
    Decision((V, Term<V>)),
    Derivations(Term<V>),
}

impl<P: Package, V: Version> Memory<P, V> {
    /// Initialize an empty memory.
    pub fn empty() -> Self {
        Self {
            assignments: Map::default(),
        }
    }

    /// Clears the memory.
    pub fn clear(&mut self) {
        self.assignments.clear();
    }

    /// Retrieve intersection of terms in memory related to package.
    pub fn term_intersection_for_package(&self, package: &P) -> Option<&Term<V>> {
        self.assignments
            .get(package)
            .map(|pa| pa.assignment_intersection())
    }

    /// Building step of a Memory from a given assignment.
    pub fn add_assignment(
        &mut self,
        assignment: &Assignment<P, V>,
        store: &Arena<Incompatibility<P, V>>,
    ) {
        match assignment {
            Decision { package, version } => self.add_decision(package.clone(), version.clone()),
            Derivation { package, cause } => self.add_derivation(
                package.clone(),
                store[*cause].get(&package).unwrap().negate(),
            ),
        }
    }

    /// Add a decision to a Memory.
    pub fn add_decision(&mut self, package: P, version: V) {
        // Check that add_decision is never used in the wrong context.
        if cfg!(debug_assertions) {
            match self.assignments.get_mut(&package) {
                None => panic!("Assignments must already exist"),
                // Cannot be called when a decision has already been taken.
                Some(PackageAssignments::Decision(_)) => panic!("Already existing decision"),
                // Cannot be called if the versions is not contained in the terms intersection.
                Some(pa) => debug_assert!(pa.assignment_intersection().contains(&version)),
            }
        }

        self.assignments.insert(
            package,
            PackageAssignments::Decision((version.clone(), Term::exact(version))),
        );
    }

    /// Add a derivation to a Memory.
    pub fn add_derivation(&mut self, package: P, term: Term<V>) {
        use std::collections::hash_map::Entry;
        match self.assignments.entry(package) {
            Entry::Occupied(mut o) => match o.get_mut() {
                // Check that add_derivation is never called in the wrong context.
                PackageAssignments::Decision(_) => debug_assert!(false),
                PackageAssignments::Derivations(t) => {
                    *t = t.intersection(&term);
                }
            },
            Entry::Vacant(v) => {
                v.insert(PackageAssignments::Derivations(term));
            }
        }
    }

    /// Extract all packages that may potentially be picked next
    /// to continue solving package dependencies.
    /// A package is a potential pick if there isn't an already
    /// selected version (no "decision")
    /// and if it contains at least one positive derivation term
    /// in the partial solution.
    pub fn potential_packages(&self) -> impl Iterator<Item = (&P, &Range<V>)> {
        // let count = self
        //     .assignments
        //     .iter()
        //     .filter_map(|(p, pa)| pa.potential_package_filter(p))
        //     .count();
        // println!("Potential packages count: {}", count);
        self.assignments
            .iter()
            .filter_map(|(p, pa)| pa.potential_package_filter(p))
    }

    /// If a partial solution has, for every positive derivation,
    /// a corresponding decision that satisfies that assignment,
    /// it's a total solution and version solving has succeeded.
    pub fn extract_solution(&self) -> Option<SelectedDependencies<P, V>> {
        let mut solution = Map::default();
        for (p, pa) in &self.assignments {
            match pa {
                PackageAssignments::Decision((v, _)) => {
                    solution.insert(p.clone(), v.clone());
                }
                PackageAssignments::Derivations(intersected) => {
                    if intersected.is_positive() {
                        return None;
                    }
                }
            }
        }
        Some(solution)
    }
}

impl<V: Version> PackageAssignments<V> {
    /// Returns intersection of all assignments (decision included).
    fn assignment_intersection(&self) -> &Term<V> {
        match self {
            PackageAssignments::Decision((_, term)) => term,
            PackageAssignments::Derivations(term) => term,
        }
    }

    /// A package is a potential pick if there isn't an already
    /// selected version (no "decision")
    /// and if it contains at least one positive derivation term
    /// in the partial solution.
    fn potential_package_filter<'a, P: Package>(
        &'a self,
        package: &'a P,
    ) -> Option<(&'a P, &'a Range<V>)> {
        match self {
            PackageAssignments::Decision(_) => None,
            PackageAssignments::Derivations(intersected) => {
                if intersected.is_positive() {
                    Some((package, self.assignment_intersection().unwrap_positive()))
                } else {
                    None
                }
            }
        }
    }
}
