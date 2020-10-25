// SPDX-License-Identifier: MPL-2.0

//! The partial solution is the current state
//! of the solution being built by the algorithm.

use crate::internal::memory::Memory;
use crate::package::Package;
use crate::term::Term;
use crate::type_aliases::{Map, SelectedDependencies};
use crate::version::Version;
use crate::{
    error::PubGrubError,
    internal::incompatibility::{Incompatibility, Relation},
};
use crate::{
    internal::assignment::Assignment::{self, Decision, Derivation},
    solver::DependencyProvider,
};

/// The partial solution is the current state
/// of the solution being built by the algorithm.
/// It is composed of a succession of assignments,
/// defined as either decisions or derivations.
#[derive(Clone)]
pub struct PartialSolution<P: Package, V: Version> {
    decision_level: usize,
    /// Each assignment is stored with its decision level in the history.
    /// The order in which assignments where added in the vec is kept,
    /// so the oldest assignments are at the beginning of the vec.
    history: Vec<(usize, Assignment<P, V>)>,
    memory: Memory<P, V>,
}

impl<P: Package, V: Version> PartialSolution<P, V> {
    /// Initialize an empty partial solution.
    pub fn empty() -> Self {
        Self {
            decision_level: 0,
            history: Vec::new(),
            memory: Memory::empty(),
        }
    }

    fn add_assignment(&mut self, assignment: Assignment<P, V>) {
        self.decision_level = match assignment {
            Decision { .. } => self.decision_level + 1,
            Derivation { .. } => self.decision_level,
        };
        self.memory.add_assignment(&assignment);
        self.history.push((self.decision_level, assignment));
    }

    /// Add a decision to the partial solution.
    pub fn add_decision(&mut self, package: P, version: V) {
        self.add_assignment(Decision { package, version });
    }

    /// Add a derivation to the partial solution.
    pub fn add_derivation(&mut self, package: P, term: Term<V>, cause: Incompatibility<P, V>) {
        self.add_assignment(Derivation {
            package,
            term,
            cause,
        });
    }

    /// If a partial solution has, for every positive derivation,
    /// a corresponding decision that satisfies that assignment,
    /// it's a total solution and version solving has succeeded.
    pub fn extract_solution(&self) -> Option<SelectedDependencies<P, V>> {
        self.memory.extract_solution()
    }

    /// Backtrack the partial solution to a given decision level.
    pub fn backtrack(&mut self, decision_level: usize) {
        // TODO: improve with dichotomic search.
        let pos = self
            .history
            .iter()
            .rposition(|(l, _)| *l == decision_level)
            .unwrap_or(self.history.len() - 1);
        *self = Self::from_assignments(
            std::mem::take(&mut self.history)
                .into_iter()
                .take(pos + 1)
                .map(|(_, a)| a),
        );
    }

    fn from_assignments(assignments: impl Iterator<Item = Assignment<P, V>>) -> Self {
        let mut partial_solution = Self::empty();
        assignments.for_each(|a| partial_solution.add_assignment(a));
        partial_solution
    }

    /// Heuristic to pick the next package to add to the partial solution.
    /// This should be a package with a positive derivation but no decision yet.
    /// If multiple choices are possible, use a heuristic.
    ///
    /// Current heuristic employed by this and Pub's implementations is to choose
    /// the package with the fewest versions matching the outstanding constraint.
    /// This tends to find conflicts earlier if any exist,
    /// since these packages will run out of versions to try more quickly.
    pub fn pick_package(
        &mut self,
        dependency_provider: &impl DependencyProvider<P, V>,
    ) -> Result<Option<(P, Term<V>)>, PubGrubError<P, V>> {
        let mut out: Option<(P, Term<V>)> = None;
        let mut min_key = usize::MAX;
        for (p, term) in self.memory.potential_packages() {
            let key = dependency_provider
                .list_available_versions(p)
                .map_err(|err| PubGrubError::ErrorRetrievingVersions {
                    package: p.clone(),
                    source: err,
                })?
                .iter()
                .filter(|&v| term.contains(v))
                .count();
            if key < min_key {
                min_key = key;
                out = Some((p.clone(), term.clone()));
            }
        }
        Ok(out)
    }

    /// Pub chooses the latest matching version of the package
    /// that match the outstanding constraint.
    ///
    /// Here we just pick the first one that satisfies the terms.
    /// It is the responsibility of the provider of `available_versions`
    /// to list them with preferred versions first.
    pub fn pick_version(available_versions: &[V], partial_solution_term: &Term<V>) -> Option<V> {
        available_versions
            .iter()
            .find(|v| partial_solution_term.contains(v))
            .cloned()
    }

    /// We can add the version to the partial solution as a decision
    /// if it doesn't produce any conflict with the new incompatibilities.
    /// In practice I think it can only produce a conflict if one of the dependencies
    /// (which are used to make the new incompatibilities)
    /// is already in the partial solution with an incompatible version.
    pub fn add_version(
        &mut self,
        package: P,
        version: V,
        new_incompatibilities: &[Incompatibility<P, V>],
    ) {
        self.add_decision(package, version);
        if self.satisfies_any_of(new_incompatibilities) {
            self.remove_last_decision();
        }
    }

