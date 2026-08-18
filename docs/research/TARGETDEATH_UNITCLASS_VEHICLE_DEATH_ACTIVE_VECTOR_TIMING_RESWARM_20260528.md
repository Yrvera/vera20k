# UnitClass Vehicle Death — Active-Vector Removal Timing

**Addresses:** `UnitClass::ReceiveDamage @ 0x00737C90`, `ObjectClass::UnInit @ 0x005F65F0`, `UnitClass vtable @ 0x007F5C70`
**Investigation Mode:** targeted-slice
**Claimed Scope:** When a ground vehicle (UnitClass) is killed during another object's AI turn, whether it leaves the LogicClass active vector immediately (synchronous, mid-pass) or deferred. Covers: vtable override verification, case-4 death branch call chain, husk/wreck entity status, and Rust delta.
**Non-Scope:** Damage math, NavalUnit sinking (covered by SUBMARINE_AND_SINKING), building death (slot 4), infantry death (slot 3), apply_area_damage snapshot (covered by slot 5). Re-deriving scheduler mechanics (already in COMMON_MIDPASS and LOGICCLASS_PERTICKUPDATE).
**Confidence:** HIGH on removal timing and vtable identity; HIGH on no-alive-husk finding; MEDIUM on Crashable=yes crash-state path (see Remaining Uncertainty).
**Active in YR:** Yes — all paths documented here are the standard stock YR land vehicle death path.

## 0. Investigation Contract

**Target question.** When a vehicle is killed by a hit during another object's AI turn (e.g. bullet detonation via `BulletClass::AI`), does it leave the LogicClass active vector SYNCHRONOUSLY inside the `ReceiveDamage` call chain (mid-attacker-pass), or is removal deferred to a later phase?

**Non-goals.** Do not re-derive damage formulas. Do not investigate naval sinking. Do not re-prove scheduler compaction mechanics (already proved in COMMON_MIDPASS).

**Evidence needed to mark COMPLETE.**
- Verified UnitClass vtable override slot for `ReceiveDamage`.
- Decompiled case-4 (HP≤0) death branch call chain tracing to `vtable+0xF8` / `ObjectClass::UnInit`.
- Determination of whether any alive husk entity is appended to the logic vector.
- Rust delta: current removal timing vs. native removal timing.

**Stop conditions.** Stop after case-4 branch is fully traced. Stop if a Ghidra path is TS-only or dormant in YR.

## 1. UnitClass vtable Override Verification

`UnitClass` vtable base confirmed at `0x007F5C70` (via `TECHNOCLASS_VTABLE_COMPLETE.md`; UnitClass vtable-method map in `UNITCLASS_GHIDRA_REPORT.md`).

`ReceiveDamage` is vtable slot 91, offset `+0x16C` (from `TECHNOCLASS_VTABLE_COMPLETE.md` row 141).

Verification: `read_memory(0x007F5C70 + 0x16C = 0x007F5DDC, 4)` → `90 7C 73 00` = `0x00737C90` = `UnitClass__ReceiveDamage`. **Verified via `read_memory 0x007F5DDC`**.

The separate 5-byte thunk `UnitClass__ReceiveDamage @ 0x00740E50` (vtable+0xE0) calls `FUN_00708C30` and is a DIFFERENT slot — it is NOT the case-4 death dispatch slot. The authoritative override is `0x00737C90` at `+0x16C`. **Active in YR: Yes.**

## 2. Case-4 Death Branch — Call Chain

`UnitClass::ReceiveDamage @ 0x00737C90` (decompiled in this session) first calls `FootClass__ReceiveDamage(...)`. If the return value is `4` (UNIT_DIED), execution enters the `case 4` block.

### 2.1 Non-ship death path (standard ground vehicle)

Condition check (case 4 entry): `param_1[10].vtable_INoticeSource` is `TechnoTypeClass*` (the type pointer). The non-ship branch fires when any of `puVar10[0xCCE]` (Naval), `puVar10[0xD69]` (Underwater), `puVar10[0xD97]` (Organic) are false OR weight below `ShipSinkingWeight`, OR current cell `LandType != 2` (water). Standard ground vehicles always satisfy this — none are Naval/Underwater/Organic, and they live on non-water cells.

