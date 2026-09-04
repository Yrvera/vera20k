---
name: sync
description: "Audit and safely synchronize VERA20k branches and worktrees. Use for branch drift, fast-forward catch-up, or cleanup. Preserves unmerged work and local-only data."
---

# Sync

Apply `ENGINE.md`'s Git and ownership rules to the requested catch-up or cleanup.
An audit alone is read-only. Carry out actions already covered by the request;
ask only when ownership, preservation, or the intended target remains ambiguous.

## Establish actual state

Inspect status, branches/upstreams, worktrees, and active task ownership. For a
sync request, fetch with pruning before comparing tips. For a read-only audit,
use `git ls-remote --heads origin` without updating refs and label cached
remote-tracking information accordingly.

For relevant branches, establish local and remote tips, ahead/behind counts,
unique commits versus `main`, worktree ownership, and whether the work is
active, unpublished, merged, or safely removable. A squashed PR can preserve
changes without making its original commits ancestors; inspect that history
before claiming work is lost or disposable.

Require a clean, task-owned checkout before switching or updating its branch.
Use fast-forward-only catch-up. Do not replace a refused fast-forward with
history rewriting. Publication follows the existing task authorization.

## Owned PR conflicts

An actual conflict in a publication-authorized PR may be resolved by its task
or continuation owner. Verify base/head/current `origin/main`, then merge
`origin/main` into the clean feature branch. Resolve from both sides' intended
behavior, validate the affected scope, and obtain fresh review when behavior or
evidence changed. Finish or abort the merge before handing off. Other branch
divergence is not permission to perform this flow.

## Preserve local-only data before cleanup

Before removing a checkout or its ignored data, run the machine-local backup
command in `LOCAL.md`'s "Local-only backup" section. Require its successful
backup commit; investigate missing files or refused deletions instead of
bypassing the protection.

From the primary checkout, run:

```text
powershell -NoProfile -File .agents/skills/sync/scripts/check_worktree_cleanup.ps1 -Worktree <absolute-path>
```

The script checks the worktree root, external reparse points, dirty files, and
previews ignored cleanup. External or unresolved links stop removal. Classify
all ignored content as backed up, uniquely valuable, or regenerable; if the
preview was truncated, inspect the remainder. Research, retail INIs, local
configuration, and tools do not become disposable because Git ignores them.

Delete only requested or clearly authorized cleanup candidates after proving
their work is preserved and no task owns them. Use `git branch -d`, not `-D`.
If stale upstream metadata alone prevents deletion of a branch merged into the
protected history, remove that association and retry `-d`; otherwise report
the refusal. A remote-only deletion can retain a proven local copy.

Use ordinary `git worktree remove` after preservation checks. Do not use
root-wide `git clean -fdX` or force removal to bypass dirty files, external
links, or unclassified ignored data. Do not rebase, reset, amend, or force-push
as part of this workflow.

## Report

Verify final tips, requested deletions, and checkout state. Briefly list what
was updated, removed, kept, unpublished, or blocked, with the preservation or
ownership reason. An audit with no safe changes is a complete result.
