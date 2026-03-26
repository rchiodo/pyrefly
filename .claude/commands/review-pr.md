Review a GitHub PR from the pyrefly opensource hub (PRCC).

The pyrefly PRCC repo ID is `3616826258465545`.

Usage: /review-pr <pr_number> or /review-pr <repo_id> <pr_number>

If only one argument is given, it is the PR number and the pyrefly repo ID is used.
If two arguments are given, the first is the repo ID and the second is the PR number.

Steps:

1. Parse the arguments from: $ARGUMENTS
2. Detect the environment: check if a `BUCK` file exists in the pyrefly project root. If it does, this is an internal (Meta) dev environment. Otherwise, it's an external (GitHub) checkout.
3. Fetch the PR with diff and comments:
   - **Internal:** Use the PRCC CLI:
     `buck run fbcode//pyrefly/facebook/scripts:prcc -- show <repo_id> <pr_number> --diff --comments`
   - **External:** Use the GitHub CLI:
     `gh pr view <pr_number> --repo facebook/pyrefly --comments`
     `gh pr diff <pr_number> --repo facebook/pyrefly`
4. Check the PR description and comments for references to GitHub issues (patterns like `#1234` or `Fixes #1234`).
   - **Internal:** Fetch them with: `buck run fbcode//pyrefly/facebook/scripts:issue_info -- show <issue_number> --comments`
   - **External:** Fetch them with: `gh issue view <issue_number> --repo facebook/pyrefly --comments`
   - Use the issue context to understand the motivation behind the PR and whether the changes fully address the issue.
5. Review the code changes following pyrefly's review guidelines (see AGENTS.md):
   - **Correctness**: Logic errors, edge cases, null/Option handling, off-by-one errors
   - **Security**: Input validation, injection risks, path traversal
   - **Performance**: Unnecessary iterations, excessive allocations/cloning, blocking in async
   - **Testing**: Missing test coverage, edge cases not covered
   - **Style**: Naming conventions (snake_case functions, CamelCase types), consistency with codebase
   - **Architecture**: Separation of concerns, tight coupling, breaking changes
   - **Pyrefly-specific**: Prefer `impl Trait` over `dyn Trait`, use `?` for error propagation, avoid `.unwrap()` in production code, check for helpers in `pyrefly_types` crate, KISS and DRY
6. Provide a structured review:

   ### Summary
   2-3 sentence overview of what the PR does and overall assessment.

   ### Issues Found
   Organized by severity:
   - **Critical**: Must fix (correctness, security)
   - **Major**: Should fix (performance, maintainability)
   - **Minor**: Nice to fix (style, documentation)

   Each issue should include file/line reference, description, and suggested fix.

   ### Positive Observations
   Well-done aspects of the code.

   ### Questions
   Clarifying questions for the author.

To list open PRs:
- **Internal:** `buck run fbcode//pyrefly/facebook/scripts:prcc -- list 3616826258465545`
- **External:** `gh pr list --repo facebook/pyrefly`