**Step sequence for a normal ground vehicle kill:**

| Step | What happens | Evidence |
|------|-------------|----------|
| 1 | `vtable+0x3B8` (disengagement/stop firing) | Decompile `0x00737C90` case-4 head |
| 2 | `UnitClass__Death_Explosion @ 0x00738680` — spawns debris/explosion AnimClass | Decompile `0x00737C90`; call before `LAB_00737f74` |
| 3 | `LAB_00737f74`: `vtable+0x124` (RecordKill) | Decompile `0x00737C90` after `LAB_00737f74` |
| 4 | CargoClass::ClearAllInOpenTransport (if `type+0x5E4` = IsOpenTopped) | Decompile `0x00737C90` |
| 5 | FootClass::EMPPassengers if transport | Decompile `0x00737C90` |
| 6 | Survivor/crew eject loop (if `type+0xD95 == 0`, i.e. NOT Crashable): iterates passengers via `FUN_004DE710` | Decompile `0x00737C90`; `CARGO_CLEAR_ALL_IN_OPEN_TRANSPORT_DAMAGE_TIMING_RESWARM_20260528.md` confirms `type+0xD95 = Crashable` |
| 7 | Parachute/infantry survivor eject (if `Crewed` at `type+0xCCD`) | Decompile `0x00737C90`; Crewed block |
| 8 | Crate drop (if `type+0xE1A` = CarriesCrate, via `WallOverlay_HeightAdjust` block) | Decompile `0x00737C90`; `CRATE_SYSTEM_GHIDRA_REPORT.md` confirms `+0xE1A` = CarriesCrate |
| 9 | **`(**(code **)(param_1->vtable + 0xf8))()`** — `ObjectClass::UnInit` | Decompile `0x00737C90`, final `if (type+0xD95 == 0) { if (flag==0) { vtable+0xF8 } }` block |

**`ObjectClass::UnInit @ 0x005F65F0`** (decompiled in this session):
- Defuses attached BombClass
- Calls `vtable+0xD4` (Limbo) → `ObjectClass::Conceal @ 0x005F4D30` → `FUN_0055BAE0` **compacts the active vector and clears `Object+0x98`**
- Clears `Object+0x90` (IsAlive)
- Appends to `PendingDeleteList @ 0x00B0F69C` for deferred destructor

**Result: the vehicle is removed from the LogicClass active vector SYNCHRONOUSLY during the `ReceiveDamage` call, which is itself called from within the attacker's AI turn.** The vector compacts LEFT at the removed slot; the scheduler increments the index without repair; the object shifted into the dead vehicle's old slot can be SKIPPED this pass. **Active in YR: Yes.**

### 2.2 Crashable path (jumpjets and crash-sequence vehicles, `type+0xD95 != 0`)

If `type+0xD95` (Crashable) is nonzero, the final block calls `(**(code **)(param_1->vtable + 0x3dc))()` instead. If that returns 0, the same `vtable+0xF8` path is taken (synchronous removal). If it returns non-zero, the entity enters a crash/dying state and is NOT immediately unregistered — it remains in the active vector. Crashable is false for stock ground vehicles (tanks, miners, war miner, tesla tank, etc.). **Active in YR: Conditional on `Crashable=yes`, which is primarily jumpjets/aircraft-style units, not standard ground vehicles.**

## 3. Husk / Wreck — Is a New Live Entity Registered?

**Verdict: No.** Standard ground vehicles in YR do NOT spawn an alive husk entity that is registered in the LogicClass active vector.

Evidence from `SUBMARINE_AND_SINKING_GHIDRA_REPORT.md` (§B.8): "Ground units do NOT have a wreck anim timer in gamemd.exe; the explosion animation is just a spawned AnimClass and the entity is deleted the same frame." The "VXL wreck" in that report refers to a cosmetic debris VoxelAnim, not a new UnitClass instance.

