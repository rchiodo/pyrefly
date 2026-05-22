/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use pyrefly_derive::TypeEq;
use pyrefly_derive::Visit;
use pyrefly_derive::VisitMut;

use crate::quantified::Quantified;
use crate::types::Type;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Visit, VisitMut, TypeEq)]
pub enum CallableResidualKind {
    /// A generic residual. The `quantified` is the quantified type variable we
    /// want to use when reconstructing a `Callable` that contains a generic
    /// residual; we'll wrap it in a Forall that scopes all the residuals.
    ///
    /// If it appears anywhere else, the fallback is `quantified.as_gradual_type()`
    Generic { quantified: Quantified },
    /// Per-var overload residual with identity for cross-var correlation.
    ///
    /// Finishing normalizes branch types so an overload residual does not
    /// contain nested overload residual markers in `branches[*].ty`.
    Overload {
        identity: OverloadResidualIdentity,
        branches: Vec<OverloadBranchProjection>,
    },
}

/// Correlation key for matching overload residuals across vars during finalization.
/// The hash is derived from the got-side type at the comparison that produced the
/// residual, making it a stable function of the value rather than of solve order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Visit, VisitMut, TypeEq)]
pub struct OverloadResidualIdentity {
    pub witness_hash: u64,
}

/// Per-branch result for a single var in an overload residual.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Visit, VisitMut, TypeEq)]
pub struct OverloadBranchProjection {
    pub branch_index: usize,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Visit, VisitMut, TypeEq)]
pub struct CallableResidual {
    pub kind: CallableResidualKind,
}
