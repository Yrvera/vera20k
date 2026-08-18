# BulletClass Detonation Same-Pass Child Spawn Order — Ghidra Research Report

**Date:** 2026-05-28
**Slot:** 3 of /re-swarm batch
**Investigation Mode:** exhaustive-slice
**Address(es):** `BulletClass::AI @ 0x004666E0`; `BulletClass::BulletDetonation (BulletClassBulletDetonationImpactDamage) @ 0x00468D80`; `WarheadTypeClass::Detonate @ 0x004690B0`; `AnimClass::Constructor @ 0x00421EA0`; `VoxelAnimClass::Constructor @ 0x007493B0`; `ObjectClass::Reveal @ 0x005F4EC0`; `FUN_0055BAA0 @ 0x0055BAA0`; `LogicClass::PerTickUpdate @ 0x0055AFB0`; `FUN_0055BAE0 @ 0x0055BAE0`

## 0. Scope Gate

**Target question:** When `BulletClass::AI` detonates, (a) do explosion/debris/sub-bullet children get appended to the live LogicClass active vector (`0x87F778`) **before** or **after** the bullet dispatches `vtable+0xF8` self-removal, and (b) are those children same-pass eligible for `vtable+0x5C` this tick under the live-count-reload rule?

**Non-goals:** Deriving spawn mechanics (settled by ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md, AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md, VOXELANIMCLASS_GHIDRA_REPORT.md). Measuring runtime vector indices (needs debugger). The nuke-flash anim listener mechanism (noted below; owned by slot 5).

**Evidence needed to mark COMPLETE:** Assembly-verified call sequence in `BulletClass::AI`'s detonation tail and in `BulletClassBulletDetonationImpactDamage`, proving whether all child appends complete before or after the `vtable+0xF8` dispatch; scheduler count-reload site confirming same-pass tail eligibility; Rust surface comparison.

**Stop conditions:** Stop when detonation call-sequence order is settled; stop if a path requires draining a full warhead damage subsystem.

## 1. Overview

The assembly of `BulletClass::AI`'s detonation tail is unambiguous: **all child spawning happens before** `vtable+0xF8`. The normal-detonation path calls `BulletClassBulletDetonationImpactDamage` at `0x00468D80` first, and only then dispatches `(*(*EBP + 0xF8))()`. `WarheadTypeClass::Detonate` — which spawns the explosion `AnimClass`, `VoxelAnimClass` debris, and airburst sub-bullets — is called entirely *inside* `BulletDetonationImpactDamage`, which completes before `AI` issues `vtable+0xF8`. Because those children are logic-enabled objects appended to the tail of the same live vector, and the scheduler reloads live count after every `vtable+0x5C` call, they are same-pass eligible if the scheduler cursor has not yet passed the new tail.

The delayed-nuke path (nuke-flash anim listener) defers and is slot-5 territory.

## 2. Verified Findings

### 2.1 Detonation Tail Call Order in `BulletClass::AI`

Active in YR: Yes.

From assembly context at `0x00467f9b..0x00467fb4`:

```
00467f9b: MOV EDX, [ESP+0x18]       ; saved detonation coords
00467f9f: MOV ECX, EBP              ; this = bullet
00467fa1: PUSH EDX
00467fa2: CALL 0x00468D80           ; BulletClassBulletDetonationImpactDamage
00467fa7: JMP 0x00467faf
; ... (alternate path for OOB case merges here at 0x00467faf)
00467faf: MOV EAX, [EBP]            ; load vtable
00467fb2: MOV ECX, EBP              ; this = bullet
00467fb4: CALL dword ptr [EAX+0xF8] ; self-remove / UnInit
```

**The CALL to 0x00468D80 is at 0x00467fa2, which precedes the CALL [EAX+0xF8] at 0x00467fb4 in linear address order with no branch escaping between them.** `BulletDetonationImpactDamage` fully returns before `vtable+0xF8` is invoked.

Evidence: Assembly context `0x00467f9b..0x00467fb4` — verified this session.

There is also an alternate path (out-of-bounds detonation from `CALL dword ptr [EDX+0x124]` at `0x00467fa9`) that merges at `0x00467faf` before the same `vtable+0xF8` call. Both paths invoke self-removal after detonation completes. Active in YR: Yes.

