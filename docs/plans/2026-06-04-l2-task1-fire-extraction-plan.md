# L2 Task 1 — Extract per-object Phase-2 fire body (`resolve_attacker_fire`)

> **For Claude:** Execute task-by-task. Each task is self-contained. This is a
> **pure refactor — zero behavior change.** The gate on every task is that
> `cargo test -p vera20k combat` stays at **143 passed; 0 failed** and the
> per-tick `state_hash` is unmoved. If any test fails or the hash moves, STOP —
> the extraction changed behavior; do not "fix" the test, fix the extraction.

**Goal:** Lift the Phase-2 fire-decision/emission loop body of `tick_combat_with_fog`
into a free function `resolve_attacker_fire`, leaving the sweep calling it in a
loop, with bit-identical behavior — the reusable per-attacker entry the later L2
shadow `unit_post` host will call.

**Architecture:** `sim/combat` only. No new authority, no phase move, no hash
change, no `SNAPSHOT_VERSION` bump. The batched Phase-3..6 damage-apply/death
model is preserved verbatim (justified by the verdict doc: damage is
deferred-projectile, so no inline-kill coupling exists to thread per-object).

**Design Doc:** `docs/plans/2026-06-02-ai-shell-unitclass-core-plan.md` (Slice L2,
Task 1) + gating verdict `docs/research/L2_FIRE_DAMAGE_TIMING_VERDICT_GHIDRA_REPORT.md`.

---

## Grounding Summary

- **Verdict doc** (`L2_FIRE_DAMAGE_TIMING_VERDICT_GHIDRA_REPORT.md`, verified live
  this session): gamemd applies weapon HP **deferred** (munition AI/detonation,
  never inline in the fire call). Consequence: the Rust **batched** P4/P6
  damage-apply/death model is correct and must be **kept**; Task 1 extracts the
  **P2 fire-emission body only**.
- **Live structure** (`src/sim/combat/mod.rs`, re-verified this session — re-Read
  before editing, lines drift): `tick_combat_with_fog` @ `:1183`; snapshot build
  loop ~`:1376`, push `:1494`, `sort_by_key` `:1533`; **Phase 2** comment `:1540`,
  16 emit vecs `:1542-1557`, loop `for snap in &snapshots {` `:1559`, body through
  `~:2132`; Phase 3 `:2133`, 3b `:2168`, 3c `:2177`, **Phase 4** `:2190`, Phase 5
  `:2222`, **Phase 6** `:2231`.
- **Borrow pattern (verified):** the P2 loop (`:1559-2132`) only *reads* `entities`
  (`entities.get`, `acquire_best_target(entities, …)`) — **zero** `entities.get_mut`
  in range; *reads* `occupancy` (`&*occupancy`); *mutates* `interner`
  (`interner.intern` ×9 — warhead/weapon/anim strings); does **not** reference
  `resource_nodes`/`fire_blocked`/`current_tick`/`live_order`/`power_states`/`keys`.
  All entity mutation is deferred to Phases 3-6 via the 16 emit vecs.
- **Control flow (verified):** the loop body uses **only `continue`** (`:1569,
  1658, 1679, 1709, 1738, 1755, 1771, 1774, 1848, 1860, 1890, 1908`) — no `break`,
  no `return`, no labeled loops. Each `continue` → an early `return` in the function.
- **Repo pattern mirrored:** the existing snapshot→emit→apply phase split already
  separates decision (P2, read-only + push) from mutation (P3-6) — the extraction
  preserves it exactly, only relocating the P2 body behind a function boundary.
- **INI:** none — pure refactor, no new constants.
- **Unknown after grounding:** the exact `&`-vs-`&mut` set is compiler-confirmed in
  Task 4 (the candidate signature below is derived from the verified usage; the
  compiler is the oracle).

## Key Technical Decisions

- **Bundle the 16 P2 emit vecs into one `#[derive(Default)] struct CombatEmit`** so
  the body can push through a single `&mut out` instead of 16 `&mut Vec` params.
  **Confidence:** high — **Source:** plan L2 §4 Task 1; repo has no existing emit
  bag, this is a new local struct (private to `combat/mod.rs`).
