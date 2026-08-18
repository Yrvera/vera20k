# Grizzly Accelerates=false Semantics — Ghidra Research Report

**Address(es):** `0x00715402` (`TechnoTypeClass::ReadINI` key reader), `0x004B0F20` (`DriveLocomotionClass::Process_Drive_Track` consumer), `0x004D3710` (`TechnoClass::SetSpeedFraction`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Stock YR semantics of `TechnoType Accelerates=false` for `[MTNK]` / Grizzly using DriveLocomotionClass.
**Non-Scope:** Chrono, hover, fly, jumpjet, ship, walk, and tunnel locomotor-specific acceleration behavior except where the Drive consumer branches required contrast.
**Confidence:** High
**Active in YR:** Yes

## 1. Target Question

Does stock YR `Accelerates=false` for Grizzly/MTNK DriveLocomotion mean immediate top speed, disable a ramp, switch helpers, or get ignored?

Answer: `Accelerates=false` is live for DriveLocomotion and disables the DriveLocomotion speed-fraction ramp. The drive tick still uses the normal drive track, terrain/slope speed-factor computation, `SetSpeedFraction`, `GetCurrentSpeed`, residual budget, and 7-budget-per-track-step loop. The only verified branch difference is that `TechnoType+0xDBD == 0` writes `DriveLocomotionClass+0x50` directly into the owning `TechnoClass` speed fraction via vtable slot `+0x544`; `TechnoType+0xDBD != 0` runs the ramp/brake code before writing a speed fraction.

## 2. Non-Goals

- No Grizzly-specific hardcoded path was investigated; prior work already settled that behavior is generic data-driven.
- No implementation changes were made.
- No Ghidra database mutation was performed.
- No fly/hover/chrono behavior is claimed beyond the negative fact that this DriveLocomotion consumer does not dispatch to those locomotors.

## 3. Evidence Needed To Mark COMPLETE

- Confirm `[MTNK]` INI values for `Speed=7`, `Accelerates=false`, and DriveLocomotion CLSID.
- Confirm `Accelerates` reader and backing field.
- Confirm default value when INI omits the key.
- Confirm the DriveLocomotion consumer branch, including false and true branches.
- Confirm the helper written by both branches clamps/stores the speed fraction consumed later in the same function.
- Scan current Rust surfaces enough to state the implementation implication and test targets.

All required evidence above is resolved.

## 4. Stop Conditions

- Stop if the consumer path leaves DriveLocomotion into another locomotor family.
- Stop if no live DriveLocomotion consumer exists and report PARTIAL.
- Stop if a missing function boundary blocks the direct branch; do not create functions.
- Stop after the Drive consumer and immediate speed helper are verified; do not expand into full locomotion math.

## 5. INI Keys

| Key | Stock YR value / default | Evidence | Effect |
|---|---:|---|---|
| `[MTNK] Speed` | `7` | `ini/rulesmd.ini:6618` | Raw unit type speed. Not changed by `Accelerates`. |
| `[MTNK] Locomotor` | `{4A582741-9839-11d1-B709-00A024DDAFD1}` | `ini/rulesmd.ini:6636` | DriveLocomotionClass. |
| `[MTNK] Accelerates` | `false` | `ini/rulesmd.ini:6643`; reader `0x00715402..0x00715416` | Writes `TechnoType+0xDBD = 0`; DriveLocomotion takes no-ramp branch. |
| `Accelerates` default | `true` | `TechnoTypeClass::Constructor @ 0x00710AF0` decompile sets `*(this+0xDBD)=1` | Types omitting the key use ramp branch. |

## 6. Key Offsets

| Offset | Owner | Type | Verified meaning |
|---:|---|---|---|
| `+0xDBD` | `TechnoTypeClass` | byte/bool | `Accelerates`. `1` means ramp branch; `0` means immediate speed-fraction assignment. |
| `+0x50` | `DriveLocomotionClass` | double | Drive target speed fraction / movement speed factor, set by `Process_Movement` from terrain/slope/health context and consumed by `Process_Drive_Track`. |
| `+0x4C` | `DriveLocomotionClass` | int | Residual movement budget carried between ticks. |
| `+0x58` | `DriveLocomotionClass` | int | Current drive-track index; `< 0x40` gates the normal ramp code. |
| `+0x5C` | `DriveLocomotionClass` | int | Current track step within selected track. |
| `+0x578` | `TechnoClass` | double | Current speed fraction, clamped by `TechnoClass::SetSpeedFraction`. |
| vtable `+0x544` | `TechnoClass` | method | `SetSpeedFraction(double)`, clamps input to `0.0..1.0`. |
| vtable `+0x538` | `TechnoClass`/`FootClass` | method | Current-speed budget helper consumed immediately after `SetSpeedFraction`. |

## 7. Verified Binary Findings

1. **Reader:** `TechnoTypeClass::ReadINI` reads key string `Accelerates` at `0x00843534`; assembly `0x007153FB..0x00715416` pushes the existing byte at `this+0xDBD` as the default, calls the bool reader, then stores `AL` back to `this+0xDBD`. **Active in YR: Yes**; `[MTNK]` has the key in `rulesmd.ini`.

2. **Default:** `TechnoTypeClass::Constructor @ 0x00710AF0` initializes `this+0xDBD` to `1`. **Active in YR: Yes**; object types without an `Accelerates=` override use the ramp branch.

3. **Drive consumer:** `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` calls owner vtable `+0x84` to get the type, reads `byte [type+0xDBD]` at assembly `0x004B0F69..0x004B0F81`, and branches on zero. **Active in YR: Yes**; `DriveLocomotionClass::Process @ 0x004B0500` calls `Process_Drive_Track` every active drive tick.

4. **False branch:** when `type+0xDBD == 0`, assembly `0x004B1261..0x004B1269` pushes the double at `DriveLocomotionClass+0x50` and calls vtable `+0x544` (`SetSpeedFraction`). There is no acceleration/deceleration math on this branch. **Active in YR: Yes** for Grizzly because `[MTNK] Accelerates=false`.

5. **True branch:** when `type+0xDBD != 0`, `0x004B0F87..0x004B1211` runs the ramp/brake path before the `SetSpeedFraction` call. It checks object kind, piggyback flag `owner+0x6C4 -> +0xE0C`, track index `< 0x40`, distance to destination, `SlowdownDistance` (`type+0x2F8`), current speed fraction (`owner+0x578`), `DeaccelerationFactor` (`type+0x300`), and `AccelerationFactor` (`type+0x308`). **Active in YR: Yes** for types that keep default `Accelerates=true`; not taken by stock Grizzly.

6. **Helper semantics:** `TechnoClass::SetSpeedFraction @ 0x004D3710` clamps values `>=1.0` to exactly `1.0`, values `<=0.0` to exactly `0.0`, otherwise stores the double verbatim at `TechnoClass+0x578`. **Active in YR: Yes**; DriveLocomotion calls this helper via vtable `+0x544` from both branches.

7. **Movement budget is still normal Drive:** immediately after the false/true branch join, `Process_Drive_Track` calls vtable `+0x538` at `0x004B126F..0x004B1274`, adds residual `DriveLocomotion+0x4C`, and consumes the result in chunks of `7` (`uStack_c8 -= 7`) per drive-track step. **Active in YR: Yes**; `Accelerates=false` does not bypass drive-track movement.

8. **Not raw `Speed=7` mutation:** `TechnoClass::GetTypeSpeed @ 0x0070EFE0` reads `type+0x678`; the `Accelerates` branch never writes this field. **Active in YR: Yes**; Grizzly remains `Speed=7` and only skips the speed-fraction ramp.

## 8. Core Logic

Pseudocode for the verified DriveLocomotion slice:

```text
Process_Drive_Track(loco):
  if no active track/destination:
    residual_budget = 0
    return

  type = owner.GetType()
  if type.Accelerates == false:
    owner.SetSpeedFraction(loco.target_speed_fraction)   // immediate assignment
  else:
    if owner kind/flags allow normal ramp and loco.track_index < 0x40:
      target = loco.target_speed_fraction
      current = owner.current_speed_fraction
      if near destination:
        current -= owner.speed_factor * type.DeaccelerationFactor
        clamp to minimum brake fraction constants
      else if current < target:
        current += type.AccelerationFactor
        clamp up to target
      else if current > target:
        current -= owner.speed_factor * type.DeaccelerationFactor
        clamp down to target
      owner.SetSpeedFraction(current)

  budget = owner.GetCurrentSpeed()
  budget = budget + loco.residual_budget
  while budget > 7:
    budget -= 7
    advance one drive-track step
  loco.residual_budget = budget
```

Critical detail: "immediate top speed" is accurate only if the currently computed `loco+0x50` target fraction is `1.0`. The false branch means "immediate current target speed fraction", not "ignore terrain, damage, slopes, group speed, or Drive track state."

## 9. Integration Points

| Function | Role | Evidence |
|---|---|---|
| `TechnoTypeClass::ReadINI @ 0x00715402` | Reads `Accelerates` into type data | string xref and assembly store to `+0xDBD` |
| `TechnoTypeClass::Constructor @ 0x00710AF0` | Default `Accelerates=true` | decompile initializes `+0xDBD = 1` |
| `DriveLocomotionClass::Process @ 0x004B0500` | Per-tick caller | decompile calls `Process_Drive_Track` then `Process_Movement` |
| `DriveLocomotionClass::Process_Movement @ 0x004B2630` | Computes target speed fraction `loco+0x50` from terrain/slope/health context | decompile writes `*(double *)(loco+0x50)` or calls `SetSpeedFraction` when target changes |
| `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` | Consumes `Accelerates` and advances drive tracks | decompile + assembly |
| `TechnoClass::SetSpeedFraction @ 0x004D3710` | Clamps/stores current speed fraction | decompile + assembly |

## 10. Current Rust Implementation Status

Rust does not currently parse `Accelerates=` in `src/rules/object_type.rs`; `ObjectType` has `accel_factor`, `decel_factor`, and `slowdown_distance`, but no `accelerates` bool.

Current movement command setup stamps `accel_factor`, `decel_factor`, and `slowdown_distance` into `MovementTarget` from `ObjectType` in `src/sim/world/world_commands.rs`. Because the parser does not carry `Accelerates=false`, stock MTNK still gets nonzero default acceleration/deceleration values and enters the Rust ramp branch in `src/sim/movement/movement_tick.rs`.

Current Rust implication: stock Grizzly is at risk of starting too slowly in Rust versus stock YR because the parsed rules data has no `Accelerates=false` flag and the movement tick ramps whenever acceleration/deceleration values are nonzero. To match YR, future Rust should preserve `Accelerates=false` as a parsed type flag and make Drive ground movement skip the current-speed ramp for that type, assigning current speed to the computed target speed for the tick.

## 11. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `[MTNK] INI values | verified | `ini/rulesmd.ini:6618`, `:6636`, `:6643` | none |
| `Accelerates` reader | verified | `0x00715402..0x00715416` | none |
| `Accelerates` default | verified | `TechnoTypeClass::Constructor @ 0x00710AF0` | exact constructor instruction address for the `+0xDBD=1` write not isolated; decompile is clear |
| Drive consumer branch | verified | `0x004B0F69..0x004B0F81`, `0x004B1261..0x004B1269` | none |
| Ramp branch contrast | verified | `0x004B0F87..0x004B1211` | full math constants beyond required branch not exhaustively named |
| `SetSpeedFraction` clamp/store | verified | `0x004D3710..0x004D3774` | none |
| Movement budget after branch | verified | `0x004B126F..`, `Process_Drive_Track` decompile loop `budget -= 7` | none |
| Hover/fly/chrono locomotors | deferred | out-of-scope | investigate separately if their own `Accelerates` consumers are needed |
| Current Rust parse | verified | `src/rules/object_type.rs` grep/read: no `accelerates` field/key | none |
| Current Rust ramp | verified | `src/sim/movement/movement_tick.rs` ramp on nonzero accel/decel | none |

## 12. Open Questions — Final State

- `[RESOLVED] OQ1 — Where is `Accelerates` read? -> `TechnoTypeClass::ReadINI` reads string `0x00843534` and stores bool at `+0xDBD`.` (evidence: `0x00715402..0x00715416`)
- `[RESOLVED] OQ2 — What is the default? -> `true` / `1`.` (evidence: `TechnoTypeClass::Constructor @ 0x00710AF0`)
- `[RESOLVED] OQ3 — Is Grizzly using the false value? -> yes, `[MTNK] Accelerates=false`.` (evidence: `ini/rulesmd.ini:6643`)
- `[RESOLVED] OQ4 — Is the Grizzly locomotor DriveLocomotion? -> yes, CLSID `{4A582741-...}`.` (evidence: `ini/rulesmd.ini:6636`)
- `[RESOLVED] OQ5 — What function consumes the flag for Drive? -> `DriveLocomotionClass::Process_Drive_Track`.` (evidence: `0x004B0F69..0x004B0F81`)
- `[RESOLVED] OQ6 — What happens when the flag is false? -> direct `SetSpeedFraction(loco+0x50)`.` (evidence: `0x004B1261..0x004B1269`)
- `[RESOLVED] OQ7 — What happens when the flag is true? -> ramp/brake math computes a fraction before `SetSpeedFraction`.` (evidence: `0x004B0F87..0x004B1211`)
- `[RESOLVED] OQ8 — Does false skip Drive track movement? -> no, it joins before `GetCurrentSpeed` and track budget consumption.` (evidence: `0x004B126F..`, `Process_Drive_Track`)
- `[RESOLVED] OQ9 — Does false mutate raw `Speed=7`? -> no verified write to type speed; helper writes `TechnoClass+0x578` speed fraction.` (evidence: `0x004D3710`, `0x0070EFE0`)
- `[RESOLVED] OQ10 — Does false switch locomotor helpers? -> no, same Drive function and same `SetSpeedFraction`/`GetCurrentSpeed` join are used.` (evidence: `0x004B1261..0x004B1274`)
- `[RESOLVED] OQ11 — Is this active in standard YR? -> yes for stock MTNK and many other YR vehicles with `Accelerates=false`.` (evidence: `rulesmd.ini`, DriveLocomotion caller `0x004B0500`)
- `[RESOLVED] OQ12 — What is Rust missing? -> parser has no `accelerates` field and movement ramps on default accel/decel.` (evidence: `src/rules/object_type.rs`, `src/sim/movement/movement_tick.rs`)
- `[DEFERRED] OQ13 — Do hover/fly/chrono locomotors consume `+0xDBD` differently?` (category: out-of-scope; reason: user constrained to Grizzly DriveLocomotion except same-consumer contrast; next-step-if-pursued: separate locomotor-family reports)
- `[DEFERRED] OQ14 — Exact constructor instruction address for `+0xDBD=1`.` (category: bounded-cost-too-high; reason: decompile proved the write but isolating the exact instruction was not necessary for the Drive handoff; next-step-if-pursued: disassembly sweep around `TechnoTypeClass::Constructor`)
- `[DEFERRED] OQ15 — Full numeric ramp constants for non-Grizzly `Accelerates=true` types.` (category: out-of-scope; reason: this slice only needed true-branch contrast; next-step-if-pursued: verify/update movement-speed-turn-rate)

## 13. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `[MTNK] Accelerates=false` sets `TechnoType+0xDBD=0` and DriveLocomotion takes direct speed-fraction assignment | `rulesmd.ini:6643`, `0x00715402..0x00715416`, `0x004B0F69..0x004B1269` | missing | `src/rules/object_type.rs`, `src/sim/world/world_commands.rs`, `src/sim/movement/movement_tick.rs` | Parse `Accelerates` default true; when false for ground Drive movement, skip current-speed ramp and set current speed to tick target speed | Issue a move order to stock MTNK; first movement tick should use full computed speed budget rather than ramping from zero/default acceleration | Do not model this by zeroing `AccelerationFactor`; that collides with the Rust fallback and hides the actual boolean flag |
| `Accelerates=false` does not change raw type speed, terrain speed factor, or drive-track stepping | `0x004B126F..`, `0x004D3710`, `0x0070EFE0`, `rulesmd.ini:6618` | unchecked beyond parser/ramp surfaces | `src/sim/movement/movement_tick.rs`, `src/util/fixed_math.rs`, drive track movement | Keep `Speed=7` conversion and terrain/cell speed modifiers intact; only bypass ramp toward the already-computed target | MTNK on normal terrain and damaged/slowed contexts should instantly assume the computed modified target, not an unconditional global maximum | Do not hardcode MTNK or force all `Accelerates=false` units to `speed` before terrain/health/group modifiers |
| Default `Accelerates=true` means omitted-key types still ramp | `TechnoTypeClass::Constructor @ 0x00710AF0`, Drive true branch `0x004B0F87..0x004B1211` | parser currently has no bool default | `src/rules/object_type.rs`, tests | Add `accelerates: bool` defaulting true; `false` only when INI says false | Type with no `Accelerates=` starts below full current speed if acceleration values are nonzero | Do not invert the flag name; binary branch is `0 = no ramp`, `1 = ramp` |

## 14. Negative Facts / Do Not Do

- Do not add an MTNK/Grizzly-specific branch; the binary uses generic `TechnoType+0xDBD`.
- Do not treat `Accelerates=false` as a different locomotor or pathfinder mode.
- Do not ignore terrain/slope/health speed modifiers; false branch assigns the current Drive target fraction, not a universal `1.0`.
- Do not model the flag solely through `AccelerationFactor=0`; stock `DRON` has `Accelerates=false` plus `AccelerationFactor=5`, proving they are distinct fields.
- Do not apply chrono/hover/fly assumptions to this report; only DriveLocomotion was verified here.

## 15. Concrete Rust Test-Name Proposals

- `rules_object_type_parses_accelerates_false_for_mtnk`
- `rules_object_type_accelerates_defaults_true_when_key_missing`
- `drive_movement_accelerates_false_starts_at_target_speed_first_tick`
- `drive_movement_accelerates_false_preserves_terrain_speed_modifier`
- `drive_movement_accelerates_true_still_ramps_from_rest`

## 16. Stale Docs / Follow-up Docs

Replacement wording for `docs/research/units/allied/MTNK.md` open follow-up:

> `Accelerates=false` semantics are now binary-verified. `TechnoTypeClass::ReadINI @ 0x00715402` stores the bool at `TechnoType+0xDBD`; `TechnoTypeClass::Constructor @ 0x00710AF0` defaults it to true. `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` reads `+0xDBD`: false calls `SetSpeedFraction(loco+0x50)` directly, skipping the ramp; true runs the acceleration/deceleration branch before `SetSpeedFraction`. For stock MTNK this means immediate current target speed fraction on the first drive tick, while preserving normal `Speed=7`, terrain/slope modifiers, and drive-track stepping.

## Sources

- Ghidra: `TechnoTypeClass::Constructor @ 0x00710AF0`
- Ghidra: `TechnoTypeClass::ReadINI @ 0x00715402`, string `Accelerates @ 0x00843534`
- Ghidra: `DriveLocomotionClass::Process @ 0x004B0500`
- Ghidra: `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`
- Ghidra: `DriveLocomotionClass::Process_Movement @ 0x004B2630`
- Ghidra: `TechnoClass::SetSpeedFraction @ 0x004D3710`
- Ghidra: `TechnoClass::GetTypeSpeed @ 0x0070EFE0`
- `ini/rulesmd.ini`
- `src/rules/object_type.rs`
- `src/sim/world/world_commands.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/components.rs`