### 2.2 All Child Spawning Happens Inside `BulletDetonationImpactDamage` (before vtable+0xF8)

Active in YR: Yes, conditional on bullet type flags.

`BulletClassBulletDetonationImpactDamage @ 0x00468D80` contains:

**Airburst=no, Cluster>0 path:**
```c
while (WarheadTypeClass::Detonate(), this->IsAlive) {
    // scatter, loop up to Cluster times
}
return;
```

**Airburst=yes path:**
```c
WarheadTypeClass::Detonate();   // single call; sub-bullets + anims spawn inside
```
Evidence: Decompile `0x00468D80` — verified this session.

`WarheadTypeClass::Detonate @ 0x004690B0` contains all three child-spawn sites:

1. **Explosion AnimClass:** `operator_new(0x1C8)` + `AnimClass__Constructor(type, coords, 0, 1, 0x2600, ...)` — spawns explosion anim before returning from Detonate. Evidence: Decompile `0x004690B0`, call at `~0x00469C90`.

2. **VoxelAnimClass debris:** `operator_new(0x148)` + `VoxelAnimClass__Constructor(type, coords, ...)` in a `while` loop. Evidence: Decompile `0x004690B0`, loop at `~0x0046A060..0x0046A090`.

3. **Airburst sub-bullets:** `CoCreateInstance` + `BulletClass__Init` + `(*vtable+0x1F0)(coords, velocity)` which is `BulletClass::Fire` — 8-direction loop + 9th at impact. Evidence: Decompile `0x004690B0` airburst block `~0x00469E90..0x0046A303`; confirmed by AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md.

All three complete before `WarheadTypeClass::Detonate` returns to `BulletDetonationImpactDamage`, which returns to `BulletClass::AI`, which then issues `vtable+0xF8`.

**Total order within one BulletClass::AI detonation tick:**
1. `BulletClassBulletDetonationImpactDamage` called (at `0x00467fa2`)
2. Inside: `WarheadTypeClass::Detonate` → explosion AnimClass appended to live vector tail
3. Inside: VoxelAnimClass debris appended to live vector tail (if warhead has debris)
4. Inside: airburst sub-bullets created via `BulletClass::Fire` → each calls `ObjectClass::Reveal` → each appended to live vector tail (if logic-enabled)
5. `BulletDetonationImpactDamage` returns
6. `vtable+0xF8` called on bullet (at `0x00467fb4`) → `ObjectClass::UnInit` → `Conceal` → `FUN_0055BAE0` compacts the bullet out of the live vector

### 2.3 Child Logic Registration via ObjectClass::Reveal → FUN_0055BAA0

Active in YR: Yes.

Each child class that is logic-enabled (`ObjectTypeClass+0x234 = 1`) calls `ObjectClass::Reveal @ 0x005F4EC0` during its constructor, which reaches:

```
0x005F5038: MOV ECX, 0x87F778  ; LogicClass singleton
0x005F503D: CALL 0x0055BAA0    ; FUN_0055BAA0 — append to active vector
```

`FUN_0055BAA0` appends to the tail of `LogicClass.items` (at `+0x04`) and increments `LogicClass.count` (at `+0x10`). Evidence: ANIMCLASS_GLOBAL_REGISTRATION_SAMEPASS_SCHEDULER_GHIDRA_REPORT.md assembly context `0x005F5038..0x005F5040`, confirmed `FUN_0055BAA0` tail-append behavior at `0x0055BAA5..0x0055BAC6`.

**Specific child eligibility:**
- **Explosion AnimClass:** `AnimClass::Constructor` calls `ObjectClass::Reveal` for normal types → appended to tail. Same-pass eligible under cursor rule. Evidence: ANIMCLASS_GLOBAL_REGISTRATION_SAMEPASS_SCHEDULER_GHIDRA_REPORT.md.
- **VoxelAnimClass debris:** `VoxelAnimClass::Constructor @ 0x007493B0` calls `ObjectClass::Reveal` → appended. Same-pass eligible. Evidence: VOXELANIMCLASS_GHIDRA_REPORT.md §6.
- **Airburst sub-bullets:** `BulletClass::Fire @ 0x00468670` calls `ObjectClass::Reveal` → appended. `BulletTypeClass` sets `+0x234 = 1` in constructor. Same-pass eligible. Evidence: AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md §2.

