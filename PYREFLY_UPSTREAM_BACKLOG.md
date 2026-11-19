# Pyrefly upstream contribution backlog

This is the working queue for changes from `rchiodo-specificity-plan-review`. The
[investigation report](LITERAL_SPECIFICITY_INVESTIGATION.md) records the
benchmarks and design decisions; this file records what to propose upstream
and in what order. The fork branch is a reference, not a branch to submit as
one PR. The commit IDs below point to regression tests and implementation
ideas, not necessarily patches that can be cherry-picked onto current `main`.

The first submission, [pyrefly-org/pyrefly#5707](https://github.com/pyrefly-org/pyrefly/pull/5707),
names the offending constructor parameter in `bad-specialization` errors and
addresses [pyrefly-org/pyrefly#5706](https://github.com/pyrefly-org/pyrefly/issues/5706).
It was open when checked on 2026-11-19. Use its review feedback to shape the
next proposal. The other entries below are candidates, not filed issues.

## Next candidates

Recheck both cases on current upstream `main` once the first PR lands. If both
still reproduce, take the descriptor case first; it has an existing issue.

### Dataclass fields with overloaded descriptors

**Use [pyrefly-org/pyrefly#2065](https://github.com/pyrefly-org/pyrefly/issues/2065).**
Start with `Desc1` in `test_dataclass_overloaded_descriptor_conformance`
(`7842ca2b0`). Its overloaded `__get__` returns the descriptor for class access
and `int` for instance access; `__set__` accepts `int`. For `y: Desc1 = Desc1()`
in a dataclass `DC1`, `DC1(3)` should type-check, `DC1(3).y` should be `int`,
and `DC1.y` should be `Desc1`.

Keep the non-data descriptor case out of the first PR. Before porting the fork
patch, check that the proposed fix distinguishes the getter overloads selected
for class and instance access without dropping valid descriptor safety checks.

### Invalid discriminators in `TypedDict` unions

**Candidate title: Report the invalid tag in a `TypedDict` union.**
Pyrefly 1.4.1 gave a generic `dict` assignment error for this repro. Recheck
current `main`; if the diagnostic still omits the tag, it should name `kind`,
show `"bird"`, and list `"cat"` and `"dog"` without changing which assignments
are accepted. The fork regression is
`test_tagged_union_literal_mismatch_explains_discriminator` (`df3019138`).

```python
from typing import Literal, TypedDict

class Cat(TypedDict):
    kind: Literal["cat"]
    lives: int

class Dog(TypedDict):
    kind: Literal["dog"]
    bark: bool

pet: Cat | Dog = {"kind": "bird", "lives": 1}
```

## Other candidates

These are repro seeds from the fork. Verify each against current upstream
before reporting it.

| Priority | Candidate | Repro and scope | Fork reference |
| --- | --- | --- | --- |
| Verify first | Builtin shadowed in a quoted class annotation | A class defines `def int(self)` alongside `x: "int" = 0`. Verify the expected name lookup against the typing spec and `main`. Report this example rather than proposing a broad class-scope change. | `72b7ecdf4` |
| Verify first | Generic constructor loses its type parameter when passed as a callable | Pass a constructor with two `list[T]` parameters through a `Callable[P, R]` wrapper. The result should remain generic and reject a call mixing `list[int]` with `list[str]`. Keep the commit's other generic-scope changes separate. | `1a6c65d29` |
| Later | Link imported overload candidates from call errors | Call an overloaded function from a different file with arguments that match no overload. The LSP diagnostic should link to the candidates' actual declarations. Check what upstream already emits for local candidates first; don't change selection. | `3c54bb668`, `821325f13`, `2b4eea77c` |
| Later | Identify the inner type in a nested assignment error | Assign `Box[list[dict[str, str]]]` to `Box[list[dict[str, int]]]`. The actionable detail is the inner `str` vs. `int` mismatch. File only if `main` still omits it. | `ce5e23dc0` |
| Later | Identify the source of a bad `**kwargs` value | Pass a `TypedDict` with `x: str` to a function expecting `x: int`. If the diagnostic still omits the source, ask it to name the unpacked `TypedDict` and key. This isn't the dict-literal required-key bug. | `75e0b62c4` |
| Later | Preserve the parameter name in a decorated-call error | A decorator returns a `Callable` with a changed parameter type. When the original and returned parameter lists align, check that the error identifies the source parameter without changing call semantics. | `c516a40ce` |
| Separate behavior proposal | Validate an incompatible class decorator | Apply a decorator that expects `type[Expected]` to an unrelated class. If accepted on `main`, verify the expected rejection before proposing a fix. This can add a new error, so don't combine it with wording changes. | `c516a40ce` |

## Don't duplicate upstream work

| Area | Disposition |
| --- | --- |
| Keyword `TypeGuard` / `TypeIs` narrowing | [pyrefly-org/pyrefly#5263](https://github.com/pyrefly-org/pyrefly/issues/5263) already has [pyrefly-org/pyrefly#5535](https://github.com/pyrefly-org/pyrefly/pull/5535). |
| Missing required `TypedDict` keys in dict-literal unpacking | [pyrefly-org/pyrefly#5405](https://github.com/pyrefly-org/pyrefly/issues/5405) is closed as completed. Bring it back only with a distinct case that still reproduces. |
| Context-manager exception suppression and narrowing | [pyrefly-org/pyrefly#385](https://github.com/pyrefly-org/pyrefly/issues/385) is closed as completed. Revisit only if a distinct suppression or reachability case remains. |
| Protocol label in member mismatches | Park `ef26eda5c`. The checked `WrongRead.read` repro already names the member and displays `() -> int` vs. `() -> str`, including inside a tuple. A label alone is not a strong first contribution. |

## Keep policy, presentation, and measurement separate

- **Inference policy:** split literal retention through immutable tuples or
  control-flow joins, integer literal folding with loop widening, and gradual
  ambiguous-overload results (`bacdb3e43`) into separate design discussions.
  Keep intentional widening for writable generic storage. Don't file a
  catch-all Pyright parity issue or bundle these changes behind a single flag.
- **Lower-priority presentation:** recursive-alias rendering (`e6457ef2a`),
  original receiver context for union attribute errors (`df3019138`), and
  generic diagnostic subject names (`f0ada0a93`) need concrete cases where the
  current output leaves the user without enough information to fix an error.
- **Other correctness work:** split `bfdcfaaa0` by concrete behavior (callable
  Protocol compatibility, Protocol variance, and functional `TypedDict` name
  validation). Verify each independently and don't file one omnibus issue.
- **Supporting evidence:** the PEP 702 commit (`5e7e4df83`) adds regression
  coverage, not a behavior change. The comparison harnesses and
  [editor scorecard](EDITOR_FEATURE_SCORECARD.md) provide reproducible evidence;
  they're not standalone bug reports.

## Before proposing a change

1. Reproduce on current upstream `main`. Capture the exact output, expected
   behavior, and a minimal example. For a diagnostic-only proposal, check that
   the error kind, primary range, and acceptance behavior stay the same.
2. Check for an existing issue or PR. Prefer discussing the narrow repro on an
   existing issue; claim it only if unassigned and taking it on. Follow
   [upstream contribution guidance](https://github.com/pyrefly-org/pyrefly/blob/main/CONTRIBUTING.md)
   and [upstream AI policy](https://github.com/pyrefly-org/pyrefly/blob/main/AI_POLICY.md)
   when writing and submitting public material.
3. Port the smallest fix and regression onto a branch based on upstream `main`.
   Propose one change at a time rather than maintaining a stack against the
   investigation branch.
