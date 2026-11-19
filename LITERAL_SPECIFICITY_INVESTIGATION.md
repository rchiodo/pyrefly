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

## Adjudication of the remaining benchmark differences

After the overload and literal-retention changes, the remaining differences
fall into three groups. None is evidence that Pyrefly should copy Pyright's
displayed type unconditionally.

1. **Mutable generic construction.** Pyright infers
   `Box[tuple[Literal["tag"], Literal[15]]]` for a mutable dataclass and then
   rejects `box.value = ("other", 16)`. Pyrefly infers
   `Box[tuple[str, int]]` and accepts the write. The wider result is the intended
   policy for invariant, writable generic storage. Literal preservation remains
   worth considering for a future boundary that can prove the inferred type is
   read-only, but not for the benchmark's mutable `Box`. Declared covariance is
   insufficient because Pyrefly intentionally does not enforce variance on
   ordinary fields.
2. **Recursive aliases.** Pyright expands one layer as
   `list[int | str | list[JsonValue]]`; Pyrefly retains the alias as
   `list[JsonValue]`. Both accept valid nested assignments and list writes,
   expose the same recursive element type, and reject non-`JsonValue` elements.
   This is a display-only difference and should be excluded from specificity
   progress totals.
3. **Defaulted ambiguous overloads.** Pyright commits to the first viable
   result, while Pyrefly retains
   `OverloadResult[list[int], list[str]]`. Pyright consequently accepts an
   integer append and rejects a string append; Pyrefly accepts either operation
   because each is supported by a viable alternative, while rejecting values
   supported by neither. This is the intentional gradual compatibility policy,
   not a result that should be changed to improve reveal-string agreement.

The actionable follow-up is therefore benchmark accounting: report these as
one intentional promotion-policy difference, one display-only family, and one
intentional overload-policy difference. Future implementation work should
target a behavioral incompatibility or a proven read-only constructor boundary,
not the remaining raw string mismatch count.

## Behavioral comparison scorecard

The next comparison should use explicit behavioral oracles rather than treating
checker agreement or a narrower reveal string as correctness. The checked-in
`scripts/typechecker_behavior_cases.py` corpus pairs accepted and rejected
operations in four categories:

- identifying type errors;
- preserving useful specificity;
- exposing valid members while rejecting invalid members; and
- selecting or retaining overload alternatives.

Run `scripts/compare_typechecker_behavior.py` with Pyrefly and Pyright binaries
to produce a scorecard and a table of disagreements. Every precision or member
probe has a negative counterpart so implicit `Any` cannot receive credit merely
for accepting an operation. Disputed policy cases remain visible individually
rather than being converted into an unexplained aggregate winner.

The initial 27-probe run against Pyrefly 1.2.0-dev.2 and Pyright 1.1.414 scored
27/27 and 25/27 respectively. The two disagreements are the intentional mutable
generic and ambiguous-overload policies described above. This targeted result
validates the scorecard, but the corpus is too small and too closely related to
this investigation to support a general claim that either checker is better.

