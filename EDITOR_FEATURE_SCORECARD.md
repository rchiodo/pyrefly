# Editor feature scorecard

This scorecard measures small, deterministic language-server behaviors rather
than subjective editor impressions. It covers quick fixes, hover, completion,
go-to-definition, and rename for Pyrefly and the open Pyright language server.
Pylance is listed separately because its extension-only behavior is not
available through reproducible open tooling.

## Methodology

The checked-in probes live in `scripts/compare_editor_features.py`. The runner
uses only the Python standard library and communicates with each server over
LSP stdio. Every feature has a semantic oracle:

- **Quick fixes:** an unresolved `Greeter` must offer an edit importing the
  symbol from `library.py`.
- **Hover:** hovering `greet` must include the symbol name, parameter and return
  types, and its docstring.
- **Completion:** completing a known `str` receiver must include `upper`.
- **Navigation:** go-to-definition on `greet` must resolve into `library.py`.
- **Rename:** renaming `message` must edit its declaration and two semantic
  references without treating `other_message` as a reference.

The oracles intentionally avoid exact titles, Markdown layout, completion
ordering, URI spelling, and full JSON payload equality. The runner creates a
temporary workspace, opens the same files in each server, and deletes the
workspace afterward.

Pyrefly also has focused assertions in
`pyrefly/lib/test/lsp/editor_scorecard.rs`. These use the lightweight in-process
LSP helpers rather than the heavyweight `lsp_interaction` harness. They protect
the scorecard's Pyrefly result without changing language-server behavior.

The comparison was run on November 19, 2026 with:

```text
python scripts/compare_editor_features.py \
  --checker "Pyrefly=<pyrefly binary> lsp" \
  --checker "Pyright=npx --yes --package pyright@1.1.414 pyright-langserver --stdio"
```

The Pyright package is pinned for reproducibility but is not added as a
repository dependency.

## Results

| Server | Version | Quick fixes | Hover | Completion | Navigation | Rename | Comparable total |
|---|---|---:|---:|---:|---:|---:|---:|
| Pyrefly | 1.2.0-dev.2 | Pass | Pass | Pass | Pass | Pass | 5/5 |
| Pyright | 1.1.414 | No matching action | Pass | Pass | Pass | Pass | 4/5 |
| Pylance | Not tested | N/A | N/A | N/A | N/A | N/A | N/A |

Pyrefly returned an import edit for the unresolved symbol, a typed and
documented hover, the receiver-appropriate completion, the correct definition
target, and exactly the three expected rename edits.

The open Pyright server passed the hover, completion, navigation, and rename
oracles. It returned no import quick fix matching the unresolved-symbol probe.
That is an observed Pyright LSP result, not evidence that Pylance lacks the
editor action: Pylance can add extension-side features, but its proprietary
extension cannot be run as a pinned open stdio server in this repository.
Consequently, no Pylance result is scored or inferred from Pyright.

This is a narrow capability scorecard, not a general ranking. It does not
measure latency, ranking quality, large-workspace behavior, framework-specific
features, or UI presentation.
