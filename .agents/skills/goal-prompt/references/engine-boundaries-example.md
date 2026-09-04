# Goal prompt examples

These are illustrative prompts, not live instructions or a historical status report.
Replace the scope and acceptance properties with those of the actual request.
Neither example grants publication authority.

## Bounded architecture work

> Improve ownership of the scenario-load path so each piece of persistent simulation
> state has one clear owner and the app only orchestrates loading. Follow ENGINE.md.
> Inspect current consumers and preserve behavior: compare the same representative
> loads before and after through deterministic hashes and snapshot bytes, with coverage
> for the affected lifecycle handoffs. Choose the Rust boundaries that best fit those
> responsibilities. Use an independent critic with access to source, evidence, the diff
> and validation; it may challenge missing consumers or unnecessary complexity. Finish
> when the selected ownership defects are resolved and the production load path passes
> the stated comparisons. Record unrelated findings separately.

The outcome and comparisons constrain the work. They do not prescribe traits, file
sizes, a scan sequence or a frozen inventory of everything a critic may inspect.
Hashes and bytes here establish refactor preservation, not parity with gamemd.

## Continuation header

> Resume the existing goal in `<worktree>` on `<branch>` at `<HEAD>`. The latest
> checkpoint is `<path>`; `<unmerged work>` awaits `<review or validation>`. Reconcile
> ownership and current Git state, retain supported prior evidence, and resume at
> `<next safe action>`. Preserve the governing goal and subsequent scope/authority
> amendments below. Recheck any premise contradicted by current source.

Fill placeholders from actual state. Keep inherited work and unresolved review visible
without copying a session diary or asserting that past completion claims remain true.
