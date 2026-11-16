# Literal specificity investigation

## Context

[Pyright PR #11967](https://github.com/microsoft/pyright/pull/11967) compared
Pyright 1.1.414 with Pyrefly 1.4.0 using 10,000 generated precision cases from
32 templates plus 13 behavioral cases. Across those 10,013 cases, its reported
reveal-type classifications were:

| Result | Cases |
| --- | ---: |
| Equivalent inferred types | 8,125 |
| Pyright inferred a narrower type | 1,575 |
| Pyrefly inferred a narrower type | 0 |
| Concrete display differences | 313 |

The behavioral probes also found that Pyright diagnosed 5/5 expected invalid
operations compared with Pyrefly's 1/5, and accepted 15/15 narrow-type
capability probes compared with Pyrefly's 7/15.

These numbers describe those versions and that corpus, not a permanent quality
ordering. The benchmark should be rerun against current Pyright and Pyrefly
before implementation decisions or progress claims are based on the totals.
Pyrefly issue [#3977](https://github.com/pyrefly-org/pyrefly/issues/3977)
tracked one related overload-materialization problem, but it has since been
closed after a targeted non-spec-compliant overload fix. That resolution is
evidence that the corpus must be reclassified before remaining differences are
assigned to literal retention or ambiguous-result handling.

Pyright's result is not automatically the correct result in every differing
case. Some differences are presentation choices, some reflect deliberate
inference policy, and some involve underspecified or compatibility-sensitive
behavior. The useful question is where retaining information improves checking
without breaking sound mutable APIs, destabilizing diagnostics, or causing
unbounded type growth.

## What the differences represent

The observed cases fall into these broad categories:

- literal retention through generic calls;
- flow merges after narrowing away `None`;
- flows combining `LiteralString` and concrete string literals;
- generic class construction;
- higher-order callback result inference;
- rendering of recursive aliases;
- ambiguous, defaulted, or disjoint overload results;
- preservation of `TypedDict` keys through generic inference;
- literal-sensitive overload selection; and
- overloads on generic receivers.

They should not be treated as one bug class:

1. **Display-only differences** change spelling or recursive-alias expansion
   without changing assignability. Recursive alias formatting belongs in this
   category until a behavioral difference is demonstrated, and should be
   addressed separately from inference.
2. **Deliberate inference-policy widening** includes promotion needed to avoid
   exposing incidental constants as durable types of mutable values.
3. **Specification-required behavior once literal information is known**
   includes using a retained literal key, discriminator, or argument to check an
   operation whose semantics are explicitly literal-sensitive.
4. **True capability loss** occurs when several viable overload alternatives
   collapse to implicit `Any`/`Unknown`, discarding useful bounds that all
   alternatives could have preserved.

## Pyrefly constraints

Pyrefly intentionally distinguishes an implicit literal from a literal required
by an annotation, and provides shallow and recursive promotion operations
([`types.rs`](crates/pyrefly_types/src/types.rs#L1133-L1138),
[`types.rs`](crates/pyrefly_types/src/types.rs#L1763-L1794)). Promotion is used
when inferring container elements
([`expr.rs`](pyrefly/lib/alt/expr.rs#L682-L739)), inferred writable class
attributes
([`class_field.rs`](pyrefly/lib/alt/class/class_field.rs#L1899-L1915)), and
forwarded/solved bindings
([`solve.rs`](pyrefly/lib/alt/solve.rs#L4968-L4974)). This is not merely lost
precision: widening prevents a first incidental value from becoming the
permanent public type of a writable attribute, container element, or solved
type variable.

Mutable generic classes are normally invariant. Preserving `Literal["x"]`
inside `list[...]`, `dict[...]`, or another mutable generic can therefore make
the inferred value incompatible with later writes or with an expected
`list[str]`/`dict[str, ...]`. A specificity change must distinguish immutable or
covariant positions from mutable or invariant positions rather than globally
disabling promotion.

Specificity also affects overload behavior and diagnostics. Pyrefly currently
uses contextual typing only after selecting an overload and has both
spec-compliant and ecosystem-compatibility disambiguation paths
([`overload.rs`](pyrefly/lib/alt/overload.rs#L860-L934),
[`overload.rs`](pyrefly/lib/alt/overload.rs#L936-L972)). When viable overload
returns cannot be disambiguated, the current result is implicit `Any`
([`overload.rs`](pyrefly/lib/alt/overload.rs#L904-L916)). The current typing
specification permits `Any` or a more precise *gradual* result that is assignable
to every remaining return type and permits operations supported by any of them.
An ordinary union does not meet that requirement because it permits only
operations common to every member. Retaining literals earlier can select a
different overload, suppress or introduce an ambiguity, and change the
diagnostic presented to the user. Existing tests already record the intentional
spec/compatibility distinction
([`overload.rs`](pyrefly/lib/test/overload.rs#L1971-L2012)).

Finally, literal retention has solver, representation, and performance costs.
Current source widens more than 256 literals of one scalar kind and caps a
general union at 4,096 members
([`simplify.rs`](crates/pyrefly_types/src/simplify.rs#L87-L111),
[`simplify.rs`](crates/pyrefly_types/src/simplify.rs#L173-L177),
[`simplify.rs`](crates/pyrefly_types/src/simplify.rs#L252-L280)). These are
examples of existing complexity controls, not proposed budgets for this work.
Any change that carries more alternatives through generic solving or overload
resolution must measure type-size growth, allocation, and checking time.

## Recommended sequence

### 0. Rerun and reclassify the benchmark

Run the comparison against current Pyright and Pyrefly before selecting an
implementation target. Preserve the generated inputs and classify every
remaining difference by root cause rather than by surface feature:

- overload candidate elimination or materialization;
- genuinely ambiguous overload returns;
- literal promotion across a binding or generic solve;
- contextual typing or partial inference;
- flow narrowing;
- display-only differences; and
- intentional policy differences.

Record the behavioral probe results alongside reveal-type counts. A narrower
display should not be counted as progress unless it improves checking or an
explicitly chosen inference policy.

### 1. Improve overload elimination before representing ambiguity

Fix cases where Pyrefly can select an overload by using information it already
has, including materialization, argument expansion, generic receiver
specialization, and solved return variables. Issue #3977 is an example of a
case that appeared to require a better ambiguous result but was instead fixed
by eliminating an inapplicable fallback overload.

Add focused cases for defaulted and disjoint arguments, generic receivers,
generic constructors, and higher-order callback results. Keep the
spec-compliant and ecosystem-compatibility expectations explicit in each test.

### 2. Represent genuinely ambiguous overload results

When multiple overloads genuinely remain viable and their returns differ,
preserve more information only with a representation whose behavior satisfies
the typing specification's gradual requirements. An ordinary union is not a
valid replacement for `Any`: for `int` and `str` alternatives, the result must
still permit both integer-only and string-only operations.

Define a small local complexity budget for this representation rather than
relying on the general 4,096-member union cap. Fall back to implicit `Any` when
the alternatives cannot be represented cheaply. This stage should explicitly
compare compatibility mode and spec-compliant mode; it should not assume that
Pyright's selected overload is always preferable.

### 3. Add context-sensitive literal preservation

Preserve implicit literals only where the consumer is known to need them:

- `TypedDict` keys passing through generic inference;
- literal-sensitive overload arguments;
- discriminators used for narrowing; and
- `LiteralString` propagation.

Prefer carrying semantic evidence or a contextual expectation to delaying
promotion everywhere. Before implementation, choose where that evidence lives:
for example, a promoted type paired with retained literal provenance, or an
unpromoted alternative carried through a specific generic solve. This must not
depend on contextual typing that is available only after overload selection.

Current tests cover constrained generic `TypedDict` indexing
([`typed_dict.rs`](pyrefly/lib/test/typed_dict.rs#L2812-L2879)), literal-based
overload selection
([`overload.rs`](pyrefly/lib/test/overload.rs#L1971-L2012)), and a known
`LiteralString`/partial-inference interaction
([`literal.rs`](pyrefly/lib/test/literal.rs#L482-L493)). Extend these focused
areas before changing general expression inference.

### 4. Refine promotion boundaries

Allow greater retention at proven-safe boundaries while keeping promotion where
values become writable or form stable public types. Declared variance alone is
not a reliable proxy for runtime mutability: an invariant user-defined class
may be immutable, and a covariant protocol may expose behavior that requires
careful checking. Start with concrete promotion sites and consumers rather than
introducing a global mutability model.

Tests must cover later mutation and assignment compatibility, not only
`reveal_type`, so a narrower display does not hide a usability regression.

### 5. Keep recursive alias rendering separate

Treat recursive-alias spelling, expansion depth, and hover/reveal output as a
display project unless assignability changes. The recursive alias suite
([`recursive_alias.rs`](pyrefly/lib/test/recursive_alias.rs)) provides the
behavioral baseline; display work should not be used to inflate inference
specificity metrics.

### 6. Expand behavioral and performance coverage

Recreate the 13 behavioral cases as focused regression tests, retain generated
templates for breadth, and rerun the 10,000-case comparison on current releases.
For every inference change, record:

- checking and diagnostic behavior, not only displayed types;
- union width and serialized/displayed type size;
- benchmark wall time and memory; and
- whether the result follows the typing specification, an intentional
  compatibility policy, or an unresolved specification question.

Before each implementation stage, define its local type-size or candidate-count
budget and acceptable performance regression. The implementation should
proceed only while those budgets remain bounded, no new diagnostic regressions
appear, and writable-value compatibility is preserved.