- **Destructure `emit` back into the 16 named locals after the loop** so Phases
  3-6 are textually **untouched** (minimal diff, zero risk to the consumers).
  **Confidence:** high — **Source:** standard Rust; preserves the exact var names.
- **`resolve_attacker_fire` stays in `combat/mod.rs`** (not a new file) for this
  task; the `unit_post` host (`src/sim/world/unit_post.rs`) is a later L2 slice.
  **Confidence:** high — **Source:** plan L2 §3 (host file is Task 2+ scope).
- **Signature takes `&EntityStore` + `&mut StringInterner` + `&OccupancyGrid`** (the
  outer fn holds `&mut`/`&mut`/`&mut`; reborrow immutably where the body only
  reads). **Confidence:** medium → compiler-confirmed in Task 4. **Source:**
  verified grep of the body's handle usage this session.

## Open Questions

### Resolved During Planning
- *Is the extraction safe given the batch model?* — Yes. Verdict doc: damage is
  deferred, so P2 is read-only on HP; the batch stays. No per-object damage-apply
  threading needed.
- *Does the body mutate entities mid-loop?* — No (zero `get_mut` in `:1559-2132`).
- *Only `continue` for control flow?* — Yes (verified); maps cleanly to `return`.

### Deferred to Implementation
- **Exact param list / mutability of `resolve_attacker_fire`.** The candidate
  signature is below; the compiler in Task 4 is the definitive oracle — add/remove
  a handle or flip `&`/`&mut` exactly as the moved body requires. Do not guess past
  the compiler.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/combat/mod.rs` | Add `CombatEmit` struct + `resolve_attacker_fire` fn; replace the P2 loop with a call + destructure. No other change. |

## Interface Changes

- `struct CombatEmit` — **private** to `combat/mod.rs` (no `pub`). Nothing external
  depends on it.
- `fn resolve_attacker_fire(...)` — **private** (`fn`, no `pub`) for this task. A
  later L2 slice widens visibility when `unit_post` consumes it. Nothing external
  depends on it yet.
- `tick_combat_with_fog`'s **public signature is unchanged** — only its body is
  refactored. Its `CombatTickResult` output is unchanged.

## Sim Checklist

- [x] No new f32/f64 — pure relocation of existing code; arithmetic unchanged.
- [x] No new hashed state — `CombatEmit` is a transient local, never stored/hashed;
      `SNAPSHOT_VERSION` untouched.
- [x] No dependency on render/ui/sidebar/audio/net — stays within `sim/combat`.
- [x] Tick ordering unchanged — Phase 2 stays Phase 2; the call replaces the loop
      in place. No `advance_tick` phase moves.
- [x] Iteration order unchanged — still `for snap in &snapshots` over the same
      sorted snapshot vec; the function is called once per snap in the same order,
      so emit/push order (and the downstream `scenario_rng` smudge cursor) is
      byte-identical.

## Risk Areas

- **Highest blast radius:** `combat/mod.rs` is hash-critical. The risk is a
  transcription slip during the body move (a missed `out.` prefix, a reordered
  push, a `continue` not converted to `return`). Mitigation: the move is verbatim
  except the 3 mechanical transforms; the 143-test combat suite + the per-tick
  hash are the gate.
- **Borrow surprises:** passing `&mut interner` and `&mut out` alongside
  `&entities`/`&occupancy` — all distinct objects, no aliasing with `&snapshots`.
  If the compiler flags a borrow conflict, it is a signal the body touches
  something not in the candidate signature — resolve by matching the body, not by
  cloning to dodge the borrow.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 2-3 | **Emit/push order within and across attackers** | The downstream `scenario_rng` smudge drain consumes `smudge_spawn_requests` in emission order; any reorder shifts the lockstep RNG cursor → desync. | Same per-snap call order + verbatim push order; `cargo test -p vera20k combat` + a per-tick `state_hash` spot-check. |
| 2 | **`continue` → `return` conversion** | A missed conversion would change which later code in the body runs for that attacker (behavior change). | Grep the moved body: every former `continue` is now `return`; no stray `continue` (would be a compile error — no enclosing loop). |
| 2 | **All 16 `<vec>.push` → `out.<vec>.push`** | A missed prefix = a use of an undefined local (compile error) or, worse, a shadowed local that silently drops emissions. | Compile (Task 4) + diff review: exactly the 16 names are rewritten; no other identifiers touched. |
| 3 | **Destructure field set == 16 declared vecs** | A missing field = the consumer phase reads a stale/empty vec. | The `let CombatEmit { .. } = emit;` names all 16; Phases 3-6 compile unchanged. |

---

## Tasks

### Task 1: Add the `CombatEmit` struct

**Why:** Define the emit bag before the function that fills it (types-first).

**Files:** Modify `src/sim/combat/mod.rs` (add the struct near `tick_combat_with_fog`,
above `:1183`, or just below the existing `use`/type decls — placement is cosmetic).

**Pattern:** New local struct; mirrors the exact element types of the existing
P2 locals (`:1542-1557`). Re-Read those lines first to confirm types still match.

**Step 1: Add the struct**
```rust
/// Transient per-tick bag of the Phase-2 fire-emission outputs. Bundles the
/// emit vectors so the per-attacker fire body (`resolve_attacker_fire`) can push
/// through one `&mut` handle. Never stored on `Simulation`, never serialized,
/// never hashed — destructured back into the named locals after the Phase-2 loop.
#[derive(Default)]
struct CombatEmit {
    /// (target_id, damage, attacker_id, warhead_id)
    damage_events: Vec<(u64, u16, u64, InternedId)>,
    remove_attack: Vec<u64>,
    /// (attacker_id, new_target_id)
    retarget_events: Vec<(u64, u64)>,
    fire_events: Vec<SimFireEvent>,
    reveal_events: Vec<RevealEvent>,
    bridge_damage_events: Vec<BridgeDamageEvent>,
    wall_damage_events: Vec<WallDamageEvent>,
    terrain_damage_events: Vec<TerrainDamageEvent>,
    tiberium_reduction_requests: Vec<TiberiumReductionRequest>,
    explosion_effects: Vec<ExplosionEffect>,
    smudge_spawn_requests: Vec<SmudgeSpawnRequest>,
    /// (id, burst_rem, burst_delay, rof_cd)
    burst_updates: Vec<(u64, u8, u8, u16)>,
    /// aircraft that fired this tick
    ammo_deduct: Vec<u64>,
    /// building IDs to advance fire index
    garrison_advance: Vec<u64>,
    pending_infantry_updates: Vec<(u64, Option<PendingInfantryFire>)>,
    animation_switches: Vec<(u64, SequenceKind)>,
}
```

**Step 2: Verify**
Pre-flight: `tasklist | grep -iE "cargo|rustc"` (PowerShell-via-Bash is denied;
use `tasklist`). Then `cargo check -p vera20k -q` → exit 0 (unused-struct warning
is fine at this stage).

**Step 3: Commit** — `git add src/sim/combat/mod.rs && git commit -m "sim/combat: add CombatEmit bag for Phase-2 fire emission (L2 Task 1, no behavior change)"`

---

### Task 2: Define `resolve_attacker_fire` and move the Phase-2 loop body into it

**Why:** This is the extraction. Relocating the body (not retyping it) keeps it
bit-identical.

**Files:** Modify `src/sim/combat/mod.rs` (add the function; e.g. directly after
`tick_combat_with_fog`).

**Pattern:** Move-by-reference. Re-Read `:1559-2132` immediately before editing to
get the current exact range (it drifts).

**Step 1: Add the function shell with the candidate signature**
```rust
/// Resolve one attacker's Phase-2 fire decision + emission for the current tick.
/// READ-ONLY w.r.t. entities/occupancy (HP/death are applied later in the batched
/// Phase 4/6); it reads target/rules/occupancy/fog and pushes events into `out`.
/// Interns warhead/weapon/anim strings (hence `&mut StringInterner`). Pure w.r.t.
/// iteration order: the caller invokes it once per snapshot in live-LOGIC order,
/// preserving emission order exactly.
fn resolve_attacker_fire(
    snap: &AttackerSnapshot,
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &mut StringInterner,
    fog: Option<&FogState>,
    occupancy: &OccupancyGrid,
    overlay_grid: Option<&OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
    binary_frame: u32,
    tick_ms: u32,
    out: &mut CombatEmit,
) {
    // <moved body goes here — see Step 2>
}
```

**Step 2: Move the loop body into the function with exactly three mechanical transforms.**
Cut the **inside** of `for snap in &snapshots { … }` (the lines between `:1559`'s
`{` and its closing `}` at `~:2132`) and paste it as the function body. Apply,
and ONLY apply, these transforms to the pasted text:
1. **Push redirection:** rewrite each of the 16 emit-vec pushes to go through `out`:
   `damage_events.push` → `out.damage_events.push`, and likewise for
   `remove_attack`, `retarget_events`, `fire_events`, `reveal_events`,
   `bridge_damage_events`, `wall_damage_events`, `terrain_damage_events`,
   `tiberium_reduction_requests`, `explosion_effects`, `smudge_spawn_requests`,
   `burst_updates`, `ammo_deduct`, `garrison_advance`, `pending_infantry_updates`,
   `animation_switches`. (These are the only 16 identifiers that move to `out`.)
2. **Control flow:** replace every `continue;` in the body with `return;`
   (there is no enclosing loop in the function; the former per-iteration skip is
   now a per-call early return). There are no `break`/`return` statements to touch.
3. **Occupancy reborrow:** the AoE context built in the body passes
   `occupancy: Some(&*occupancy)`; since the param is already `&OccupancyGrid`,
   leave it as `Some(&*occupancy)` (still valid) OR simplify to `Some(occupancy)` —
   either compiles; prefer leaving it verbatim to minimize diff.

Do **not** alter any other line: not the order of pushes, not the arithmetic, not
the helper calls (`acquire_best_target`, `combat_aoe::apply_aoe_damage`,
`combat_weapon::select_garrison_weapon`, `target_coords`, `cell_center_coords`,
etc.), not the per-branch logic.

**Step 3: Verify (deferred to Task 4 — the loop still references the old locals
until Task 3, so the crate will not compile between Task 2 and Task 3; that is
expected. Do Tasks 2 and 3 as one edit session, then compile.)**

---

### Task 3: Replace the Phase-2 loop with a call + destructure-back

**Why:** Wire the sweep to the extracted function while keeping Phases 3-6 reading
the same named locals (zero change to consumers).

**Files:** Modify `src/sim/combat/mod.rs` at the former Phase-2 loop site
(`~:1542-2132`).

**Step 1: Replace the 16 `let mut <vec> = Vec::new();` declarations (`:1542-1557`)
and the whole `for snap in &snapshots { … }` loop (`:1559-2132`) with:**
```rust
    // Phase 2: per-attacker fire decision + emission, in live-LOGIC snapshot
    // order. Each attacker is resolved through `resolve_attacker_fire` (the
    // reusable per-object fire body); emission order is identical to the prior
    // inline loop, so the downstream scenario_rng smudge cursor is unmoved.
    let mut emit = CombatEmit::default();
    for snap in &snapshots {
        resolve_attacker_fire(
            snap,
            entities,
            rules,
            interner,
            fog,
            occupancy,
            overlay_grid,
            overlay_registry,
            terrain,
            binary_frame,
            tick_ms,
            &mut emit,
        );
    }
    // Destructure back into the named locals so Phases 3-6 are untouched.
    let CombatEmit {
        damage_events,
        remove_attack,
        retarget_events,
        fire_events,
        reveal_events,
        bridge_damage_events,
        wall_damage_events,
        terrain_damage_events,
        tiberium_reduction_requests,
        explosion_effects,
        smudge_spawn_requests,
        burst_updates,
        ammo_deduct,
        garrison_advance,
        pending_infantry_updates,
        animation_switches,
    } = emit;
