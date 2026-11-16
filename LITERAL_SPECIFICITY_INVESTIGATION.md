# Literal specificity investigation

## Context

[Pyright PR #11967](https://github.com/microsoft/pyright/pull/11967) compared
Pyright 1.1.414 with Pyrefly 1.4.0 using 10,000 generated cases from 32
templates plus 13 behavioral cases. Its reported generated-case results were:

| Result | Cases |
| --- | ---: |
| Equivalent inferred types | 8,125 |
| Pyright inferred a narrower type | 1,575 |
| Pyrefly inferred a narrower type | 0 |
| Concrete display differences | 313 |

These numbers describe those versions and that corpus, not a permanent quality
ordering. The benchmark should be rerun against current Pyright and Pyrefly
before implementation decisions or progress claims are based on the totals.
Pyrefly issue [#3977](https://github.com/facebook/pyrefly/issues/3977) tracks the
related specificity gap.

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
([`overload.rs`](pyrefly/lib/alt/overload.rs#L904-L916)). Retaining literals
earlier can select a different overload, suppress or introduce an ambiguity,
and change the diagnostic presented to the user. Existing tests already record
the intentional spec/compatibility distinction
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

### 1. Preserve bounded ambiguous-overload results

Start with the clearest capability loss: when multiple overloads remain viable
and their returns differ, form a bounded union or another explicit alternative
result rather than immediately returning implicit `Any`. Reuse the existing
union simplification and caps, and retain the current fallback when the
alternative set cannot be represented cheaply. Add cases for ambiguous,
defaulted, and disjoint arguments, generic receiver overloads, generic
constructors, and higher-order callback results.

This stage should explicitly compare the compatibility mode and the
spec-compliant overload mode. It should not assume that Pyright's selected
overload is always preferable; the goal is to preserve common information when
selection is genuinely ambiguous.

### 2. Add context-sensitive literal preservation

Preserve implicit literals only where the consumer is known to need them:

- `TypedDict` keys passing through generic inference;
- literal-sensitive overload arguments;
- discriminators used for narrowing; and
- `LiteralString` propagation.

Prefer carrying semantic evidence or a contextual expectation to delaying
promotion everywhere. For example, current tests cover constrained generic
`TypedDict` indexing
([`typed_dict.rs`](pyrefly/lib/test/typed_dict.rs#L2812-L2879)), literal-based
overload selection
([`overload.rs`](pyrefly/lib/test/overload.rs#L1971-L2012)), and a known
`LiteralString`/partial-inference interaction
([`literal.rs`](pyrefly/lib/test/literal.rs#L482-L493)). Extend these focused
areas before changing general expression inference.

### 3. Model variance and mutability explicitly

Allow greater retention in immutable or covariant results while keeping
promotion at mutable or invariant boundaries. Tests should cover later mutation
and assignment compatibility, not only `reveal_type`, so a narrower display
does not hide a usability regression.

### 4. Keep recursive alias rendering separate

Treat recursive-alias spelling, expansion depth, and hover/reveal output as a
display project unless assignability changes. The recursive alias suite
([`recursive_alias.rs`](pyrefly/lib/test/recursive_alias.rs)) provides the
behavioral baseline; display work should not be used to inflate inference
specificity metrics.

### 5. Expand behavioral and performance coverage

Recreate the 13 behavioral cases as focused regression tests, retain generated
templates for breadth, and rerun the 10,000-case comparison on current releases.
For every inference change, record:

- checking and diagnostic behavior, not only displayed types;
- union width and serialized/displayed type size;
- benchmark wall time and memory; and
- whether the result follows the typing specification, an intentional
  compatibility policy, or an unresolved specification question.

The implementation should proceed only while those measurements remain bounded
and mutable/invariant compatibility is preserved.