From the `UnitClass::ReceiveDamage` decompilation: the case-4 non-ship path calls `UnitClass__Death_Explosion` (which spawns `AnimClass` objects for debris/explosion) and then calls `vtable+0xF8`. There is no `InfantryClass__Constructor` or `UnitClass__Constructor` call for a husk entity. The spawned AnimClass and VoxelAnim objects are logic objects appended to the tail of the active vector (as per `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md` §3.4: "Newly appended tail entries can be reached in the same pass"), but they are cosmetic — not the original vehicle.

**Active in YR: Yes (no alive husk is spawned for ground vehicles).** The original vehicle entity is unregistered synchronously. Debris anims may be appended to the tail and could run in the same pass if the loop index has not advanced past the tail.

## 4. Bullet Detonation Path Interaction

From `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md` §3.1: `BulletClass::AI` calls `BulletDetonation` then `vtable+0xF8` (self-unregister). `BulletDetonation` calls `WarheadTypeClass__Detonate` → `apply_area_damage` (§5, slot 5 of this swarm, `TARGETDEATH_APPLY_AREA_DAMAGE_LIVE_VECTOR_ITERATION_RESWARM_20260528.md`) → `vtable+0x16C` on each victim → `UnitClass::ReceiveDamage` → case 4 → `vtable+0xF8` on the killed vehicle.

**The vehicle is removed DURING the bullet's `BulletClass::AI` execution, before the bullet itself self-unregisters.** The bullet then calls its own `vtable+0xF8`. Both removals happen within the same `BulletClass::AI` call, which is a single `vtable+0x5C` call from the PerTickUpdate scheduler. Any object shifted into the bullet's former slot AND any object shifted into the killed vehicle's former slot can be skipped this pass. **Active in YR: Yes for ordinary projectile-killed vehicles.**

## 5. Rust Delta

| gamemd behavior | Rust current behavior | Evidence | Status |
|---|---|---|---|
| Killed vehicle calls `vtable+0xF8` synchronously inside `ReceiveDamage`, removing it from the active vector before the attacker's AI turn returns. | Rust collects dead entity IDs in `dead_entities: Vec<u64>` during `tick_combat_with_fog`, then calls `handle_entity_deaths` in Phase 6 which calls `entities.remove(dead_id)` — a batch AFTER all damage for the tick is processed. | `src/sim/combat/mod.rs:2201` (`handle_entity_deaths` call); `:1006` (`entities.remove(dead_id)`); `:2200` comment "Phase 6: handle death effects". | **DRIFT**: Rust defers removal to a post-damage batch; native gamemd removes synchronously inside `ReceiveDamage` during the active pass. |
| Debris AnimClass objects spawned by `Death_Explosion` are appended to the active-vector tail and may run in the same pass. | Rust spawns explosion effects as `ExplosionEffect` structs returned from combat, processed by the app layer — not as live sim entities in the same pass. | `src/sim/combat/mod.rs:2218` (`explosion_effects.extend(death.explosion_effects)`). | **DRIFT**: Rust explosion effects are app-layer and do not participate in the sim-tick active-object pass. |
| If `Crashable=yes` (jumpjets), entity may remain in active vector temporarily. | Rust has no `Crashable` field or crash-sequence state for vehicles — all non-animation vehicle deaths are immediate `entities.remove`. | `src/sim/combat/mod.rs:1000` (`// Structures and voxel vehicles: immediate despawn`). | **DRIFT for Crashable=yes, but Crashable=no ground vehicles match in terms of "entity is logically dead" — just at wrong timing.** |

**Core timing difference (player-visible consequence):** In gamemd, a vehicle killed during tick N leaves the active vector during tick N's object pass. In Rust, the vehicle is removed at the end of the combat phase of tick N, but if any system in the same tick reads from the entity store between damage application and `handle_entity_deaths`, it can see a dead vehicle still present. More critically, any Rust system that runs AFTER `handle_entity_deaths` but within the same tick finds the vehicle gone — which matches gamemd's within-tick ordering. The main player-visible ordering risk is that in gamemd, objects shifted by the removal can be SKIPPED this pass; Rust's snapshot/keyed approach cannot produce this skip.

