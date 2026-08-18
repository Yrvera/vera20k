# Wall Damage RNG Classification - Ghidra Research Report

**Address(es):** `0x00480CB0` primary, `0x00480D0C` RNG call, `0x004896AD` area-damage caller, `0x0075F477` direct-hit caller, `0x0073B056` forced crusher caller  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** wall/overlay damage RNG gate in `CellClass::DestroyOverlay` versus current Rust `src/sim/overlay_grid.rs::damage_wall_recursive`  
**Non-Scope:** full wall connection rebuild, bridge damage RNG, ore/tiberium destruction, projectile collision, and all non-wall overlay destruction paths  
**Confidence:** High for the RNG gate and Rust mismatch; Medium for broader caller taxonomy because only immediate callers were checked  
**Active in YR:** Yes

## Working Notes Required Before Investigation

Target question: Is current Rust `rng.next_range_u32(flags.strength)` correct for wall damage, or is gamemd using inclusive `RandomRanged(0, Strength)`?

Non-goals: Do not re-audit the global RNG helper, do not patch Rust, do not re-investigate all wall connectivity/destruction behavior.

Evidence needed to mark COMPLETE: decompile plus assembly context for the wall damage gate, proof of active YR caller path, Rust call-site scan, and implementation handoff with concrete tests.

Stop conditions: Stop after the exact wall RNG gate, comparison, bypass conditions, and current Rust delta are classified; defer any wider caller identity or non-RNG wall behavior.

## 1. Overview

`CellClass::DestroyOverlay` is the live wall-overlay damage stage function. For non-forced damage below overlay `Strength`, gamemd calls `Random::RandomRanged(0, Strength)` and advances the wall only when the returned roll is **strictly less than** the damage value.

Current Rust is RED for this call site. It uses `next_range_u32(strength)`, which is exclusive `RandomRanged(0, Strength - 1)`, and then advances when `roll <= damage` because it returns only when `roll > damage`.

## 2. Class Layout / Key Offsets

| Offset | Owner | Meaning | Evidence | Active in YR |
|---|---|---|---|---|
| `CellClass+0x44` | cell | `OverlayTypeIndex`; `-1` means no overlay | `0x00480CB9` load, `0x00480CBC` compare | Yes |
| `CellClass+0x11E` | cell | overlay data byte; upper nibble is wall damage stage | `0x00480BB7..0x00480BBD`, decompile write `+0x10` | Yes |
| `OverlayTypeClass+0x2A8` | overlay type | `Wall=yes` flag | `0x00480CD2..0x00480CDA` | Yes |
| `OverlayTypeClass+0x2A4` | overlay type | `Strength=` random upper bound | `0x00480CE9`, `0x00480D03` | Yes |
| `OverlayTypeClass+0x2A0` | overlay type | `DamageLevels=` stage count | prior `WALL_DAMAGE_STAGE_INCREMENTER_GHIDRA_REPORT.md` plus same function decompile | Yes |
| `ScenarioClass+0x218` | scenario | gamemd random object used by `RandomRanged` | `0x00480CFD..0x00480D0C` | Yes |

## 3. Core Logic

Verified binary gate in `CellClass::DestroyOverlay`:

```text
if OverlayTypeIndex == -1: return 0
if overlay.Wall == false: return 0

if damage != -1:
    if damage < overlay.Strength:
        if MapEditorMode == 0:
            roll = RandomRanged(0, overlay.Strength)
            if roll >= damage: return 0

advance overlay damage byte by 0x10
```

Load-bearing assembly:

| Address | Instruction / effect | Meaning |
|---|---|---|
| `0x00480CE4` | `CMP ESI,-0x1`; `JZ 0x00480D1E` | `damage == -1` bypasses RNG |
| `0x00480CE9` | `MOV EAX,[EBP+0x2A4]` | load overlay `Strength` |
| `0x00480CEF` | `CMP ESI,EAX`; `JGE 0x00480D1E` | `damage >= Strength` bypasses RNG and advances |
| `0x00480CF3..0x00480CFB` | test `g_MapEditorMode`; jump if nonzero | map editor mode bypasses RNG |
| `0x00480D03` | `PUSH EAX`; `0x00480D04 PUSH 0x0` | pass `high=Strength`, `low=0` |
| `0x00480D06` | `LEA ECX,[Scenario+0x218]` | use scenario RNG |
| `0x00480D0C` | `CALL 0x0065C7E0` | call `Random::RandomRanged` |
| `0x00480D11` | `CMP EAX,ESI`; `0x00480D13 SETL AL` | true only when `roll < damage` |
| `0x00480D18` | `JZ 0x00481093` | if not `roll < damage`, return no damage |

