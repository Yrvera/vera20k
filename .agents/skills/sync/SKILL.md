---
name: sync
description: >
  Audit and safely synchronize this repository's GitHub Flow branches:
  protected main plus short-lived feature/* branches. Use for branch drift,
  safe fast-forward pulls, branch or worktree cleanup, or questions about what
  is out of date. Never force-pushes, rewrites history, deletes unmerged work,
  or removes unprotected local-only data.
---

# Sync — GitHub Flow Branch Audit

Audit branch state first, then perform only explicitly authorized safe actions.
The standard lifecycle is `main` → `feature/<topic>` → PR → `main` → cleanup.
There is no long-lived `dev` branch.

## Safety rules

- Require a clean working tree before switching branches or updating refs.
- Fetch with pruning before drawing conclusions.
- Use fast-forward-only pulls and merges. Never rebase, reset, amend, or force-push.
- Never commit or push directly to `main`; it moves through reviewed PRs or an
  explicit user-owned GitHub action.
- Never push an unpublished/local-ahead feature branch without user authorization.
- Branch deletion requires explicit user confirmation after the audit identifies
  the exact local/remote refs and proves where their unique commits remain.
- Use `git branch -d`, never `-D`. If safe deletion refuses, stop and explain why.
- Do not switch or delete a branch checked out by another worktree or active task.
- Before removing a worktree or cleaning ignored files, run
  `powershell -NoProfile -File .agents/skills/sync/scripts/check_worktree_cleanup.ps1 -Worktree <absolute-path>`
  from the primary checkout. Treat an external junction/symlink as a hard stop,
  and classify every ignored path as backed up, uniquely valuable, or
  regenerable before proceeding.
- Never run root-wide `git clean -fdX`. Ignored project contracts, skills,
  research, INIs, and tool source are authoritative local data, not build litter.

## 1. Preflight and fetch

For a normal sync run, use:

```text
git status --porcelain
git worktree list --porcelain
git fetch --all --prune
```

If the user explicitly requests a read-only/no-ref-mutation audit, do not fetch or
prune. Use `git ls-remote --heads origin` for live remote tips, label local
remote-tracking refs as cached, and make no changes.

If the tree is dirty, another task owns the checkout, or Cargo/worktree ownership is
unclear, stop before mutation and report the conflict.

### Local-only backup preflight

Before an authorized action could remove a checkout or local-only data, refresh
the adjacent private backup with:

```text
powershell -NoProfile -File ..\vera20k-docs-backup\backup.ps1
```

Require a successful backup commit before continuing. If the backup script is
missing, refuses because source files disappeared, or reports unapproved
deletions, stop. Do not bypass it by copying a few visible files or by passing
the deletion override without separately reviewing the exact stale paths.

## 2. Audit

Collect:

```text
git branch --format='%(refname:short)|%(upstream:short)|%(upstream:track)|%(objectname:short)'
git rev-list --left-right --count <branch>...main
git ls-remote --heads origin
```

For `main` and every relevant feature branch report:

- local tip and upstream tip;
- ahead/behind relative to upstream;
- commits unique to the branch versus `main`;
- whether it is checked out in any worktree;
- whether it is unpublished, active, merged, or a cleanup candidate.

Treat a legacy `dev` ref like any other obsolete branch. Never recreate or
fast-forward it. It is deletable only when it has zero unique commits versus `main`
and the user explicitly confirms local/remote deletion.

Present the audit before changing refs. A compact example:

```text
main                    [origin/main] sync
feature/current-task    (local only)  4 unique commits — keep
feature/merged-task     [origin/...]  0 unique commits — delete candidate
```

## 3. Classify actions

- **NO ACTION** — synchronized or legitimate unique work.
- **PULL MAIN** — local `main` is behind `origin/main` and can fast-forward.
- **PULL FEATURE** — a clean feature branch is behind its upstream and can fast-forward.
- **PUBLISH CANDIDATE** — local branch has unpushed commits; report only unless the
  user asked to push or open/update a PR.
- **DELETE CANDIDATE** — branch has zero unique commits versus `main`, or the user
  explicitly wants a remote-only deletion while retaining a proven local copy.
- **ANOMALY** — divergence, missing commits, unclear ownership, dirty state, or a
  protected-branch mismatch. Stop and request direction.

## 4. Execute safe catch-up

For an authorized pull:

```text
git switch main
git pull --ff-only origin main
```

or:

```text
git switch feature/<topic>
git pull --ff-only
```

If `--ff-only` refuses, stop. Do not substitute a merge commit or rebase.

## 5. Clean up confirmed branches

After rechecking unique commits and worktree ownership:

```text
git branch -d feature/<topic>
git push origin --delete feature/<topic>
git fetch origin --prune
```

For a confirmed remote-only deletion that keeps the local branch:

```text
git push origin --delete feature/<topic>
git branch --unset-upstream feature/<topic>
```

If stale upstream metadata makes `git branch -d` refuse even though the branch is
merged into the current protected history, remove only that upstream association and
retry `-d`; never escalate to `-D`.

For worktree removal, first run the local-only backup preflight and the bundled
cleanup check. `git worktree remove --force` is forbidden while the check reports
external reparse points, dirty files, or unclassified ignored content.

## 6. Verify and report

Verify exact local/remote tips, absence of deleted refs, a clean working tree, and no
unexpected branch switch. Report kept, updated, deleted, skipped, and unpublished
branches in one screen. Zero mutations is a valid result when nothing is safely
actionable.

## Never do

- `git push --force` or `--force-with-lease`
- `git rebase`, `git reset --hard`, or `git commit --amend`
- `git branch -D`
- direct commits or pushes to `main`
- automatic publication of local feature work
- deletion of an unmerged or worktree-owned branch
- root-wide ignored-file cleanup
- forced removal of a worktree containing external junctions or symlinks
- recreation of a long-lived `dev` branch