## 6. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::ReceiveDamage @ 0x00737C90` | Full UnitClass damage + case-4 death handler | Decompiled in this session | Yes |
| `ObjectClass::UnInit @ 0x005F65F0` | Entry point for synchronous active-vector removal + PendingDeleteList queue | Decompiled in this session | Yes |
| `FUN_0055BAE0` | Compacts active vector LEFT, clears `Object+0x98` | `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md` §2 | Yes |
| `UnitClass__Death_Explosion @ 0x00738680` | Spawns debris/explosion AnimClass BEFORE UnInit | Decompiled in this session | Yes (for ground vehicles with `DeathWeaponAnims`) |
| `ObjectClass::Conceal @ 0x005F4D30` | Called from Limbo/vtable+0xD4, calls FUN_0055BAE0 | `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md` §2 | Yes |
| `handle_entity_deaths @ src/sim/combat/mod.rs:804` | Rust batch death handler (deferred, Phase 6 of combat tick) | Code read in this session | N/A (Rust) |

## 7. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Vehicle killed inside a bullet's AI turn is removed from the active vector DURING that bullet's `vtable+0x5C` call; objects shifted into the dead vehicle's old slot can be skipped this pass. | Rust defers removal to `handle_entity_deaths` Phase 6 — all deaths in a tick are batched; no mid-pass vector compaction occurs. | `src/sim/combat/mod.rs::handle_entity_deaths`; future sim-level active-object scheduler. | Vector `[bullet, vehicle_A, vehicle_B]` where bullet detonates killing vehicle_A; in native gamemd vehicle_A removes synchronously during bullet's AI, shifting vehicle_B into vehicle_A's slot; if bullet was at index 0, vehicle_B (now at index 1) is skipped until next pass. | `vehicle_killed_by_bullet_shifts_successor_skipped_same_pass` | Do NOT model this as a separate "dead entity queue flushed at tick end" — the removal is synchronous to the damage call. |
| Ground vehicle death spawns AnimClass debris at tail of active vector (via `Death_Explosion`), before the vehicle itself is unregistered. Newly appended anims CAN run in the same pass if the scheduler index has not yet reached the tail. | Rust spawns explosion effects as app-layer `ExplosionEffect` structs, not as live sim entities in the current pass. | `src/sim/combat/mod.rs:2218`, future `AnimClass`-equivalent logic-vector particle. | Tank at index 5 (out of 10) dies; `Death_Explosion` appends 2 debris anims at tail (now indices 10 and 11); scheduler at index 5+1=6 continues and reaches indices 10 and 11 this pass — both debris anims run this tick. | `vehicle_death_debris_anim_appended_tail_runs_same_pass` | Do NOT use app-layer render effects as proxies for logic-vector debris anims when tick ordering of debris AI matters. |
| `Crashable=yes` vehicles (jumpjets, certain aircraft-like units) do NOT call `vtable+0xF8` when their crash-state initiator returns non-zero; they remain in the active vector for the crash sequence. | Rust performs immediate `entities.remove` for all non-animation vehicles — no crash-state intermediate. | `src/sim/combat/mod.rs:1000`. | Jumpjet killed; in native, the entity persists for crash sequence ticks; in Rust it is immediately removed. | `crashable_vehicle_remains_in_active_vector_during_crash_sequence` | Do not apply the Crashable intermediate path to non-Crashable ground vehicles; standard ground vehicle removal is synchronous and immediate. |

## 8. Negative Facts / Do Not Do

