# Slice S5 — Passive-acquire scan-timer + RNG consumption (consume-only) — DESIGN

**Status:** DESIGN — ready for `/write-plan` after the two flagged write-plan decodes (§ Open RE).
**Date:** 2026-06-11
**Scope choice (user):** **consume-only first** — align the dominant scenario_rng draw + the
scan cadence in the gamemd-faithful host position, WITHOUT setting TarCom. The authoritative
acquire (TarCom-set + exact threat-score ranking + the conditional cloak-detect draw) is **S5b**.
**Parent:** S4 TechnoClass common bracket (`techno_common_post` host, S4a). S4c already shadows the
eligibility gate.
**Verified spec:** `GRIZZLY_PASSIVE_TARGET_SCANNER_VTABLE_39C_GHIDRA_REPORT.md` (audited 2026-06-11,
YELLOW — see AUDIT_LOG) + live decodes recorded below. **Lockstep-critical:** wrong draw
count/stream/position = full-match desync.

## Goal
Reproduce, in the `techno_common_post` host, gamemd's per-unit passive-acquire **scan cadence** and
its **per-scan `scenario_rng` draw** — deterministically and bit-aligned — without yet acquiring a
target.

## Architecture Context

gamemd (`TechnoClass::AI_Update`, post-`Mission_Dispatch`): once the unit's **scan timer**
(`+0x180` start / `+0x188` duration, in `g_CurrentFrameCounter` frames) has expired, AND the gate
passes (`vtable+0x4c4()==0` AND mission ∈ {Move 2, Harvest 10, Guard 5} AND
`PassiveAcquireGate`/`FUN_00709290`), it calls `vtable+0x39C` (`0x00709820`,
`Retaliate_And_Scan`). That function, **at its top, unconditionally**:
1. writes `+0x4FC = g_CurrentFrameCounter` (PassiveScanFrame),
2. **draws `Random__RandomRanged(0,2)` on `Scen->Random`** (scenario_rng; `ECX=[0x00A8B230]+0x218`,
   verified `disassemble 0x00709820 @0x0070984D/0x00709883`),
3. re-arms the scan timer: `start(+0x180)=g_CurrentFrameCounter`,
   `duration(+0x188) = base + roll`, where `base = RulesClass+0xe04` (`GuardAreaTargetingDelay`, when
   mission `+0xAC == 0xb` = Area Guard) else `RulesClass+0xe08` (`NormalTargetingDelay`).

Only AFTER that, and only when **TarCom (`+0x2B4`) is null**, does it run the scan body
(`vtable+0x3C4 → FUN_004D9920 → Greatest_Threat 0x006F8DF0`) and set TarCom via `vtable+0x3C8`
(`Assign_Target 0x006FCDB0`). That scan body — and its conditional cloak-detect draw — is **S5b**.

