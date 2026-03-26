Fetch and display details for a GitHub issue in the pyrefly repository.

Usage: /issue-info <issue_number>

The issue_number can be in the format #1234 or just 1234.

Steps:
1. Parse the issue number from: $ARGUMENTS
2. Detect the environment: check if a `BUCK` file exists in the pyrefly project root to determine internal vs external.
3. Fetch the issue with comments:
   - **Internal:** `buck run fbcode//pyrefly/facebook/scripts:issue_info -- show <issue_number> --comments`
   - **External:** `gh issue view <issue_number> --repo facebook/pyrefly --comments`
4. Present a summary of the issue including:
   - Title, status, labels, assignee
   - Description
   - Key comments (skip bot/automated comments, focus on human discussion)
