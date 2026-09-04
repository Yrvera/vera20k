# Checkpoints and handoffs

For sustained implementation, keep one concise current checkpoint; replace it rather
than append a diary. A small completed task needs only its final result. Use the task's
own worktree or the goal's requested location; do not create a shared status ledger.

When handing off, record worktree/branch and HEAD, relevant unmerged commits, changed
files and dirty state, literal validation output and coverage, review/PR status,
residuals/blockers, and the exact next safe action. Include scope amendments,
publication authority and stop conditions so continuation does not revive old intent.
Mark interrupted edits or incomplete builds honestly; do not rerun work to beautify
the checkpoint.

At a planned pause finish the current operation and leave no merge or owned Cargo run
pending. On an immediate stop, start no new investigation, test or cleanup; preserve
work, stop assigning workers, and report any ongoing operation. Do not kill a compile
or another task's process. On resumption, reconcile actual Git/process state before
acting on this checkpoint.