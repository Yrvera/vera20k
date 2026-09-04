---
name: shadow
description: "Observe an active VERA20k program across tasks when requested as a shadow/watchdog. Reports new risks without editing, building, or directing workers."
---

# Shadow

Read the program's goal and stop condition. Identify participating tasks, branches,
worktrees, and PRs; paused work can retain ownership. With no active program,
report that and do not start recurring work.

Watch for scope drift, contradictory assumptions, competing owners, aging PRs,
missing required reviews, and accumulating parity/determinism risk. Judge the
chosen contract, not new gates. Inaccessible evidence is not proof of missing work.

Stay read-only: no checkout/ref/PR/Ghidra mutations, builds, or worker messages.
Use read-only remote queries instead of fetch. Inspect unrelated tasks only when
evidence connects them to the program; send advice to the user.

For standing observation, use the host's scheduling/loop mechanism and the user's
cadence/preferences. Keep cursors and SHAs outside the checkout; read only new
commits, PR changes, and transcript turns. A one-time assessment needs no loop.

Notify only on meaningful changes, citing evidence and trigger/frequency; prioritize
imminent data loss or competing writers. Stay quiet otherwise, using the host's
no-op form if required. At the user's stop or program completion, stop this
observer and its own recurring follow-up, leaving other tasks untouched.