    /// Can ONLY be called if the last assignment added was a decision.
    fn remove_last_decision(&mut self) {
        self.decision_level -= 1;
        let (_, last_assignment) = self.history.pop().unwrap();
        self.memory.remove_decision(last_assignment.package());
    }

    fn satisfies_any_of(&mut self, incompatibilities: &[Incompatibility<P, V>]) -> bool {
        incompatibilities
            .iter()
            .any(|incompat| self.relation(incompat) == Relation::Satisfied)
    }

    /// Check if the terms in the partial solution satisfy the incompatibility.
    pub fn relation(&mut self, incompat: &Incompatibility<P, V>) -> Relation<P, V> {
        incompat.relation(|package| self.memory.term_intersection_for_package(package))
    }

    /// Find satisfier and previous satisfier decision level.
    pub fn find_satisfier_and_previous_satisfier_level(
        &self,
        incompat: &Incompatibility<P, V>,
    ) -> (&Assignment<P, V>, usize, usize) {
        let ((satisfier_level, satisfier), previous_assignments) =
            Self::find_satisfier(incompat, self.history.as_slice())
                .expect("We should always find a satisfier if called in the right context.");
        let previous_satisfier_level =
            Self::find_previous_satisfier(incompat, satisfier, previous_assignments);
        (satisfier, satisfier_level, previous_satisfier_level)
    }

    /// A satisfier is the earliest assignment in partial solution such that the incompatibility
    /// is satisfied by the partial solution up to and including that assignment.
    /// Also returns all assignments earlier than the satisfier.
    fn find_satisfier<'a>(
        incompat: &Incompatibility<P, V>,
        history: &'a [(usize, Assignment<P, V>)],
    ) -> Option<(
        (usize, &'a Assignment<P, V>),
        &'a [(usize, Assignment<P, V>)],
    )> {
        Self::find_satisfier_helper(incompat, Self::new_accum_satisfied_from(incompat), history)
    }

    /// Earliest assignment in the partial solution before satisfier
    /// such that incompatibility is satisfied by the partial solution up to
    /// and including that assignment plus satisfier.
    fn find_previous_satisfier<'a>(
        incompat: &Incompatibility<P, V>,
        satisfier: &Assignment<P, V>,
        previous_assignments: &'a [(usize, Assignment<P, V>)],
    ) -> usize {
        let package = satisfier.package().clone();
        let incompat_term = incompat.get(&package).expect("This should exist");
        let satisfier_term = satisfier.as_term();
        let is_satisfied = satisfier_term.subset_of(incompat_term);
        let mut accum_satisfied = Self::new_accum_satisfied_from(incompat);
        accum_satisfied.insert(package, (is_satisfied, satisfier_term));
        // Search previous satisfier.
        Self::find_satisfier_helper(incompat, accum_satisfied, previous_assignments)
            .map_or(1, |((level, _), _)| level.max(1))
    }

    fn new_accum_satisfied_from(incompat: &Incompatibility<P, V>) -> Map<P, (bool, Term<V>)> {
        incompat
            .iter()
            .map(|(p, _)| (p.clone(), (false, Term::any())))
            .collect()
    }

    /// Iterate over the assignments (oldest must be first)
    /// until we find the first one such that the set of all assignments until this one (included)
    /// satisfies the given incompatibility.
    pub fn find_satisfier_helper<'a>(
        incompat: &Incompatibility<P, V>,
        accum_satisfied: Map<P, (bool, Term<V>)>,
        all_assignments: &'a [(usize, Assignment<P, V>)],
    ) -> Option<(
        (usize, &'a Assignment<P, V>),
        &'a [(usize, Assignment<P, V>)],
    )> {
        let mut accum_satisfied = accum_satisfied;
        for (idx, (level, assignment)) in all_assignments.iter().enumerate() {
            let package = assignment.package();
            let incompat_term = match incompat.get(package) {
                // We only care about packages related to the incompatibility.
                None => continue,
                Some(i) => i,
            };
            let (is_satisfied, accum_term) = match accum_satisfied.get_mut(package) {
                None => panic!("A package in incompat should always exist in accum_satisfied"),
                Some((true, _)) => continue, // If that package term is already satisfied, no need to check.
                Some(x) => x,
            };
            // Check if that incompat term is satisfied by our accumulated terms intersection.
            *accum_term = accum_term.intersection(&assignment.as_term());
            *is_satisfied = accum_term.subset_of(incompat_term);
            // Check if we have found the satisfier
            // (all booleans in accum_satisfied are true).
            if *is_satisfied && accum_satisfied.iter().all(|(_, (satisfied, _))| *satisfied) {
                return Some(((*level, assignment), &all_assignments[0..idx]));
            }
        }
        None
    }
}