```

**Step 2: Leave Phases 3-6 (`:2133` onward) exactly as-is** — they consume the
named locals, which now come from the destructure. No edits below the destructure.

**Note on `mut`:** Phases 3-6 may mutate some of these locals (e.g. re-use a vec).
If the compiler warns a destructured binding needs `mut`, add `mut` to that field
binding in the destructure (e.g. `mut damage_events,`). Match the original
declarations: any vec that was `let mut` and is later pushed/drained in P3-6 needs
`mut` here. (Most are consumed by-value/iterated; add `mut` only where the
compiler asks.)

---

### Task 4: Compile and reconcile the signature

**Why:** The compiler is the oracle for the exact handle set/mutability.

**Step 1:** Pre-flight `tasklist | grep -iE "cargo|rustc"`, then `cargo check -p vera20k -q`.

**Step 2:** Reconcile any error **by matching the moved body, never by changing
behavior**:
- *"cannot find value `X` in this scope"* inside `resolve_attacker_fire` → the body
  uses a captured local/param `X` not in the signature; add it as a param and pass
  it at the call site (read-only `&` unless the body mutates it). Candidates not in
  the current signature would be a surprise — re-Read the body and confirm.
- *"cannot borrow … as mutable"* / *"as immutable"* → flip the param's `&`/`&mut`
  to match the body's usage (e.g. if the body only reads `interner`, narrow to `&`;
  the grep showed it interns, so `&mut` is expected).
- *"binding `X` does not need to be mutable" / "needs to be mutable"* in the
  destructure → add/remove `mut` per Task 3 Step 2's note.
- A leftover bare `<vec>.push` (missing `out.`) → "cannot find value" — apply the
  Task 2 transform #1 to it.

**Step 3:** Re-run `cargo check -p vera20k -q` until exit 0, no errors. Unused-import
or dead-code warnings unrelated to this change are out of scope (do not touch).

---

### Task 5: Verify bit-identical behavior

**Why:** Prove the refactor changed nothing observable.

**Step 1:** `cargo test -p vera20k combat` — read the literal `test result:` line.
**Expected: `143 passed; 0 failed`** (the count before this change). Confirm
`-p vera20k` (a wrong `-p` exits 101 without running). If any combat test fails,
**STOP and revert** — the extraction changed behavior; do not edit the test.

**Step 2:** Run the broader sim suite as a regression check:
`cargo test -p vera20k` (separate bounded pass). All previously-green tests stay
green; in particular any replay/`state_hash` golden tests are unmoved (the refactor
adds/removes no hashed state). Read the literal `test result:` lines.

---

### Task 6: Commit

`git add src/sim/combat/mod.rs && git commit -m "sim/combat: extract per-object Phase-2 fire body into resolve_attacker_fire (L2 Task 1, no behavior change)"`

(Commit on `dev`. The plan doc itself lives under `docs/` which is gitignored —
do not add it.)

---

## Sources & References

- **Design doc:** `docs/plans/2026-06-02-ai-shell-unitclass-core-plan.md` (Slice L2, Task 1).
- **Gating verdict:** `docs/research/L2_FIRE_DAMAGE_TIMING_VERDICT_GHIDRA_REPORT.md`
  (deferred-projectile → keep the batched P4/P6 model; verified via
  `decompile_function 0x736df0`, `get_function_callees 0x6FDD50`,
  `get_function_callers 0x489280`/`0x468D80`).
- **Live code (this session):** `src/sim/combat/mod.rs` — `tick_combat_with_fog` `:1183`,
  Phase-2 vecs `:1542-1557`, loop `:1559-2132`, Phases 3-6 `:2133-2250`; signature
  `:1183-1199`. Re-Read before editing.
- **Related:** `src/sim/world/mod.rs` (Phase-5 combat call site + smudge drain — NOT
  edited here); `src/sim/world/unit_post.rs` (future L2 host — NOT created here).