### 2.4 Same-Pass Eligibility Under the Count-Reload Rule

Active in YR: Yes.

`LogicClass::PerTickUpdate` at `0x0055B613` reloads `LogicClass+0x10` live count after every `vtable+0x5C` call. A child appended at step 2–4 above is at the tail. When the bullet's AI call returns and the scheduler increments index, the new tail (with children) can be reached if the bullet was not the last item. Evidence: LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md, count reload at `0x0055B613`.

**Cursor and compaction interaction:** The bullet at index `i` appends N children to tail (new tail indices `old_count + 0..N-1`), then self-removes via `vtable+0xF8`. `FUN_0055BAE0` shifts `items[i+1..old_count-1]` left by one, and decrements count. The N children were appended **after** `old_count-1`, so they remain at indices `old_count-1..old_count+N-2` after the compaction (the shift only affects items that were before the old tail, not the newly appended ones). Scheduler then increments to `i` (now pointing at the shifted successor, which can be skipped), but the children sit at `old_count-1..old_count+N-2` — ahead of the current cursor — so they are reachable this pass.

Evidence: assembled from `FUN_0055BAE0` compaction at `0x0055BB09..0x0055BB21` (shifts entries from index+1 forward) + tail-append at `FUN_0055BAA0`.

**Caution:** The successor at index `i` (shifted into the bullet's old slot) is skipped per the standard no-index-repair rule. The children at tail are NOT skipped — they remain reachable if cursor < new-count.

### 2.5 Nuke-Flash Anim Listener Path (Different Mechanism — Slot 5)

Active in YR: Conditional on nuke-type warhead.

`BulletClass::AI` has a separate branch (around `0x00467eb1..0x00467f99`) that, instead of calling `BulletDetonationImpactDamage` immediately, creates a nuke-flash AnimClass with `drawFlags=0x2600` (`CALL 0x00421EA0` at `0x00467f2d`), stores the anim pointer at `EBP+0x154` (bullet `+0x154`), sets `EBP+0x158 = 1` (IsInLimbo), and appends `this` (the bullet) to a global AnimClass remove-listener array (`0x00B0F5B8`). The actual `BulletDetonationImpactDamage + vtable+0xF8` is deferred until the listener fires. This is the delayed-nuke case. The explosion anim here is spawned **before** the bullet enters limbo, using `drawFlags=0x2600`. Slot 5 should document the full listener callback and when the listener-triggered detonation fires.

Evidence: Assembly `0x00467eac..0x00467f9b` — verified this session.

## 3. Rust Shape vs. Gamemd

| Mechanism | gamemd behavior | Current Rust |
|---|---|---|
| Explosion AnimClass spawn timing | Spawned inside `BulletDetonationImpactDamage`, appended to live vector tail, before bullet `vtable+0xF8`. | `WorldEffect` retained vector in `src/sim/components.rs:823..923`, advanced by `retain_mut` at end-of-tick in `src/sim/world/mod.rs:1826..1840`. Not a live-vector object; no same-pass AI eligibility. |
| VoxelAnimClass debris spawn timing | Same — inside Detonate, appended to live tail, before bullet UnInit. | Same `WorldEffect` pool or similar retained list. No live-vector membership. |
| Airburst sub-bullets spawn timing | Spawned via `BulletClass::Fire` inside Detonate, each appended to live vector tail via `ObjectClass::Reveal`, before parent bullet UnInit. | Airburst sub-weapon not yet implemented (`src/sim/combat/` has no airburst spawn path). |
| Bullet self-removal order | `BulletDetonationImpactDamage` returns fully → then `vtable+0xF8` removes bullet from live vector. | `homing_movement.rs` snapshots keys and returns detonated IDs; damage/despawn happen via caller-side snapshot, not inside a live-vector AI call. |
| Same-pass child AI | Children at tail are reachable same pass if cursor < new-count. | Not modeled; `WorldEffect` retain_mut and snapshot-based projectile handling prevent same-pass AI of spawned effects. |

## 4. Coverage Ledger

| Area | Status | Evidence |
|---|---|---|
| BulletClass::AI detonation tail call order (normal path) | Verified | `0x00467fa2..0x00467fb4` assembly |
| BulletClass::AI OOB detonation alternate path | Verified | `0x00467fa9..0x00467fb4` assembly |
| BulletDetonationImpactDamage Airburst=yes vs Cluster paths | Verified | Decompile `0x00468D80` |
| WarheadTypeClass::Detonate explosion AnimClass spawn | Verified | Decompile `0x004690B0` |
| WarheadTypeClass::Detonate VoxelAnimClass debris spawn | Verified | Decompile `0x004690B0` |
| WarheadTypeClass::Detonate airburst sub-bullet spawn | Verified | Decompile `0x004690B0`; AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md |
| AnimClass reveal → live vector append | Verified | ANIMCLASS_GLOBAL_REGISTRATION_SAMEPASS_SCHEDULER_GHIDRA_REPORT.md `0x005F5038` |
| VoxelAnimClass reveal → live vector append | Verified | VOXELANIMCLASS_GHIDRA_REPORT.md; ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md |
| Sub-bullet reveal → live vector append | Verified | AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md |
| Count-reload makes tail-appended children same-pass eligible | Verified | LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md `0x0055B613` |
| Nuke-flash anim listener path | Noted/out-of-scope | `0x00467eac..0x00467f99`; deferred to slot 5 |
| Rust current shape | Scanned | `src/sim/components.rs`, `src/sim/world/mod.rs`, `src/sim/movement/homing_movement.rs` |

## 5. Open Questions Log

- `[RESOLVED] OQ-01 — Does BulletDetonationImpactDamage complete before vtable+0xF8? → Yes.` Evidence: `0x00467fa2..0x00467fb4`.
- `[RESOLVED] OQ-02 — Do explosion/debris/sub-bullet appends happen before bullet UnInit? → Yes; all are inside WarheadTypeClass::Detonate, which is called from BulletDetonationImpactDamage.` Evidence: Decompile `0x004690B0`.
- `[RESOLVED] OQ-03 — Are children logic-enabled and thus appended to live vector? → Yes for stock explosion AnimClass, VoxelAnimClass debris, and airburst sub-bullets.` Evidence: Prior registration reports.
- `[RESOLVED] OQ-04 — Are tail-appended children same-pass eligible? → Yes, conditionally on cursor position.` Evidence: `0x0055B613`.
- `[DEFERRED] OQ-05 — What is the exact pre-compaction vs post-compaction runtime vector index for a stock V3 missile strike? → Needs runtime debugger.` Category: runtime-index logging.
- `[DEFERRED] OQ-06 — Does the nuke-flash listener-triggered detonation use the same BulletDetonationImpactDamage → vtable+0xF8 order? → Assembly shows the listener ultimately reaches the same tail; slot 5 should verify the callback path.` Category: out-of-scope-this-slot.

## 6. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Explosion AnimClass and VoxelAnimClass debris are appended to the live logic vector **before** the bullet UnInits at vtable+0xF8. They are same-pass eligible. | Current `WorldEffect` retained vector is not a live-vector object and uses `retain_mut` at end-of-tick — no same-pass AI. | `src/sim/components.rs:823..923`, `src/sim/world/mod.rs:1826..1840`; future generic AnimClass/VoxelAnimClass runtime. | Bullet B at index i detonates. Explosion anim E is appended to tail at old_count. Bullet UnInits, shifting index-i successor. E at tail is reached by scheduler later same pass if cursor < new-count. | `bullet_detonation_explosion_anim_is_same_pass_eligible` | High: delaying explosion anim AI to next tick changes smoke/fire timing and any chained anim children. |
| Airburst sub-bullets are spawned via `BulletClass::Fire` (→ `ObjectClass::Reveal`) inside `WarheadTypeClass::Detonate`, before parent bullet UnInits. Each sub-bullet is appended to the live vector tail and is same-pass eligible. | Airburst sub-bullet spawn not implemented in Rust (`src/sim/combat/` has no airburst path). | Future `src/sim/combat/` airburst spawn, active-object scheduler. | V3 rocket detonates at index i. 9 sub-bullets spawned, each appended to tail. Parent UnInits. Sub-bullets at tail are reached same pass if cursor < new-count. Each sub-bullet's first AI either detonates or begins homing flight this tick. | `airburst_sub_bullets_appended_before_parent_uninit_are_same_pass_eligible` | High: if sub-bullets are queued to next tick they don't detonate at minimal range this tick, changing area damage timing. |
| The bullet's immediate successor in the live vector (shifted into bullet's old slot by compaction) is **skipped** this pass; the appended children at the tail are **not** skipped. | Snapshot-based detonation in `homing_movement.rs` does not model compaction skip. | `src/sim/movement/homing_movement.rs:379..569`; future authoritative live-object projectile path. | Active vector `[A, Bullet, B, ..., C_succ]`. Bullet detonates, appends explosion anim, UnInits. B is shifted to bullet's old slot and skipped this pass. Explosion anim at new tail is reached this pass. | `bullet_detonation_skips_shifted_successor_but_reaches_appended_children` | High: if both successor and children are processed same pass, detonation chain timing drifts. |

