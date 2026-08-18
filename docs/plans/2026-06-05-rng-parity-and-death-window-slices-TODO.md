# TODO — RNG-cursor parity + death-window slices (continue 2026-06-05)

*Authored 2026-06-04. Handoff for the next session. Source of truth for the findings:*
`docs/research/substrate/LOGICCLASS_TICK_RNG_SUBSTRATE_NEAR_COMPLETENESS_AUDIT_20260604.md`
*(full status matrix, 55 gaps, rung map, do-not-rewrite list). This file is just the actionable plan.*

---

## Where we left off (state at end of 2026-06-04)

- Ran a 17-agent read-only audit of LogicClass tick order + RNG streams. Verdict: substrate is **near-complete** on data shapes / walk primitive / frame timing / RNG instance routing / save-load-hash. The remaining frontier is three things, captured as 5 ranked slices below.
- **No code written yet.** A worktree off dev was set up and then **reverted** at the user's request — the user will set up the worktree/branch themselves, then say "continue."
- Rule for all of this work: **gamemd-native semantics, Rust-native structure.** These slices **deliberately change the lockstep hash** (they fix wrong RNG draws / death timing) — acceptance tests are hash-NOT-neutral by design, except Slice 4's Presence change.

## Branch / workspace (user is handling)

- Branch off **dev** (the user will set this up; worktree or in-place, their call).
- ⚠️ The main working tree had **14 uncommitted files from an active parallel session** (incl. `world/mod.rs`, `world_hash.rs`, `snapshot.rs`). Don't entangle them. A worktree off dev avoids the collision; merge conflicts later are accepted by the user.
- ⚠️ Worktrees don't get gitignored dirs — **copy `ini/` into the worktree** before any cargo (include_str! needs it).

## ⚠️ Read-this-first caveats before coding

1. **Line numbers in the audit are from the `factory-house-substrate-p1p2` branch (dc7a34d9), NOT dev.** Re-grep every site on the dev-based branch — numbers and surrounding code will differ.
2. **Verify which substrate pieces exist on dev (89ba3ca7) before starting Slices 4–5.** The death-window machinery (`ObjectSubstrate`, `pending_delete`, two-phase `uninit`/`flush_pending_delete`, `Presence` FSM, `derived_presence`) landed on the factory branch — confirm it's actually merged to dev. If dev lacks it, Slices 4–5 are blocked off dev (branch off a substrate branch, or rebase). **Slices 1–3 are independent of the substrate and fine off dev.**
3. **Subagent audit claims are unverified per project discipline.** For each slice, re-verify the exact gamemd algorithm (draw count / order / variant / boundary) against the binary + the cited `*_RNG_CLASSIFICATION` doc BEFORE implementing. A wrong `>=` vs `>` or off-by-one rejection ships wrong parity.
4. **Run cargo as a separate foreground pass** (`cargo check -p vera20k`, then targeted `cargo test -p vera20k <module>`). Do NOT bury it in a background workflow. Read the literal `test result:` line before claiming pass/fail.

---

## The plan: workflow shape