- Do not model ground vehicle active-vector removal as deferred to a post-tick cleanup. Evidence: `vtable+0xF8` is called SYNCHRONOUSLY inside `UnitClass::ReceiveDamage` case 4, before the function returns. (`0x00737C90` decompile final block; `ObjectClass::UnInit @ 0x005F65F0` compacts vector immediately via `FUN_0055BAE0`).
- Do not spawn an alive husk UnitClass entity registered in the logic vector for standard ground vehicle death. Evidence: `UnitClass__Death_Explosion` spawns AnimClass only; no `UnitClass__Constructor` or `ObjectClass::Reveal` for a husk is present in the case-4 branch. (`SUBMARINE_AND_SINKING_GHIDRA_REPORT.md` §B.8 confirms "entity is deleted the same frame").
- Do not treat the 5-byte thunk at `0x00740E50` (UnitClass vtable+0xE0, labelled `UnitClass__ReceiveDamage`) as the full case-4 death handler. The authoritative UnitClass ReceiveDamage override is `0x00737C90` at `vtable+0x16C`. Evidence: `read_memory 0x007F5DDC` → `0x00737C90`; `0x00740E50` is a 5-byte thunk to `FUN_00708C30`.
- Do not apply the Crashable (`type+0xD95 != 0`) deferred-removal path to standard ground vehicles. Tanks, miners, war miners, terror drones, etc. have `Crashable=false`. Only jumpjet-style units use that flag. Evidence: `CARGO_CLEAR_ALL_IN_OPEN_TRANSPORT_DAMAGE_TIMING_RESWARM_20260528.md` confirms `type+0xD95 = Crashable`.
- Do not claim the `apply_area_damage` snapshot (slot 5's finding) protects the outer PerTickUpdate cursor. Evidence: slot 5 report (`TARGETDEATH_APPLY_AREA_DAMAGE_LIVE_VECTOR_ITERATION_RESWARM_20260528.md`) explicitly states "outer PerTickUpdate cursor is NOT protected by this snapshot".

## 9. Remaining Uncertainty

- `Crashable=yes` crash-state vtable+0x3DC is NOT fully traced. If it returns non-zero, the entity lingers in the active vector. What crash sequence keeps it alive and when does the deferred removal eventually happen? This requires a separate targeted Ghidra pass on the jumpjet/aircraft crash state machine. Not needed for standard ground vehicle parity.
- The exact object shifted into the dead vehicle's former index depends on runtime vector state. Static Ghidra proves the mechanism; concrete index numbers require a runtime debugger trace on a live scenario.
- `FUN_004DE710` (passenger/crew eject helper) is not fully decompiled here. It is relevant to OpenTopped transport vehicle death specifically. Its spawn path (reveals a new entity → appends to active-vector tail?) is not confirmed in this session. Standard non-transport vehicle deaths are unaffected.

## Sources

- Ghidra read-only decompiled in this session:
  - `UnitClass::ReceiveDamage @ 0x00737C90`
  - `ObjectClass::UnInit @ 0x005F65F0`
  - `UnitClass__Death_Explosion @ 0x00738680`
  - `0x00740E50` (5-byte thunk — identity check)
  - `FUN_004DE710 @ 0x004DE710` (partial)
- Ghidra read-only memory reads in this session:
  - `read_memory 0x007F5DDC` (4 bytes) → `0x00737C90` — UnitClass vtable+0x16C ReceiveDamage override
- Prior reports cross-referenced (not re-derived):
  - `docs/research/COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md` — scheduler mechanics, FUN_0055BAE0, Conceal/UnInit
  - `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md` — live count reload, append contract
  - `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md` — tail-append same-pass eligibility
  - `docs/research/SUBMARINE_AND_SINKING_GHIDRA_REPORT.md` §B.8 — ground unit death comparison, no-husk finding
  - `docs/research/TECHNOCLASS_VTABLE_COMPLETE.md` — slot 91 = ReceiveDamage at +0x16C
  - `docs/research/UNITCLASS_GHIDRA_REPORT.md` — vtable method map, INI key offsets
  - `docs/research/CARGO_CLEAR_ALL_IN_OPEN_TRANSPORT_DAMAGE_TIMING_RESWARM_20260528.md` — `type+0xD95 = Crashable`
  - `docs/research/CRATE_SYSTEM_GHIDRA_REPORT.md` — `type+0xE1A = CarriesCrate`
  - `docs/research/TARGETDEATH_APPLY_AREA_DAMAGE_LIVE_VECTOR_ITERATION_RESWARM_20260528.md` — slot 5: apply_area_damage snapshot scope
- Rust source scanned:
  - `src/sim/combat/mod.rs` (handle_entity_deaths, Phase 6 batch removal)
  - `src/sim/world/mod.rs` (tick_combat_with_fog call site)

Status: COMPLETE for standard ground vehicle removal timing, no-alive-husk finding, and Rust delta. Crashable=yes deferred-removal path is bounded as Remaining Uncertainty.