Current Rust: `combat_targeting.rs::acquire_best_target` ranks `(dist_sq, threat_class, stable_id)`
(nearest-first — wrong vs gamemd's threat-score), driven from `world_orders.rs::tick_order_intents_pre_combat`
(a *pre-combat* phase, only for `OrderIntent` entities). S4c (`techno_ai.rs`) already models the
eligibility predicate (`s4c_passive_acquire_eligible`: mission {Move,Guard,Harvest} + weapon +
(OpportunityFire ∨ Guard)). Neither runs the gamemd scan-timer cadence, and neither consumes RNG.

## Why consume-only is coherent (the RNG analysis that scopes it)

Two scenario_rng draws live in the scan path:

1. **Timer-jitter `RandomRanged(0,2)`** — *unconditional*, fires on every timer re-arm at the top of
   `0x00709820`, **independent of whether TarCom is set** (it runs whenever the gate is open, target
   or not). This is the dominant, high-frequency draw. **Consume-only reproduces it exactly.**
2. **Cloak-detect `RandomRanged(0,99)`** in `Evaluate_Candidate 0x006F7CA0` — *conditional*: gated on
   `target->vtable+0xC8()` (`UnitClass__Discovered_By`-family) AND `attackerType+0xd31==0` AND
   **not** `IsPlayerControl` (player-controlled short-circuits before drawing), rolled vs the
   per-AI-difficulty table `RulesClass+0xe14[Owner+0x184]`. It fires only inside the scan body,
   **only when TarCom is null** (which gamemd stops re-entering once a target is acquired). It is
   therefore **inherently coupled to modelling TarCom-set** — skipping the scan under-draws, running
   it without setting TarCom over-draws. **Cannot** be reproduced in any consume-only variant → S5b.

`Calculate_Threat_Score 0x0070CD10` is **RNG-free** (ftol/sqrt/health-ratio only), so the deferred
exact ranking adds zero stream complexity.

**Consequence:** consume-only is fully deterministic (VERA-lockstep-safe) and aligns the dominant
timer-jitter draw for *every* opportunity-firing unit. It does **not** achieve full gamemd-stream
parity in matches where AI units scan discovered-gated targets (those still drift on draw #2 until
S5b). This is a **named, accepted deferral**, blocked on the S5b target-set model.

## Impact Analysis

- **New scenario_rng consumption** in the live tick for every unit on Move/Guard/Harvest carrying a
  weapon with OpportunityFire (or on Guard). This is the **first** time this path draws RNG →
  **large but deterministic golden re-baseline** (unlike S4b's dormant field). Every armed-mover
  scenario shifts. Re-baseline after proving the shift is the new draws only.
- Touches: `techno_ai.rs` (`techno_common_post` — already rules-threaded by S4b), `game_entity.rs`
  (new hashed scan-timer field), `world_hash.rs` (fold), `ruleset.rs` (parse two keys),
  `snapshot.rs` (`SNAPSHOT_VERSION` bump).
- Determinism: the scan timer is hashed authoritative state (it gates future draws). Must use a
  deterministic frame counter (`binary_frame`) and the existing `SimRng` mask-reject `n(0,2)`.
- Does NOT touch the existing `OrderIntent`/`acquire_best_target` pre-combat path — that stays the
  (wrong-ranked) acquire until S5b relocates+fixes it. **Known transient drift:** during S5, VERA
  still acquires via the old nearest-first path AND now also consumes the scan-timer RNG. The two
  coexist; S5b reconciles them (removes the old path, moves acquire into the host).

## Tiny-Detail Ledger (parity constraints)

- Scan timer is in **frames** = `g_CurrentFrameCounter` → model on `session.binary_frame` (NOT
  `session.tick`). `[ini: rulesmd.ini:305 "targeting rates in frames"]` `[GHIDRA 0x0070982e +0x180=g_CurrentFrameCounter]`
- Re-arm duration = `base + RandomRanged(0,2)` (inclusive 0..2). `base = GuardAreaTargetingDelay` iff
  mission `+0xAC == 0x0b` (Area Guard) else `NormalTargetingDelay`. `[GHIDRA 0x0070983a CMP +0xac,0xb]`
- `NormalTargetingDelay` **default 27**, `GuardAreaTargetingDelay` **default 36** (`[General]`,
  `ReadInt`). `[ini: rulesmd.ini:304-305]` `[GHIDRA RulesClass__ReadGeneral: s_GuardAreaTargetingDelay→+0xe04, s_NormalTargetingDelay→+0xe08]`
- The draw is **`RandomRanged(0,2)` on `scenario_rng`** (= `SimRng::next_range_u32_inclusive(0,2)`),
  exactly one per re-arm. `[GHIDRA 0x0070984D MOV ECX,[0x00A8B230];ADD ECX,0x218;CALL Random__n]`
- Draw/re-arm happens **only when the full gate passes** (timer expired AND `vtable+0x4c4()==0` AND
  mission∈{2,10,5} AND PassiveAcquireGate). Timer-expired-but-gate-closed ⇒ no draw, timer stays
  expired (fires immediately when the gate next opens). `[GHIDRA 0x006fa679..0x006fa67d]`
- Initial timer state fires on the **first** eligible tick: gamemd sentinel `start==-1, duration==0`
  ⇒ `duration==0` ⇒ fire. Model with the `MissionTimer` SENTINEL. `[GHIDRA 0x006fa64e if start==-1: if dur==0 fire]`
- `n(0,2)` consumes **one** raw draw (span 2, mask 3, reject>2). `[src/sim/rng.rs next_range_u32_inclusive]`
- Eligibility gate = S4c predicate (mission {2,10,5} + weapon + (OpportunityFire ∨ Guard)). `[techno_ai.rs s4c_passive_acquire_eligible]`
- **DEFERRED to S5b (named):** the scan body, TarCom-set (`Assign_Target` not "Set_ArchiveTarget"),
  exact threat-score ranking (`Greatest_Threat` strict-greater / equal-keeps-scan-order /
  quarter-half early-return / radius `(range>>8)+1+(type+0x68c>>8)`), the cloak-detect
  `RandomRanged(0,99)`, and `+0x4FC`/`+0x50C`. `[doc: GRIZZLY_PASSIVE_TARGET_SCANNER §5, this design §"Why consume-only"]`

## Chosen Approach (consume-only)

- **Host:** `techno_common_post(sim, id, rules)` (S4a bracket, Unit arm — gamemd AI_Update position).
  Runs per live Unit, after the mission commit. (Already rules-threaded by S4b.)
- **Per-entity hashed state:** `passive_scan_timer: MissionTimer` (reuse the existing `start_frame`
  +`duration` + SENTINEL primitive) — the `+0x180/+0x188` equivalent, in `binary_frame` units.
  `#[serde(default)]`, folded into `hash_entities`.
- **Parse:** `GeneralRules.normal_targeting_delay` (default 27) + `guard_area_targeting_delay`
  (default 36) from `[General]` ints.
- **Per-Unit step:**
  1. eligibility gate (S4c predicate) + the `vtable+0x4c4` pre-check (see Open RE) — if closed, return.
  2. if `passive_scan_timer` not expired at `binary_frame`, return.
  3. expired + gate open → `roll = sim.scenario_rng.next_range_u32_inclusive(0,2)`;
     `base = if mission==AreaGuard(11) {guard_area} else {normal}`;
     re-arm `passive_scan_timer = MissionTimer::armed(binary_frame, base + roll)`.
  4. **no** TarCom set, no scan body, no `+0x4FC/+0x50C`.
- `SNAPSHOT_VERSION` 25→26; golden re-baseline (verify the shift is only the new draws — every
  armed-mover scenario shifts; the rng-routing/global-parity tests will need re-baselining).

## Design

### Components
- `GeneralRules`: `normal_targeting_delay: u32` (27), `guard_area_targeting_delay: u32` (36).
- `GameEntity.passive_scan_timer: MissionTimer` (hashed; `#[serde(default)]` = SENTINEL).
- `techno_ai.rs`: a `passive_scan_consume(sim, id, rules)` helper called from `techno_common_post`.

### Interfaces / Contracts
- Reuses `MissionTimer::{armed, is_expired, SENTINEL}` (start_frame + duration vs a `now` frame) —
  the established frame-timer primitive (same as gates/invulnerability).
- Mission read: the entity's derived/committed mission for the {2,10,5}/{11} classification (the host
  has already committed `mission.current`); Area-Guard detection needs a Mission==AreaGuard mapping.

### Data Flow
`advance_tick → object_ai_stage → unit_techno_bracket → techno_common_post → passive_scan_consume`
(reads `sim.session.binary_frame`, the entity mission + type weapon/OpportunityFire, mutates
`sim.scenario_rng` + `entity.passive_scan_timer`).

### Error Handling
`rules == None` or unknown type ⇒ return (no draw), mirroring S4b. Miners (Unit arm) run the host;
they pass the gate only if armed + on an eligible mission (a harvester on Harvest(10) with a weapon +
OpportunityFire would scan — matches gamemd; stock harvesters have no weapon ⇒ gate fails).

### Testing Strategy (mirror S4b's `state()`-diff exactness)
- `s5_no_draw_when_gate_closed` (no weapon / non-eligible mission ⇒ 0 draws).
- `s5_one_draw_on_timer_expiry` (eligible + expired ⇒ exactly one `n(0,2)`; timer re-armed to
  `base+roll`).
- `s5_no_draw_while_timer_live` (eligible but not expired ⇒ 0 draws).
- `s5_area_guard_uses_guard_delay` (mission 11 re-arms with 36-base, else 27).
- `s5_draws_from_scenario_not_main`.
- `s5_first_tick_fires` (SENTINEL initial ⇒ fires on first eligible tick).
- `s5_golden_rebaselined` (version bump; deterministic replay; baselines re-measured, shift = new draws only).

### Determinism
Scan timer hashed; `binary_frame`-keyed; single `scenario_rng` draw per re-arm via the existing
mask-reject helper. Two VERA clients with identical inputs draw identically (lockstep-safe).

## Architectural Decisions
- Follows the S4b pattern exactly (rules-threaded host helper + hashed per-entity gate field +
  `state()`-diff tests + version bump + golden re-baseline). No new pattern.
- Reuses `MissionTimer` rather than a bespoke timer struct — matches how VERA models gamemd CDTimers.
- Does NOT relocate the existing `OrderIntent` acquire (that's S5b) — avoids a half-migration this slice.

## Open RE (resolve at /write-plan — both bounded)
1. **`vtable+0x4c4` (`0x004DF310`) gate semantics.** Reads `this+0x5c4` (`MOV EDX,[ECX+0x5c4]`).
   Decode what `+0x5c4` represents (the predicate that must be 0 to allow the passive scan; likely a
   recoil/firing/special-state). For consume-only, model it as the gate's extra term; if it's a
   state VERA doesn't track yet, default-open and flag (like S4b's `vtable+0x1c8`). Also: the
   `else` branch (`vtable+0x4cc` when `+0x4c4 != 0`) — confirm it draws no scenario_rng on the
   eligible path, else it's an additional consume item.
2. **Mission-id 0x0b = Area Guard mapping** in VERA's `MissionType` — confirm the enum value so the
   base-delay selector (`guard_area` vs `normal`) is exact.

## Alternatives Considered
- **Flip-now with placeholder ranking** — sets a wrong (nearest-first) target every match; player-
  visible drift; rejected (user chose consume-only).
- **Decode-first full S5** — decode the cloak-roll condition fully + relocate acquire + exact score in
  one slice; larger, and the score is RNG-free so it doesn't need to precede the cadence work.
  Deferred to S5b.
- **Run the scan body without setting TarCom** — over-draws the cloak-detect roll (gamemd stops once
  TarCom is set); not faithful. Rejected.

## Deferred follow-ups (S5b and beyond)
- S5b: relocate acquire into the host, set TarCom (`Assign_Target`), exact threat-score ranking
  (needs `Calculate_Threat_Score 0x0070CD10` decode — RNG-free), the cloak-detect `RandomRanged(0,99)`
  (needs the `vtable+0xC8`/`+0xd31`/`RulesClass+0xe14` decode), `+0x4FC/+0x50C`, and retire the old
  `OrderIntent` nearest-first path. The §4 `Evaluate_Candidate` filter offsets (cloak/sensor,
  visibility `+0x41A/0x41B`, bridge `OnBridge`, `type+0x231`, `weapon+0xB4`) spot-verified then.
