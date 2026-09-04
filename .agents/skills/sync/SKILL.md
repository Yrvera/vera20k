---
name: sync
description: "Audit or synchronize VERA20k branches/worktrees while preserving ownership, unmerged work, and local-only data."
---

# Sync

Apply ENGINE's Git/authorization rules. Inspect status, branches/upstreams,
worktrees, and task ownership. Fetch/prune for synchronization; for a read-only
audit use `git ls-remote --heads origin` without ref updates.

Establish relevant tips, ahead/behind counts, unique work versus `main`, and
ownership. Squashed PRs can preserve changes without preserving commit ancestry;
inspect history before declaring work disposable.

Use clean, owned checkouts and fast-forward-only catch-up. An actual conflict in
a publication-authorized PR may be resolved by its owner: verify base/head/current
`origin/main`, merge `origin/main` into the feature branch, resolve both sides'
intent, validate, and review changed behavior/evidence. Finish or abort the merge.
Other divergence is not permission to rewrite history.

## Cleanup

Before removing a checkout or ignored data:

1. Run the primary checkout's `LOCAL.md` "Local-only backup" command. Require a
   successful backup commit; do not bypass missing files or refused deletions.
2. From the primary checkout run:

   ```text
   powershell -NoProfile -File .agents/skills/sync/scripts/check_worktree_cleanup.ps1 -Worktree <absolute-path>
   ```

3. External/unresolved links stop removal. Classify all ignored content as backed
   up, uniquely valuable, or regenerable; inspect any truncated preview remainder.
   Gitignored research, INIs, configuration, and tools are not disposable.

Delete authorized candidates only after proving preservation and no active owner.
Use `git branch -d`, never `-D`. If stale upstream metadata alone blocks deletion
of a branch merged into protected history, unset that association and retry `-d`;
otherwise report refusal. Remote-only deletion may retain a proven local copy.

Use ordinary `git worktree remove`; never bypass dirty files, external links,
or unclassified data with force removal or root-wide `git clean -fdX`.
This workflow never rebases, resets, amends, or force-pushes.

Verify final tips, deletions, and checkout state. Report updated/removed/kept/
unpublished/blocked work with its preservation or ownership reason.