## 7. Negative Facts / Do Not Do

- **Do not swap-remove the bullet from the live vector.** The remover (`FUN_0055BAE0 @ 0x0055BAE0`) shifts later entries left and decrements count. Swap-remove changes which successor is skipped. Evidence: `0x0055BB09..0x0055BB21`.
- **Do not process detonation children in a separate post-pass batch and still call them "same tick."** Native children appended before `vtable+0xF8` are appended to the live pass in progress; a post-pass batch changes their AI order relative to all objects that ran after them natively. Evidence: count reload at `0x0055B613` combined with tail-append proof.
- **Do not call vtable+0xF8 (UnInit/self-removal) before BulletDetonationImpactDamage completes.** All child spawns happen inside the detonation call; reversing the order would remove the bullet from the live vector before its children are appended, changing cursor and count. Evidence: `0x00467fa2..0x00467fb4`.
- **Do not model the nuke-flash anim listener path as a normal immediate detonation.** The `BulletClass::AI` limbo path stores the anim, sets IsInLimbo, appends bullet to g_AnimClass_RemoveListeners, and defers detonation — it does not call `BulletDetonationImpactDamage` immediately. Evidence: `0x00467eac..0x00467f99`. Active in YR: Yes (V3 nuke-style warheads in stock content).
- **Do not infer that the Cluster=N path spawns real child objects.** `Cluster=N` (Airburst=no) calls `WarheadTypeClass::Detonate` N times with scatter — it spawns explosion AnimClass per call, but does NOT spawn N new BulletClass instances. Only `Airburst=yes` spawns real sub-bullets. Evidence: `BulletClassBulletDetonationImpactDamage` decompile fork at `if (BulletType[0x294] == 0)`.