Because `RandomRanged(0, Strength)` is inclusive per `RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md`, the non-forced advance probability for `0 <= damage < Strength` is:

```text
P(advance) = damage / (Strength + 1)
```

Examples:

| Damage | Strength | Binary advancing rolls | Probability |
|---:|---:|---:|---:|
| `0` | `400` | none | `0/401` |
| `1` | `400` | `{0}` | `1/401` |
| `100` | `400` | `0..99` | `100/401` |
| `399` | `400` | `0..398` | `399/401` |
| `400` | `400` | no RNG gate | guaranteed |

## 4. INI Keys

| Key | Owner | Default / retail example | Effect | Evidence | Active in YR |
|---|---|---|---|---|---|
| `Wall=yes` | overlay/rules type | `GAWALL`, `NAWALL`, `CAFNC*` set it in retail INI | required before wall damage gate runs | `0x00480CD2..0x00480CDA`; `ini/rulesmd.ini` wall sections | Yes |
| `Strength=` | overlay/rules type | `GAWALL Strength=300`, `NAWALL Strength=300`; tests use 400 | inclusive upper bound for RNG gate | `0x00480CE9`, `0x00480D03`; `src/map/overlay_types.rs` parser | Yes |
| `DamageLevels=` | art overlay section | wall art controls stage count | not RNG, but determines post-gate stage thresholds | prior wall reports and parser scan | Yes |
| `Wall=` | warhead | many combat warheads set `Wall=yes` | area damage may call `DestroyOverlay` | `Apply_area_damage @ 0x004896AD` caller context | Yes |
| `WallAbsoluteDestroyer=` | warhead | set on some special warheads | same call path into `DestroyOverlay` with damage value in area damage | `Apply_area_damage @ 0x004896AD`; prior warhead docs | Yes |
| `Wood=` | warhead | fire/wood-affecting warheads | gates wood-material wall overlays | `Apply_area_damage @ 0x00489693..0x0048969D` | Yes |

## 5. Integration Points

Verified immediate callers of `CellClass::DestroyOverlay @ 0x00480CB0`:

| Caller | Evidence | Damage argument | Active in YR |
|---|---|---|---|
| `Apply_area_damage @ 0x00489280`, callsite `0x004896AD` | decompile and assembly context | area damage value | Yes, standard warhead area/cell damage path |
| `FUN_0075F330 @ 0x0075F330`, callsite `0x0075F477` | decompile and assembly context | weapon damage from `+0x98` | Yes/Conditional, direct weapon-hit path with non-null weapon object |
| `UnitClass::PerCellProcess @ 0x00739EC0`, callsite `0x0073B056` | decompile and assembly context | `-1` forced destroy | Conditional, crushing/engineer-style overlay contact |
| self-recursion from `CellClass::DestroyOverlay` | caller list | `0xC8` chain damage | Yes, concrete-wall chain reaction after stage threshold |
| `BuildingClass::Limbo @ 0x00445880` | caller list only | not rechecked | Conditional; deferred because not RNG classification critical |

## 6. Current Rust Implementation Status

Current Rust surface:

| File / function | Current behavior | Classification |
|---|---|---|
| `src/sim/overlay_grid.rs::damage_wall_recursive` | `if damage != u16::MAX && strength > 0 && damage < strength { roll = rng.next_range_u32(strength); if roll > damage { return; } }` | RED |
| `src/sim/rng.rs::next_range_u32` | wrapper for exclusive `0..max_exclusive`, implemented via inclusive `RandomRanged(0, max-1)` | correct helper, wrong call site for this binary contract |
| `src/sim/world/mod.rs::apply_wall_damage_events` | feeds combat wall events through `damage_wall_overlay` using sim RNG | affected deterministic surface |
| `src/sim/combat/combat_tests.rs` wall tests | deterministic replay and forced-destroy coverage, but no boundary test for `damage=0`, `damage=1`, or `damage=strength-1` | missing tests |

Current Rust versus binary for `damage < strength`:

| Case | Binary | Current Rust |
|---|---|---|
| Range | `RandomRanged(0, Strength)` inclusive | `next_range_u32(Strength)` = `RandomRanged(0, Strength-1)` |
| Advance condition | `roll < damage` | `roll <= damage` |
| No-op condition | `roll >= damage` | `roll > damage` |
| `damage=0` | never advances | advances when roll is 0 |
| `damage=Strength-1` | may still fail on rolls `Strength-1` or `Strength` | always advances |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CellClass::DestroyOverlay` RNG gate | verified | decompile `0x00480CB0`; assembly `0x00480CE4..0x00480D18` | none for gate |
| Inclusive bound `RandomRanged(0, Strength)` | verified | `PUSH Strength`, `PUSH 0`, call `0x0065C7E0`; RNG report | none |
| Strict comparison `roll < damage` | verified | `0x00480D11 CMP EAX,ESI`; `0x00480D13 SETL AL`; `0x00480D18 JZ return` | none |
| `damage == -1` bypass | verified | `0x00480CE4..0x00480CE7` | Rust sentinel is `u16::MAX`, implementation mapping is out-of-scope but appears intentional |
| `damage >= Strength` bypass | verified | `0x00480CEF..0x00480CF1` | none |
| `MapEditorMode` bypass | verified | `0x00480CF3..0x00480CFB` | current Rust has no map-editor mode branch; likely out-of-scope for runtime sim |
| Area-damage wall caller | verified | `Apply_area_damage` decompile; callsite `0x004896AD` | broader damage value derivation deferred |
| Direct weapon-hit caller | touched-not-exhausted | `FUN_0075F330` decompile; callsite `0x0075F477` | exact gameplay owner/caller chain deferred |
| Forced crusher/contact caller | touched-not-exhausted | `UnitClass::PerCellProcess`; callsite `0x0073B056` | full crusher ability gate deferred |
| Building limbo caller | deferred | caller list names `0x00445880` | not needed for RNG classification |
| Current Rust call site | verified | `src/sim/overlay_grid.rs:357..360` scan | implementer should patch separately |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Is `CellClass::DestroyOverlay` live in YR? -> Yes, area damage and standard wall overlays call it.` (evidence: `Apply_area_damage @ 0x004896AD`; `rulesmd.ini` wall sections)
- `[RESOLVED] OQ2 - Does the wall gate call `RandomRanged(0, Strength)` or `RandomRanged(0, Strength-1)`? -> It calls `RandomRanged(0, Strength)` inclusive.` (evidence: `0x00480D03 PUSH Strength`; `0x00480D04 PUSH 0`; `0x00480D0C CALL 0x0065C7E0`; RNG report)
- `[RESOLVED] OQ3 - Is the comparison `roll > damage`, `roll >= damage`, or another form? -> The damage advances only when `roll < damage`; no-op is `roll >= damage`.` (evidence: `0x00480D11 CMP EAX,ESI`; `0x00480D13 SETL AL`; `0x00480D18 JZ return`)
- `[RESOLVED] OQ4 - Does equal damage/strength consume RNG? -> No, `damage >= Strength` jumps past the RNG gate.` (evidence: `0x00480CEF CMP ESI,EAX`; `0x00480CF1 JGE 0x00480D1E`)
- `[RESOLVED] OQ5 - Does forced destruction consume RNG? -> No, `damage == -1` jumps past the RNG gate.` (evidence: `0x00480CE4 CMP ESI,-1`; `0x00480CE7 JZ 0x00480D1E`)
- `[RESOLVED] OQ6 - Does zero damage ever advance through the RNG gate? -> No for normal runtime, because `roll < 0` is impossible for inclusive nonnegative roll.` (evidence: `0x00480D11..0x00480D18`)
- `[RESOLVED] OQ7 - Is current Rust exclusive `next_range_u32(strength)` correct? -> No.` (evidence: `src/sim/overlay_grid.rs:359`; `src/sim/rng.rs` settled wrapper contract)
- `[RESOLVED] OQ8 - Is current Rust comparison correct if range is changed? -> No, it must not advance on `roll == damage`.` (evidence: `SETL` at `0x00480D13`)
- `[RESOLVED] OQ9 - Does area damage reach this code in YR? -> Yes, wall/wood/absolute warhead flags gate a call to `DestroyOverlay`.` (evidence: `Apply_area_damage @ 0x00489687..0x004896AD`)
- `[RESOLVED] OQ10 - Are warhead flags themselves part of this RNG mismatch? -> No, they decide whether to call the wall function, not the roll range once inside.` (evidence: `0x00489687..0x004896AD`, then `0x00480CB0` gate)
- `[RESOLVED] OQ11 - Does chain reaction use same RNG gate? -> Yes, recursion passes damage `0xC8`; it then enters the same `DestroyOverlay` gate unless bypassed by damage >= Strength.` (evidence: `CellClass::DestroyOverlay` decompile; self caller list)
- `[RESOLVED] OQ12 - Does current Rust have tests for the exact boundary? -> No, wall tests cover forced destruction, deterministic replay, and damage=strength skip but not low-bound or strength-minus-one RNG cases.` (evidence: `src/sim/combat/combat_tests.rs` scan)
- `[DEFERRED] OQ13 - What exact higher-level caller invokes `FUN_0075F330`?` (category: out-of-scope; reason: direct-hit caller identity does not change the verified wall RNG gate; next-step-if-pursued: investigate weapon impact pipeline)
- `[DEFERRED] OQ14 - What exact conditions gate `BuildingClass::Limbo` wall destruction?` (category: out-of-scope; reason: caller not needed for current RNG classification; next-step-if-pursued: decompile `0x00445880` slice)
- `[DEFERRED] OQ15 - Should Rust model map-editor mode bypass?` (category: requires-different-system-context; reason: runtime sim has no editor-mode surface in this slice; next-step-if-pursued: classify map editor vs runtime build mode)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| For `0 <= damage < Strength`, gamemd draws `RandomRanged(0, Strength)` and advances only when `roll < damage` | `0x00480D03..0x00480D18`; RNG report | mismatch | `src/sim/overlay_grid.rs::damage_wall_recursive` | use inclusive wall-strength roll and strict less-than advance condition | deterministic fake/seeded RNG where first wall roll equals `damage` must leave wall unchanged | Do not use `roll <= damage`; that preserves the stale doc bug |
| `damage == -1` forced destruction bypasses RNG | `0x00480CE4..0x00480CE7`; crusher caller `0x0073B056 PUSH -1` | equivalent sentinel appears to be `u16::MAX`; no RNG draw should occur | `damage_wall_overlay`, `WallDamageEvent` producer paths | keep forced-destroy no-roll behavior | forced wall destruction should leave RNG state unchanged while clearing overlay/entity | Do not route forced destroy through `RandomRanged` |
| `damage >= Strength` bypasses RNG and advances | `0x00480CEF..0x00480CF1` | current Rust already skips the roll here | `damage_wall_recursive` | preserve no-draw behavior at equality and above | `damage == strength` should advance stage and keep RNG state unchanged | Do not convert to unconditional inclusive roll for all damage values |

