# AnimClass Bouncer ApplyAreaDamage Call Row - Ghidra Research Report

**Address(es):** `AnimClass::AI @ 0x00423AC0`, `Apply_area_damage @ 0x00489280`, `FUN_0048A620 @ 0x0048A620`, `CoordStruct::FromDoubles @ 0x004399A0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** The accepted bouncer/meteor impact branch call row from `AnimClass::AI` into `Apply_area_damage`, and the immediately following transient combat-light helper call.
**Non-Scope:** Full `Apply_area_damage` target iteration internals, full `ReceiveDamage` formulas, BounceClass physics, water/splash branch assets, generic warhead detonation, and Rust implementation edits.
**Confidence:** High
**Active in YR:** Conditional. The branch is active for constructed `AnimClass` instances whose type set `Bouncer=yes` or `IsMeteor=true`, whose bounce result is `1` or `2`, whose terrain/water gate accepts the impact, and whose `ExpireAnim=` pointer is non-null. Retail `artmd.ini` has stock bouncer/meteor examples (`DBRIS1LG`, `METLARGE`, `METSMALL`, `METDEBRI`).

Working notes:
- `Target question`: What exact call row does `AnimClass::AI` use for bouncer accepted-impact `Apply_area_damage @ 0x00489280` and the following `FUN_0048A620` helper?
- `Non-goals`: Do not re-derive settled `BounceAnim`/`ExpireAnim` constructor rows, direct same-cell bounce damage, full area-damage internals, or full combat-light rendering beyond this caller's arguments.
- `Evidence needed to mark COMPLETE`: assembly proof for gate, stack/register argument order, coordinate conversion, damage conversion, source/owner/warhead values, helper call arguments, and side-effect ordering.
- `Stop conditions`: stop once the `ExpireAnim` non-null/allocation split, `Apply_area_damage` call row, helper call row, Rust-facing delta, stale-doc wording, and remaining uncertainties are recorded.

## 1. Overview

The bouncer accepted-impact branch does not use the parent anim as the `Apply_area_damage` source object and does not credit a house. It converts the embedded BounceClass floating position to integer `CoordStruct`, converts `AnimType.Damage` with `Math::ftol`, passes `AnimType.Warhead`, pushes source object `0`, destroy-overlay/tiberium flag `1`, owner house `0`, then calls `Apply_area_damage`.

The immediately following helper call is `FUN_0048A620`, the transient combat-light helper, with the same converted damage, same warhead, same converted impact coords, force/create bool `0`, and low flag bits `0`. Both the area damage and helper are gated by `ExpireAnim != null`; allocation failure for the visual `ExpireAnim` row jumps to the damage/helper block and does not skip it.

## 2. Class Layout / Key Offsets

| Field | Offset | Type | Purpose | Active in YR |
|---|---:|---|---|---|
| `AnimClass.Type` | `+0x0C8` | `AnimTypeClass*` | Reads `ExpireAnim`, `Damage`, and `Warhead` for this branch | Conditional |
| `AnimClass.BounceClass` | `+0x128` | embedded bounce state | `CoordStruct::FromDoubles` source for impact coords | Conditional |
| copied bounce coords | `+0x140` | three floats/doubles copied to locals | visual `ExpireAnim` constructor coords | Conditional |
| `AnimType.Damage` | `+0x2A8` | double | converted with `Math::ftol` for both area damage and helper size | Conditional |
| `AnimType.ExpireAnim` | `+0x304` | `AnimTypeClass*` | non-null gate for visual row plus damage/helper block | Conditional |
| `AnimType.Warhead` | `+0x330` | `WarheadTypeClass*` | passed to `Apply_area_damage` and `FUN_0048A620` | Conditional |
| `Warhead.CombatLightSize` | `+0x13C` | float | helper size override | Conditional |
| `Warhead.Bright` | `+0x150` | bool | helper fallback gate when force bool is zero | Conditional |

## 3. Core Logic

### 3.1 Gate and Visual Allocation Split

Active in YR: Conditional.

`AnimClass::AI` enters the accepted-impact branch after the bouncer vtable result is `1` or `2` and the terrain/water gate routes to `0x00423DE7`. It reads `AnimType+0x304`:

- `0x00423DE7`: `MOV EDX,[ESI+0xC8]`
- `0x00423DED`: `MOV EAX,[EDX+0x304]`
- `0x00423DF3..0x00423DF5`: null test; null jumps to `0x00423EFD`

If `ExpireAnim` is non-null, it attempts `operator_new(0x1C8)` for the visual row. Allocation failure jumps to `0x00423E75`, the damage/helper block. Therefore:

- `ExpireAnim == null`: no visual row, no `Apply_area_damage`, no helper.
- `ExpireAnim != null` and allocation succeeds: visual row, then `Apply_area_damage`, then helper.
- `ExpireAnim != null` and allocation fails: no visual row, but `Apply_area_damage` and helper still run.

Evidence: `0x00423DF5 -> 0x00423EFD`; `0x00423E24..0x00423E26` allocation null jumps to `0x00423E75`.

### 3.2 `Apply_area_damage` Call Row

Active in YR: Conditional.

The call at `0x00423EAB` uses `Apply_area_damage`'s normal `__fastcall` shape:

`Apply_area_damage(CoordStruct* coords, int damage, ObjectClass* source, WarheadTypeClass* warhead, char destroy_tiberium, HouseClass* owner_house)`

Verified bouncer caller row:

| Parameter | Value | Evidence |
|---|---|---|
| `ECX` coords | pointer returned by `CoordStruct::FromDoubles(&local)` from embedded `AnimClass+0x128` bounce state | `LEA EDI,[ESI+0x128]` at `0x00423E7F`; `CALL 0x004399A0` at `0x00423EA2`; `MOV ECX,EAX` at `0x00423EA7` |
| `EDX` damage | `Math::ftol(AnimType+0x2A8 Damage)` | `FLD [EAX+0x2A8]` at `0x00423E8B`; `CALL 0x007C5F00` at `0x00423E94`; `MOV EBP,EAX`; `MOV EDX,EBP` |
| stack arg 1 source object | `0` | `PUSH 0x0` at `0x00423E92` before the call row |
| stack arg 2 warhead | `AnimType+0x330` | `MOV ECX,[EAX+0x330]` at `0x00423E85`; `PUSH ECX` at `0x00423E91` |
| stack arg 3 destroy overlay/tiberium flag | `1` | `PUSH 0x1` at `0x00423E7D` |
| stack arg 4 owner house | `0` | `PUSH 0x0` at `0x00423E7B` |

`Apply_area_damage @ 0x00489280` confirms the convention: it copies `ECX` to the coord local, `EDX` to base damage, reads stack `+0x8` as source object, stack `+0xC` as warhead, stack `+0x10` as the destroy-tiberium/overlay side-effect flag, and stack `+0x14` as owner/house context. Evidence: entry `0x00489291..0x004892D7`, receive-damage call context `0x00489A91..0x00489AB6`, and prior normal detonation argument report.

Consequences:

- The parent `AnimClass` pointer is not passed as source.
- The parent owner object, if any, is not consulted here.
- The owner house context is null/zero.
- `destroy_tiberium` is forced true (`1`), so Apply-area side effects that depend on that flag are enabled, subject to the warhead and cell gates inside `Apply_area_damage`.

### 3.3 Coordinate Conversion

Active in YR: Conditional.

The impact coords for both calls come from the embedded bounce state, not from `AnimClass.GetCoords()` and not from the earlier visual-constructor local copy. At `0x00423E7F` the code sets `EDI = AnimClass + 0x128`, then calls `CoordStruct::FromDoubles @ 0x004399A0` with a local output pointer.

`CoordStruct::FromDoubles` performs three `Math::ftol` conversions and writes X/Y/Z to the provided `CoordStruct`. Evidence: decompile of `0x004399A0`.

### 3.4 `FUN_0048A620` Helper Call Row

Active in YR: Conditional.

After `Apply_area_damage`, the branch re-reads the same type, converts the same `Damage`, converts the same embedded bounce state into coords, then calls `FUN_0048A620 @ 0x0048A620`.

Verified bouncer caller row:

| Parameter | Value | Evidence |
|---|---|---|
| `ECX` damage | `Math::ftol(AnimType+0x2A8 Damage)` | `0x00423EBA..0x00423ECF`; `MOV ECX,EBP` at `0x00423EF6` |
| `EDX` warhead | `AnimType+0x330 Warhead` | `MOV EBP,[EAX+0x330]` at `0x00423EC0`; stored/reloaded through stack; `MOV EDX,[ESP+0x30]` at `0x00423EEF` |
| stack coords | by-value `CoordStruct` from `CoordStruct::FromDoubles(AnimClass+0x128)` | `CALL 0x004399A0` at `0x00423ED8`; copy three dwords to `ESP` at `0x00423EDF..0x00423EF3` |
| stack force/create bool | `0` | `PUSH 0x0` at `0x00423EB8` |
| stack low flag bits | `0` | `PUSH 0x0` at `0x00423EB6` |

`FUN_0048A620` itself stores `ECX` as signed damage and `EDX` as warhead pointer (`0x0048A62C..0x0048A630`). Because this bouncer caller passes force bool `0` and flags `0`, the helper creates a transient light only if its global/detail throttle allows the call and the warhead pointer is non-null with `Warhead+0x150 Bright` true. Evidence: helper gates `0x0048A62E..0x0048A668`.

The helper is not an `AnimClass` spawner. It allocates a 24-byte transient visual (`operator_new(0x18)`) and calls `0x005FF250`; no simulation damage is performed by this helper. Evidence: `0x0048A6BD..0x0048A6EC`.

### 3.5 Side-Effect Sequence

Active in YR: Conditional.

For accepted bouncer/meteor impact with non-null `ExpireAnim`, the order is:

1. Optional visual `ExpireAnim` allocation and constructor if allocation succeeds (`0x00423E1A..0x00423E70`).
2. `Apply_area_damage` call, even if the visual allocation failed (`0x00423E75..0x00423EAB`).
3. `FUN_0048A620` transient combat-light helper (`0x00423EB0..0x00423EF8`).
4. Later bouncer AI continuation and parent destruction path.

No branch exists between the `Apply_area_damage` return and the helper call in `0x00423EAB..0x00423EF8`; helper execution is not gated by the return value from `Apply_area_damage`.

## 4. INI Keys

| Key | Owner | Binary field | Default/effect in this slice | Active in YR |
|---|---|---:|---|---|
| `ExpireAnim=` | AnimType | `+0x304` | null skips visual, area damage, and helper | Conditional |
| `Damage=` | AnimType | `+0x2A8` | double converted with `Math::ftol`; zero causes `Apply_area_damage` early return but helper can still create if its visual gates pass | Conditional |
| `Warhead=` | AnimType | `+0x330` | null causes `Apply_area_damage` early return and prevents helper creation when force=false | Conditional |
| `Bouncer=` | AnimType | `+0x35A` | constructor enables bouncer path | Conditional |
| `IsMeteor=` | AnimType | `+0x356` | constructor enables bouncer path | Conditional |
| `Bright=` | Warhead | `+0x150` | helper fallback gate because this caller passes force bool `0` | Conditional |
| `CombatLightSize=` | Warhead | `+0x13C` | helper size override when positive | Conditional |

Retail evidence includes `ini/artmd.ini:14986..14999` (`DBRIS1LG`: `ExpireAnim=TWLT036`, `Damage=20`, `DamageRadius=80`, `Warhead=HE`, `Bouncer=yes`) and `ini/artmd.ini:19065..19119` style meteor/debris sections (`METLARGE`, `METSMALL`, `METDEBRI`) with `IsMeteor=true` or `Bouncer=yes`, `ExpireAnim`, `Damage`, and `Warhead`.

## 5. Integration Points

The scoped path is:

`AnimClass::AI` bouncer result `1/2` -> accepted terrain/water gate -> `ExpireAnim != null` -> optional visual constructor -> `Apply_area_damage` with source/owner zero -> `FUN_0048A620` with force/flags zero -> AI continuation.

`Apply_area_damage` then owns normal cell/object/overlay side effects. This report only verifies the bouncer caller's argument values; it does not claim the full downstream damage distribution formula.

`FUN_0048A620` owns transient combat-light creation. With this bouncer caller, its own gates matter because the caller does not force creation and passes no color-mask flags.

## 6. Current Rust Implementation Status

Current Rust has general combat AoE surfaces in `src/sim/combat/mod.rs` and `src/sim/combat/combat_aoe.rs`, plus `WarheadType` parsing in `src/rules/warhead_type.rs`. It parses `bright`, but the existing scanned `WarheadType` surface does not expose `CombatLightSize` or `CLDisable*` masks.

Current Rust parses `bounce_anim` and `expire_anim` in `src/rules/art_data.rs`, but no generic SHP bouncer/meteor runtime was found. `src/sim/components.rs` has generic spawn descriptors and world effects, but no accepted-impact bouncer branch that calls AoE damage with null source/null house, nor a transient combat-light helper event matching `0x0048A620`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AnimClass::AI` `ExpireAnim` non-null gate | verified | `0x00423DE7..0x00423DF5` | none |
| visual allocation failure still reaches side effects | verified | `0x00423E24..0x00423E26 -> 0x00423E75` | none |
| bouncer `Apply_area_damage` call row | verified | `0x00423E75..0x00423EAB`; `0x00489280` entry | none |
| coord conversion source | verified | `0x00423E7F..0x00423EA7`; `0x004399A0` | exact BounceClass internal float update math out of scope |
| helper `FUN_0048A620` call row | verified | `0x00423EB0..0x00423EF8`; `0x0048A620` entry | none |
| helper creation gates for this caller | verified | `0x0048A62E..0x0048A668` | exact presented frame count deferred to combat-light report/runtime |
| full `Apply_area_damage` internals | touched-not-exhausted | `0x00489280` decompile | broader warhead/overlay/object effects are outside this slot |
| Rust bouncer runtime | verified-missing | `rg` over `src/rules/art_data.rs`, `src/sim/components.rs`, `src/sim/combat` | implement later, no Rust edits here |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - What gates the call row? -> bouncer return 1/2, accepted terrain/water gate, and non-null ExpireAnim; allocation success is not required for damage/helper.` (evidence: `0x00423C24..0x00423CCF`; `0x00423DE7..0x00423E75`)
- `[RESOLVED] OQ-02 - What coords are passed to Apply_area_damage? -> integer CoordStruct from embedded BounceClass at AnimClass+0x128 via CoordStruct::FromDoubles.` (evidence: `0x00423E7F..0x00423EA7`; `0x004399A0`)
- `[RESOLVED] OQ-03 - What damage amount is passed? -> Math::ftol of AnimType+0x2A8 Damage.` (evidence: `0x00423E8B..0x00423EA9`)
- `[RESOLVED] OQ-04 - What warhead is passed? -> AnimType+0x330 Warhead pointer.` (evidence: `0x00423E85`; `0x00423E91`)
- `[RESOLVED] OQ-05 - What source object is passed? -> zero/null, not parent AnimClass and not parent owner.` (evidence: `PUSH 0x0` at `0x00423E92`; `Apply_area_damage` stack convention at `0x00489280`)
- `[RESOLVED] OQ-06 - What owner house is passed? -> zero/null.` (evidence: `PUSH 0x0` at `0x00423E7B`)
- `[RESOLVED] OQ-07 - Is destroy-tiberium/overlay flag enabled? -> yes, stack arg is `1`.` (evidence: `PUSH 0x1` at `0x00423E7D`)
- `[RESOLVED] OQ-08 - Does helper use the same data? -> same damage, same warhead, same embedded BounceClass coords.` (evidence: `0x00423EB0..0x00423EF8`)
- `[RESOLVED] OQ-09 - Is helper forced? -> no; force bool and low flags are both zero, so helper requires its normal visual gate and warhead Bright fallback.` (evidence: `0x00423EB6..0x00423EB8`; `0x0048A62E..0x0048A668`)
- `[RESOLVED] OQ-10 - Is helper an AnimClass or damage helper? -> no; it allocates a 0x18 transient light and calls `0x005FF250`.` (evidence: `0x0048A6BD..0x0048A6EC`)
- `[DEFERRED] OQ-11 - Full downstream Apply_area_damage object/overlay damage behavior?` (category: `out-of-scope`; reason: this slot only verifies bouncer caller arguments; next-step-if-pursued: dedicated Apply_area_damage branch audit)
- `[DEFERRED] OQ-12 - Exact combat-light rendered pixels/frame count?` (category: `needs-runtime-debugger`; reason: helper/lifetime are documented elsewhere, but rendered frame count depends on runtime cadence; next-step-if-pursued: capture fixed-speed Bright impact frames)

## 9. Visual/UI Composition Ledger

This report only claims creation of a transient combat light from this caller.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `AnimClass::AI @ 0x00423E70` | non-null `ExpireAnim`; allocation success | `ExpireAnim` AnimType | impact coords | normal AnimClass draw later | Conditional | impact visual |
| 2 | `Apply_area_damage @ 0x00423EAB` | non-null `ExpireAnim`; allocation success not required | none | impact coords | none | Conditional | simulation/overlay side effects |
| 3 | `FUN_0048A620 @ 0x00423EF8` | non-null `ExpireAnim`; helper's own gates | transient 0x18 light | impact coords | direct transient-light renderer | Conditional | combat flash |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `ExpireAnim=` target | conditional | conditional | conditional | no | no | yes | no | no | `0x00423E51..0x00423E70` |
| transient combat-light object | conditional | conditional | conditional | no | no | yes | no | no | `0x0048A620`; `0x005FF250` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Bouncer accepted-impact area damage uses coords from embedded BounceClass, damage `ftol(AnimType.Damage)`, warhead `AnimType.Warhead`, source `0`, owner house `0`, destroy flag `1` | `0x00423E75..0x00423EAB`; `0x00489280` convention | missing | future generic `AnimClass` bouncer runtime; `src/sim/combat/*` bridge | Call AoE with null source/null house and true overlay/tiberium side-effect flag after accepted impact | `anim_bouncer_area_damage_callrow_uses_null_source_and_owner` | Do not credit the parent anim owner or use the parent `AnimClass` as damage source |
| `ExpireAnim != null` gates damage/helper; visual allocation success does not | `0x00423DF3..0x00423E26`; `0x00423E75` | missing | future bouncer runtime and effect allocation boundary | Side effects run when `ExpireAnim` resolves but visual spawn is disabled/fails; no side effects when `ExpireAnim` is absent | `anim_bouncer_expireanim_ref_gates_damage_not_visual_allocation` | Do not tie deterministic damage to renderer/effect allocation success |
| Helper call is transient combat light with force `0`, flags `0`, same damage/warhead/coords | `0x00423EB0..0x00423EF8`; `0x0048A620` | missing | new transient combat-light VFX event; `src/rules/warhead_type.rs` for `CombatLightSize`/CL masks | Emit helper event only if native helper gates pass: visual throttle/flags and force-or-warhead-Bright | `anim_bouncer_combat_light_helper_uses_warhead_bright_gate` | Do not always spawn a flash for every bouncer damage event |
| `Apply_area_damage` return does not gate the helper call | `0x00423EAB..0x00423EF8` | missing | future bouncer runtime | Run helper setup immediately after AoE call regardless of boolean/2 return | `anim_bouncer_helper_runs_after_area_damage_return_value_ignored` | Do not skip flash because AoE captured/killed/no-targeted something |

### Stale Docs / Follow-up Docs

- `docs/research/ANIMCLASS_BOUNCER_METEOR_EXPIREANIM_IMPACT_SPAWNS_GHIDRA_REPORT.md`: replace any wording that says allocation success gates damage/helper with: `ExpireAnim+0x304 non-null gates the bouncer accepted-impact visual, Apply_area_damage, and FUN_0048A620 block. The visual allocation result gates only the ExpireAnim constructor; allocation failure jumps to 0x00423E75, where area damage and the helper still run.`
- `docs/research/ANIMCLASS_BOUNCER_IMPACT_GATES_GHIDRA_REPORT.md`: add call-row detail: `Bouncer AI calls Apply_area_damage with coords from AnimClass+0x128 via CoordStruct::FromDoubles, damage ftol(Type+0x2A8), source object 0, warhead Type+0x330, destroy flag 1, owner house 0. The following FUN_0048A620 call uses the same damage/warhead/coords with force=0 and flags=0.`

## Negative Facts / Do Not Do

- Do not pass the parent `AnimClass` as the `sourceObj` to `Apply_area_damage`.
- Do not infer or synthesize a source house from an owner link for this bouncer branch; the caller passes owner house `0`.
- Do not make `Damage=` alone trigger this area-damage block; `ExpireAnim` non-null is the caller-side gate.
- Do not skip area damage or helper when the `ExpireAnim` visual allocation fails.
- Do not always emit a combat-light flash; this caller passes force `0` and flags `0`, so `Warhead.Bright` and helper visual gates still matter.
- Do not collapse the helper into map lighting or an `AnimClass`; `FUN_0048A620` creates a transient 0x18 visual object.

## Remaining Uncertainty

- Full downstream `Apply_area_damage` behavior is intentionally out of scope; this report only proves the bouncer caller row.
- Exact runtime presented-frame count and final pixels for the transient combat light remain delegated to the combat-light visual report/runtime capture.
- Exact BounceClass float evolution before `CoordStruct::FromDoubles` is out of scope; this report verifies only the conversion source and call row.

## Sources

- Read-only Ghidra decompile/context: `AnimClass::AI @ 0x00423AC0`, call ranges `0x00423DE7..0x00423EF8`.
- Read-only Ghidra decompile: `Apply_area_damage @ 0x00489280`, `FUN_0048A620 @ 0x0048A620`, `CoordStruct::FromDoubles @ 0x004399A0`.
- Prior reports checked: `ANIMCLASS_BOUNCER_IMPACT_GATES_GHIDRA_REPORT.md`, `ANIMCLASS_BOUNCER_METEOR_EXPIREANIM_IMPACT_SPAWNS_GHIDRA_REPORT.md`, `COMBAT_LIGHT_SPAWN_0X0048A620_BRIGHT_CLDISABLE_GHIDRA_REPORT.md`, `WARHEAD_DETONATE_GHIDRA_REPORT.md`, `AAHEATSEEKER2_GUARDWH_DETONATION_PARAMETERS_GHIDRA_REPORT.md`.
- INI checked: `ini/artmd.ini`, `ini/art.ini`, `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan: `src/rules/art_data.rs`, `src/rules/warhead_type.rs`, `src/sim/components.rs`, `src/sim/combat/mod.rs`, `src/sim/combat/combat_aoe.rs`.