Recommended for the next session (ultracode on): one workflow with
**Phase Verify (5 parallel, read-only)** → produce a precise, binary-grounded implementation spec per slice (exact helper signature, exact call replacements with re-grepped file:line, exact boundary conditions, exact test assertions); then
**Phase Implement (sequential, dependency order)** → write Rust-native code + acceptance tests.
Then **foreground cargo gate + per-slice commit**. Keep Slice 5 last and watch it closely (it may expose latent consumer bugs — that's the point).

Dependency order: **1 → 2** (share the raw-modulo helper) → **3** (independent) → **4 → 5** (4 unblocks 5).

---

## SLICE 1 — Particle RNG raw-modulo conversion  ·  [SMALL-IMPL]  ·  rank 1

- [ ] Add one raw signed-abs-modulo helper to `src/sim/rng.rs` (`abs(next_u32() as i32) % n` shape — confirm the exact gamemd primitive: `Random__Next` then `% n`, matching the existing `raw_probability_sample` shape).
- [ ] Replace every `next_range_u32(n)` (mask-reject) at particle lifetime/jitter/offset/insert sites with the raw-modulo helper. Audit-cited sites (RE-GREP on dev): `particles/{spawn:96/99/229, fire:65/116, smoke:88/89/178/213, gas:86/87/198}`.
- **Why rank 1:** particles spawn on nearly every explosion; each `next_range_u32` rejection advances the **shared scenario cursor** a variable number of times → desyncs every later scenario consumer that tick. Mechanical, no spine touch.
- **gamemd semantics:** `PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` (current, matches code). Verify the modulo is computed on the raw draw, not a ranged draw.
- **Acceptance tests (hash-NOT-neutral):** fixture spawning a known particle burst (e.g. MaxEC=80); assert the scenario cursor advances by exactly `count` raw draws (one/spawn), not the variable rejection count; assert produced values equal `abs(Next) % MaxEC` for a seed=1 hand-table; assert `state_hash` changes from baseline then is stable across re-runs. Regression test: fail if any `particles/*` site reintroduces `next_range_u32`.

## SLICE 2 — Smudge 50/50 + wall-damage RNG variant fix  ·  [SMALL-IMPL]  ·  rank 2

- [ ] Anim scorch/crater 50/50 (`combat/smudge_dispatch.rs:212`, re-grep): replace raw-high-bit with `RandomRanged(0,0x7FFFFFFE)` normalized, accept `< 0x40000000`, with `0x7FFFFFFF` rejection (matches gamemd draw COUNT).
- [ ] Wall damage (`map/overlay_grid.rs:366-367`, re-grep): switch `next_range_u32(strength)` (`[0,S-1]`) to `next_range_u32_inclusive(0,strength)` (`[0,S]`); change `roll > damage` to **return/no-op when `roll >= damage`** (gamemd inclusive boundary). Diverges at `roll == damage` and the range top.
- **Why rank 2:** same shared-scenario-cursor-shift class, fires on every scorch/crater anim + every wall hit below strength.
- **gamemd semantics:** `SMUDGE_RNG_CLASSIFICATION`, `WALL_DAMAGE_RNG_CLASSIFICATION`.
- **Acceptance tests (hash-NOT-neutral):** Wall — assert no-op at `roll == damage`, draw-consumed-then-no-damage; advance only when `roll < damage`; pin inclusive range `[0,strength]`. Smudge — assert scorch chosen when masked `< 0x40000000`; a masked `0x7FFFFFFF` triggers a redraw (extra cursor advance). Compare cursor after a fixed scorch+crater+wall-hit sequence to a hand-derived count.
- **Dep:** reuses Slice 1's raw-modulo helper — do Slice 1 first.

## SLICE 3 — `random_assignment` SP color + draw-order parity  ·  [SMALL-IMPL]  ·  rank 3

- [ ] In `resolve_random_assignments` (`skirmish_launch.rs:291-306`, re-grep) add the random **color** draw (`RandomRanged(0,7)` with collision-retry) per human node and AI slot, in gamemd's node/slot order (all humans country→color, then all AI), matching `0x0069B8C0`.
- **Scope:** SP path only. The MP branch uses a network callback (`vtable+0x6c/+0x70`, zero RNG) — **out of scope** until the net layer exists.
- **Why rank 3:** offsets the scenario cursor **before tick 0** → desyncs the whole match from tick 1; fires every skirmish. Instance routing already correct (binary-confirmed Scen).
- **gamemd semantics:** `0x0069B8C0` (verify the collision-retry loop count — it's part of the cursor advance).
- **Acceptance tests (hash-NOT-neutral, pre-tick-0):** given a fixed config (N humans, M AI, all "random"), assert the scenario cursor advances by exactly `2*(N+M)` draws (country+color) in gamemd order, retries counted; resulting (country,color) tuples match a seed=1 golden table cross-checked vs `0x0069B8C0`; tick-0 `state_hash` differs from baseline.

## SLICE 4 — Decouple Presence FSM from drain timing (+ defeat-before-AI)  ·  [SMALL-IMPL]  ·  rank 4

- [ ] Make `derived_presence` (`game_entity.rs:510-516`, re-grep) return `Dying` when `dying` is set, so `debug_assert_presence_consistent` no longer depends on all corpses being force-drained first.
- [ ] Move `check_defeat` (`world/mod.rs:~1924`, re-grep) to run **before** the AI-manage step (gap 18) so a house defeated this tick can't issue AI commands.
- **Why rank 4:** pure invariant-hardening that **unblocks Slice 5** — today the Presence assert would false-fire the moment you reduce the drain count.
- **Acceptance tests:** Presence change is **hash-neutral** (debug assert + unhashed `serde(skip)` field) — assert `state_hash` byte-identical before/after across a fixture run; unit test a `Dying` entity surviving to the assert → `derived_presence()` returns `Dying`, assert passes. Defeat reorder: a house whose last building dies this tick issues no AI command that tick; hash-neutral only if no house's defeat status changes on the boundary tick (if it does, it's a correctness fix — pin the new hash).
- **Dep / blocker:** requires the substrate death-window on the branch (see caveat #2).

## SLICE 5 — Dying-gates on raw-store consumers, then collapse to one drain  ·  [LARGE-MIGRATION]  ·  rank 5

- [ ] Add a `dying`-gate to every raw-store consumer that currently relies on the early drains: vision (P3), power (P4), production (P7), AI (P8), particles (P5.5), retaliation (P6).
- [ ] Then remove the command-boundary drain (`world/mod.rs:1954`) and end-of-P5 drain (`:2477`), leaving the single end-of-tick drain (`:1903`) to match gamemd's `ProcessPendingDelete` @ end-of-`Main_Tick`. (Re-grep all three; numbers are from the factory branch.)
- **Why rank 5 / risk:** highest structural impact (restores gamemd's Dying-window visibility to mid-tick systems — kill-credit, last-attacker, power/vision counting) but highest risk — touches the spine + every raw-store consumer, and the current drains are **masking real consumer bugs** (in-code comments at `:1946-1953` and `:2470-2476` admit this). Removing them may surface those bugs — expected; handle as they appear. **Must come after Slice 4.**
- **gamemd semantics:** `SLICE6_DEFERRED_DELETE_DYING_WINDOW` (§3.4-3.5, §8, §10; single end-of-tick drain; `vtable+0x44` = `IsAlive==0` always-true post-UnInit).
- **Acceptance tests (hash-NOT-neutral):** per consumer, a corpse uninit'd this tick must be excluded by the dying-gate from its count. Critical: **kill-credit + last-attacker** tests — an instant-hit kill of B before B's turn must make B's `last_attacker`/retaliation reflect the Dying window exactly as gamemd. After collapse, membership + presence asserts still pass and `state_hash` is deterministic across re-runs.
- **Dep:** Slice 4 (Presence must derive `Dying` first).

---

## DO NOT REWRITE (carry-over from audit §8)

- Don't fill the no-op `object_ai_stage`/`techno_ai_shell` — absorption is a later, sequenced migration.
- Don't collapse the subsystem-phase split (movement/combat/retaliation) — gated on Slice 5 + the projectile-authority decision.
- Don't reorder the hashed-order contracts (`state_hash` fold, `LogicVector` serialize, `SimRng` serde layout) — lockstep/replay contract; lock with a golden-preimage test before touching.
- Don't unify the `_scaled` (mapgen) vs `_inclusive` (RandomRanged) RNG helpers — different draw counts. (Slices 1-2 fix call sites, not the helpers.)
- Don't remove the command-region drain (`:1954`) in isolation — it comes out *with* Slice 5's dying-gates.

## Side quest (independent of the slices, do anytime)

Two doc families actively misroute future RE work — patch with `/verify-doc` or `/audit`:
- `PER_FRAME_RNG_CONSUMPTION_ORDER` + `RNG_SYSTEM §3.1` — mislabel several Scen->Random consumers (lightning, ore, smudge, particles) as g_MainRng.
- `SYNC_CHECKSUM_MAINTICK_OBJECT_SUM` + `DESYNC_DETECTION_MAINTICK_COMPARE` — label `+0xD64` an "8-byte state_hash"; a re-decompile shows it's the g_Tactical camera **scroll (x,y)**. This also leaves J4 ("gamemd has no live MP checksum") UNCHECKED until `Main_Tick` is re-decompiled.

## Open decisions for tomorrow

- Confirm substrate death-window is on the chosen branch (gates Slices 4-5).
- Commit cadence: per-slice commits (matches project slice-commit style) vs grouped.
- Whether Slice 5 stops for review after the dying-gates (before removing drains) given it may expose consumer bugs.