Proposed test names:

- `wall_damage_roll_equal_damage_does_not_advance`
- `wall_damage_zero_never_advances_or_consumes_success`
- `wall_damage_equal_strength_bypasses_rng_and_advances`

## Negative Facts / Do Not Do

- Do not implement the prior-doc formula `RandomRanged(0, Strength) > damage -> no-op`; binary says no-op when `roll >= damage` via `SETL` + `JZ` at `0x00480D13..0x00480D18`.
- Do not call `next_range_u32(strength)` for this gate; that is `0..Strength-1`, while gamemd pushes `Strength` as inclusive high at `0x00480D03`.
- Do not let `damage=0` have a one-roll chance to advance; binary requires `roll < 0`, impossible for `RandomRanged(0, Strength)`.
- Do not consume RNG for `damage == Strength`; binary `JGE` bypasses the RNG call.
- Do not treat `WallAbsoluteDestroyer` as necessarily passing `-1` in area damage; observed area-damage caller gates the call but passes the damage value into `DestroyOverlay`.

## Stale Docs / Follow-up Docs

Replace this wording in `WALL_DAMAGE_STAGE_INCREMENTER_GHIDRA_REPORT.md` and dependent plans:

```text
if (Random::RandomRanged(0, Strength) > damage) return 0;
Probability of stage advance per tick = damage/Strength.
```

with:

```text
roll = Random::RandomRanged(0, Strength);
if (roll >= damage) return 0;
For 0 <= damage < Strength, probability of stage advance is damage / (Strength + 1). damage == 0 never advances through the normal RNG gate; damage >= Strength bypasses the roll and advances.
```

## Sources

- Ghidra decompile: `CellClass::DestroyOverlay @ 0x00480CB0`
- Ghidra assembly context: `0x00480CE4..0x00480D18`
- Ghidra decompile: `Apply_area_damage @ 0x00489280`
- Ghidra assembly context: callsite `0x004896AD`
- Ghidra decompile: `FUN_0075F330 @ 0x0075F330`
- Ghidra assembly context: callsite `0x0075F477`
- Ghidra decompile: `UnitClass::PerCellProcess @ 0x00739EC0`
- Ghidra assembly context: callsite `0x0073B056`
- `C:/Users/enok/Documents/ra2-rust-game-docs/RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/WALL_DAMAGE_STAGE_INCREMENTER_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/WALL_CONNECTION_AND_DESTRUCTION_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/overlay_grid.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/combat/combat_tests.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs`
