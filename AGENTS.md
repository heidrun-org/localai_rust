# AGENTS.md

## Issue Fix Workflow

Every issue fix in this repository follows the skill [issue_fix_workflow](.agents/skills/issue_fix_workflow/SKILL.md). Read the skill before you start an issue fix, each time that the developer asks to fix an issue.
The skill names no detail of this repository: it reads the values of this repository in the section "Issue Fix Workflow values" below.

### Issue Fix Workflow values

- Target branch: main
- Worktree creation command: `git worktree add .agents/worktrees/<branch> -b <branch> main`
- Worktree folder: `.agents/worktrees/<branch>`
- Launch command: `cd .agents/worktrees/<branch> && pnpm install && pnpm test`
- Issue tracker commands: `gh issue view`, `gh issue comment`, `gh issue create`, `gh pr create`, `gh pr merge --merge`, `gh issue close` (all with `--repo heidrun-org/localai_rust`)
- Chat rename: not available