## 8. Remaining Uncertainty

- **Concrete runtime vector indices** for a stock V3 strike cannot be proven statically. The static rule is complete; "sub-bullet at tail reaches same-pass AI" requires knowing whether cursor < new-count at the moment the parent detonates, which depends on total live-vector population at that tick. Deferred to runtime debugger.
- **Nuke-flash listener callback order** — when the listener fires and calls the deferred detonation, does it follow the same BulletDetonationImpactDamage → vtable+0xF8 sequence? Assembly suggests yes, but slot 5 should verify.
- **VoxelAnimClass reveals** in some construction paths skip `ObjectClass::Reveal` (the VOXELANIMCLASS_GHIDRA_REPORT.md notes alternate construction paths); the claim that all VoxelAnim debris from WarheadTypeClass::Detonate appends to the live vector should be verified against the specific construction call at `~0x0046A060`.

## Sources

- Fresh read-only Ghidra assembly context: `0x00467b7a..0x00467ff5` (BulletClass::AI detonation tail).
- Fresh read-only Ghidra decompile: `BulletClassBulletDetonationImpactDamage @ 0x00468D80`.
- Fresh read-only Ghidra decompile: `WarheadTypeClass::Detonate @ 0x004690B0`.
- Prior reports (not re-decompiled): `ANIMCLASS_GLOBAL_REGISTRATION_SAMEPASS_SCHEDULER_GHIDRA_REPORT.md`, `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`, `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`, `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`, `AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md`, `VOXELANIMCLASS_GHIDRA_REPORT.md`, `AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md`, `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`.
- Rust static scan: `src/sim/components.rs:823..923`, `src/sim/world/mod.rs:1826..1840`, `src/sim/movement/homing_movement.rs:379..569`, `src/sim/combat/`.

## Status

COMPLETE for the scoped detonation-child append/remove order and same-pass eligibility slice.