The bug-augmented corpus separates normative probes from unscored compatibility
comparisons. After fixing keyword-bound `TypeGuard`/`TypeIs` narrowing
([#5263](https://github.com/pyrefly-org/pyrefly/issues/5263)) and required-key
checks through TypedDict unpacking
([#5405](https://github.com/pyrefly-org/pyrefly/issues/5405)), its 32 scored
probes give Pyrefly 32/32 and Pyright 31/32. Pyright loses the gradual
ambiguous-overload capability probe. Mutable generic writes and literal
retention through comprehensions
([microsoft/pyright#7586](https://github.com/microsoft/pyright/issues/7586))
remain visible as unscored policy comparisons.

For a broader baseline, `scripts/compare_typechecker_conformance.py` derives
1,528 hard-oracle probes from the checked-in official typing conformance files.
It scores required diagnostics, including warning-level requirements such as
PEP 702 deprecations, and single-line `assert_type` assertions using the suite's
Python 3.13 configuration. Optional `# E?` behavior is excluded, as are three
overload assertions that require the result to render exactly as `Any` even
though the typing specification permits a more precise gradual result.

The first run against the same checker versions produced:

| Category | Pyrefly | Pyright |
|---|---:|---:|
| Errors | 794/813 | 812/813 |
| Specificity | 409/413 | 413/413 |
| Members | 256/262 | 257/262 |
| Overloads | 40/43 | 42/43 |
| Total | 1499/1531 | 1524/1531 |
| Unexpected diagnostic locations | 34 | 37 |

After implementing the identified workstreams, the same normative comparison
produces:

| Category | Pyrefly | Pyright |
|---|---:|---:|
| Errors | 813/813 | 813/813 |
| Specificity | 413/413 | 413/413 |
| Members | 262/262 | 262/262 |
| Overloads | 48/48 | 48/48 |
| Total | 1536/1536 | 1536/1536 |
| Unexpected diagnostic locations | 2 | 2 |

The normative corpus therefore no longer favors either checker. The targeted
corpus favors Pyrefly by one scored overload capability and two unscored
inference-policy comparisons. Diagnostics outside the scored oracle locations
still require individual adjudication before they can be classified as false
positives.

The location totals changed substantially after correcting two accounting
problems in the comparison script. Optional `# E?` sites are valid diagnostic
locations even though they are excluded from normative scores, and the
conformance corpus includes `.pyi` files whose grouped overload oracles were
previously omitted. Counting both as known locations reduced the apparent
unexpected totals from 37/39 to 2/2 and added eight required stub-file overload
probes. This is a scorecard fix, not a checker behavior change.

The four remaining locations were adjudicated from their surrounding
conformance cases:

- Pyrefly reports two dataclass descriptor-definition diagnostics on cases the
  suite expects to be valid. These look like genuine descriptor/dataclass
  compatibility gaps, but changing descriptor semantics is not isolated enough
  for this measurement-only change.
- Pyright reports an invariant assignment on the line explicitly marked `# OK`
  in the variance-inference case. This looks like a variance-inference gap.
- Pyright also reports that a static method should not name its first parameter
  `self`. That is an additional style diagnostic rather than a failed protocol
  compatibility requirement.

## Diagnostic-quality scorecard

The targeted corpus now attaches structured quality oracles to eight invalid
operations covering assignments, calls, member access, protocols, TypedDicts,
generic bounds, and overload selection. The score does not compare complete
message strings. Instead, each oracle independently checks:

- whether the primary range overlaps the offending expression;
- whether the diagnostic kind or message identifies the broad category;
- whether expected and actual type fragments are present, allowing equivalent
  literal and widened forms;
- whether the relevant variable, parameter, member, or key is named;
- whether candidate declarations are exposed as related locations; and
- whether the message includes the expected actionable explanation.

The current branch build and Pyright 1.1.414 produce:

| Checker | Range | Category | Actual type | Expected type | Name | Related candidates | Hint | Total |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Pyrefly 1.2.0-dev.2 | 8/8 | 8/8 | 8/8 | 7/7 | 8/8 | 1/1 | 8/8 | 48/48 |
| Pyright 1.1.414 | 8/8 | 6/8 | 7/8 | 6/7 | 6/8 | 0/1 | 8/8 | 41/48 |

Pyrefly satisfies every structured quality oracle. It selects useful primary
ranges, identifies diagnostic subjects and both sides of type mismatches,
includes TypedDict unpack source types, and links locally defined overload
candidates through structured related locations with their expected signatures.
Imported or synthesized overloads without a local definition range retain the
closest-signature prose instead. Pyright provides every actionable hint but
omits the annotated assignment target and bounded-generic parameter names, does
not identify the TypedDict key/category or unpack source type, and does not
expose overload candidates through structured related locations.

Behavioral scoring remains separate from wording quality. On the expanded
36-probe targeted corpus, Pyrefly scores 36/36 and Pyright 33/36. Pyright misses
the guarded literal-union TypedDict member probe and two gradual
ambiguous-overload capability probes; the mutable-generic and comprehension
cases remain unscored policy comparisons.
