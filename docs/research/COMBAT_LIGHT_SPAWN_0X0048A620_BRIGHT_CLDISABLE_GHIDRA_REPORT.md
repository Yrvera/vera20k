# Combat Light Spawn 0x0048A620 -- Ghidra Research Report

**Address(es):** `0x0048A620` primary helper; `0x004690B0` warhead detonation caller; `0x00423AC0` damaging AnimClass caller; `0x005FF250`, `0x005FF390`, `0x005FF850`, `0x005FFFA0` transient light lifecycle/draw  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** combat-light helper around `0x0048A620`, Bright/CombatLightSize/CLDisable gates, lifetime, detail/throttle gates, and live standard-YR caller proof from Warhead detonation plus damaging AnimClass AI.  
**Non-Scope:** damage math, AnimList selection, ordinary building/map lamps, LightConvert palette construction, particle spark light details except as proof that the helper object class is shared.  
**Confidence:** High for helper identity, Warhead caller, parser offsets, flags, lifetime, and Rust deltas; Medium for exact first/last rendered frame count because draw frequency relative to the logic tick can vary with render scheduling.  
**Active in YR:** Yes, conditional on a Bright bullet or damaging animation/light caller and render/throttle gates.

## Target Question

What does the combat light spawn helper around `0x0048A620` actually create, what exact `Bright`, `CombatLightSize`, `CLDisableRed/Green/Blue`, size, lifetime, detail/throttle, and caller-liveness rules determine its player-visible output, and what should Rust implement without confusing it with map lamps, smudges, or `AnimClass` effects?

## Non-Goals

- Do not re-investigate damage formulas, AnimList selection, ordinary building/map lamps, LightConvert cache generation, particle spark light internals, or superweapon ambient transitions.
- Do not modify Rust, INI, in-repo docs, or Ghidra state.

## Evidence Needed To Mark COMPLETE

- Decompile plus assembly/xref proof of helper identity and allocation size.
- Decompile plus assembly/xref proof for Warhead detonation and damaging AnimClass caller liveness.
- Parser-address proof for `Bright`, `CombatLightSize`, and all `CLDisable*` keys.
- Lifecycle proof for creation, draw, aging, deletion, and cleanup.
- Current Rust scan proving the implementation delta.

## Stop Conditions

- Stop if the helper is proven to belong to a different visual system than the target and hand off that correction.
- Stop at the helper interface boundary for non-scoped callers; list them only as xrefs unless they contradict the core helper behavior.
- Stop before runtime-only frame-presentation claims; report the logic lifetime exactly and mark exact presented-frame count as runtime-only.

## Verified vs Inferred

- Verified from live Ghidra: helper creates a 0x18-byte transient screen-space light object, Warhead detonation caller flags/gates, parser offsets, size formula, direct draw path, age/delete lifecycle, and current Rust gaps.
- Inferred from binary plus stock INI: `CombatLightSize=40%` is intended to enter the helper as a fractional value because the binary uses `ReadDouble`, clamps at `1.0`, and stock comments describe percent semantics. No runtime capture was taken.

## 1. Overview

`0x0048A620` creates a short-lived 24-byte transient screen-space light/flash object. It is not an `AnimClass`, not a `SmudgeClass`, and not the ordinary large `LightSourceClass` used by building lamps and map cell lighting. The Warhead detonation path reaches it only after `BulletClass+0xE0 Bright` is true and the impact is not diverted to the special bouncer/nullify-crater path.

## 2. Class Layout / Key Offsets

| Object / field | Offset | Type | Verified use | Evidence | Active in YR |
|---|---:|---|---|---|---|
| transient combat light | size `0x18` | allocation | helper allocates 24 bytes before `0x005FF250` | `0x0048A6BD..0x0048A6E7` | Yes |
| transient light coord X/Y/Z | `+0x00/+0x04/+0x08` | int | copied by constructor and used by draw coords conversion | `0x005FF250`; `0x005FF850` | Yes |
| transient light age/phase | `+0x0C` | int | starts 0; logic update adds 8; delete when value reaches `0x50` | `0x005FF250`; `0x005FF390` | Yes |
| transient light size index | `+0x10` | int | indexes prebuilt 0x100x0x80 flash surface table | `0x005FF250`; `0x005FF850` | Yes |
| transient light flags | `+0x14` | uint | bit 0 darkens; bits 1..3 disable RGB brightening channels | `0x0048A6EC`; `0x005FF850` | Yes |
| Warhead CombatLightSize | `+0x13C` | float | explicit size override, clamped to `0..1` then scaled by `63.0` | `0x0075D490`; `0x0048A668..0x0048A69B` | Yes |
| Warhead Bright | `+0x150` | bool | parsed and used by helper only when caller passes force=false | `0x0075D5FF`; `0x0048A64A..0x0048A662` | Conditional |
| Warhead CLDisableRed/Green/Blue | `+0x151/+0x152/+0x153` | bool | become flags `2/4/8` in Warhead detonation caller | `0x0075D619`; `0x0075D633`; `0x0075D64D`; `0x00469BF0..0x00469C41` | Yes |
| Bullet Bright | `+0xE0` | bool | Warhead detonation branch gate for combat light | `0x00469BD6..0x00469C41`; `0x004664E2..0x004664E6` | Yes |
| Weapon Bright | `+0x12F` | bool | copied into `BulletClass+0xE0` by normal TechnoClass fire | `0x00772806`; `0x006FF831..0x006FF859` | Yes |

## 3. Core Logic

The helper receives signed damage in `ECX`, a warhead pointer in `EDX`, coords on the stack, a force/create bool, and low flag bits. Creation is allowed when either `(DetailLevel/global visual gate != 0 and frame-throttle allows)` or `(flags & 0xF) != 0`. Then either the force bool must be true, or a non-null warhead must have `Bright` at `+0x150`.

Size selection:

| Condition | Size result | Evidence |
|---|---|---|
| `CombatLightSize <= 0.0` | arithmetic `(damage << 6) >> 8`, i.e. signed `damage / 4`, clamped to `[0x15, 0x3F]` | `0x0048A668..0x0048A6B8` |
| `CombatLightSize > 0.0` | clamp `CombatLightSize` to max `1.0`, multiply by `63.0`, `Math__ftol` | `0x0048A67B..0x0048A69B`; constants `0.0`, `1.0`, `63.0` read at `0x007E1748`, `0x007E2AC8`, `0x007E518C` |

Color/draw flags:

| Flag | Meaning in draw path | Warhead source | Evidence |
|---:|---|---|---|
| `0x1` | darken all channels by `0x100 - mask_byte`; not set by Warhead detonation | other callers only | `0x005FF850` darken branch |
| `0x2` | do not brighten red channel | `CLDisableRed=yes` | `0x00469BF8..0x00469C02`; `0x005FF850` per-channel branch |
| `0x4` | do not brighten green channel | `CLDisableGreen=yes` | `0x00469C07..0x00469C11`; `0x005FF850` |
| `0x8` | do not brighten blue channel | `CLDisableBlue=yes` | `0x00469C13..0x00469C1D`; `0x005FF850` |

Lifetime:

| Step | Behavior | Evidence |
|---|---|---|
| constructor | `+0x0C = 0`, `+0x14 = 0`, then append pointer to vector `0x00AC167C/0x00AC1688` | `0x005FF250` |
| logic update | iterate vector backwards, add `8` to `+0x0C`; when new value is `>= 0x50`, remove from vector and free | `0x005FF390` |
| draw | `TacticalClass_Draw` calls `0x005FFFA0`, which draws every current vector entry through `0x005FF850` | `0x006D4664`; `0x005FFFA0` |
| cleanup | scenario/global cleanup removes each vector entry through `0x005FF2D0` and frees it | `0x00534450` |

This is a transient direct-render visual. It does not dirty or recompute the map lighting grid.

## 4. INI Keys

| Key | Owner | Default | Binary reader | Effect |
|---|---|---:|---|---|
| `WeaponType.Bright` | weapon | false | `WeaponTypeClass__ReadINI @ 0x00772806` -> `+0x12F` | normal fired bullets copy this into `Bullet+0xE0`; Warhead detonation checks that bullet field |
| `WarheadType.Bright` | warhead | false | `WarheadTypeClass__ReadINI_Body @ 0x0075D5FF` -> `+0x150` | helper fallback gate only when caller did not force creation; not the direct Warhead detonation branch gate |
| `CombatLightSize` | warhead | `0.0` | `0x0075D490` -> `+0x13C` via `ReadDouble` | if positive, overrides damage-derived size; max clamped to `1.0` before `*63.0` |
| `CLDisableRed` | warhead | false | `0x0075D619` -> `+0x151` | sets flag `0x2`; red channel remains unchanged |
| `CLDisableGreen` | warhead | false | `0x0075D633` -> `+0x152` | sets flag `0x4`; green channel remains unchanged |
| `CLDisableBlue` | warhead | false | `0x0075D64D` -> `+0x153` | sets flag `0x8`; blue channel remains unchanged |
| `[Options] DetailLevel` / global `0x00A8EB78` | options | initializer writes `2`; persisted options can override `0..2` | `OptionsClass__SetDefaults @ 0x005FA370`; xrefs from helper/draw | helper/draw throttle path uses it; nonzero CL flags bypass the zero-flag throttle branch |

Stock YR evidence: `rulesmd.ini` contains many weapon `Bright=yes` entries, many warhead `Bright=yes` entries, `CombatLightSize=40%` on a warhead, and `MirageWH` with `Bright=true`, `CLDisableBlue=true`, `CLDisableGreen=true`. The binary reader stores `CombatLightSize` as a float and the helper treats values above `1.0` as `1.0`.

## 5. Integration Points

| Caller | Live path | What is passed / gated | Evidence | Active in YR |
|---|---|---|---|---|
| `WarheadTypeClass__Detonate @ 0x004690B0` | ordinary bullet/warhead impact | checks `Bullet+0xE0`; builds flags from warhead `CLDisable*`; passes force=true; damage from bullet `+0x6C`; warhead pointer from bullet `+0x128` | `0x00469BD6..0x00469C41` | Yes |
| `AnimClass__AI @ 0x00423AC0` | damaging animation expiration/impact path | after animation damage uses the same helper for a transient light; stock art has damaging anims with `Damage`, `DamageRadius`, `Warhead` | `0x00423EA0..0x00423EF8`; `artmd.ini` damaging anim blocks | Yes, conditional by anim type |
| `TacticalClass_Draw @ 0x006D4200` | rendering | draws transient lights after object rendering and before lasers/bolts/rad beams | `0x006D4664` | Yes |
| `LogicClassPerTickUpdateLiveVector @ 0x0055B540` | logic update | ages/removes transient lights before later draw | `0x0055B5BE`; `0x005FF390` | Yes |

Other xrefs exist (Lightning Storm, waves, triggers, particles, voxel anims, ReceiveDamage), but they are outside this slot except to prove this helper is a generic transient screen flash primitive.

## 6. Current Rust Implementation Status

Current Rust parses `WeaponType::bright` and `WarheadType::bright`, but does not parse `CombatLightSize` or `CLDisableRed/Green/Blue` in `src/rules/warhead_type.rs`. No Rust surface found for transient combat lights, their 10-tick age/remove model, direct screen-space draw ordering, or CLDisable channel masks. Current map lighting lives in `src/map/lighting.rs` and is rebuilt through `src/app_init.rs`; that model is for persistent per-cell lighting and ordinary point lights, not this transient visual.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| helper identity `0x0048A620` | verified | decompile + assembly `0x0048A620`; ctor `0x005FF250` | none |
| helper size formula | verified | `0x0048A668..0x0048A6BD`; constants read from memory | none |
| helper creation gates | verified | `0x0048A62E..0x0048A668` | none |
| Warhead parser offsets | verified | `0x0075D490`; `0x0075D5FF`; `0x0075D619`; `0x0075D633`; `0x0075D64D` | none |
| Weapon Bright -> Bullet Bright | verified | `0x00772806`; `0x006FF831..0x006FF859`; `0x004664E2..0x004664E6` | none |
| Warhead detonation caller | verified | `0x00469BD6..0x00469C41` | none |
| transient light draw flags | verified | `0x005FF850` | none |
| lifetime/update/delete | verified | `0x005FF250`; `0x005FF390`; `0x00534450` | exact visual frame count under variable render cadence is runtime-only |
| damaging AnimClass caller | verified | `0x00423EA0..0x00423EF8`; stock art damaging anim keys | none for path liveness; exact visual frame count same runtime caveat |
| ordinary map LightSourceClass | deferred | out-of-scope; separate reports cover `0x00554760` | no action in this slot |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ1 -- Is 0x0048A620 a smudge, AnimClass, ordinary LightSourceClass, or distinct transient visual? -> Distinct 0x18-byte transient screen-space light object, drawn by 0x005FF850.` (evidence: `0x0048A6BD`; `0x005FF250`; `0x005FF850`)
- `[RESOLVED] OQ2 -- What gates creation? -> visual/detail frame throttle OR low flags nonzero, then force bool OR warhead Bright.` (evidence: `0x0048A62E..0x0048A668`)
- `[RESOLVED] OQ3 -- What does Warhead detonation use as Bright? -> `Bullet+0xE0`, not direct `Warhead+0x150`; normal Fire_At copies `Weapon+0x12F` into bullet Bright.` (evidence: `0x00469BD6`; `0x006FF831..0x006FF859`; `0x004664E2`)
- `[RESOLVED] OQ4 -- What are CLDisable flag bits? -> red `0x2`, green `0x4`, blue `0x8`; set channels are left unbrightened.` (evidence: `0x00469BF8..0x00469C1D`; `0x005FF850`)
- `[RESOLVED] OQ5 -- How is damage-derived size computed? -> signed arithmetic damage/4, clamped to `[21,63]`.` (evidence: `0x0048A69F..0x0048A6B8`)
- `[RESOLVED] OQ6 -- How does CombatLightSize override? -> positive float, max clamp 1.0, multiply by 63.0, `Math__ftol`.` (evidence: `0x0048A668..0x0048A69B`; `0x0075D490`)
- `[RESOLVED] OQ7 -- How long does the transient live? -> age starts 0, +=8 per logic update, removed/freed at `>=0x50`.` (evidence: `0x005FF250`; `0x005FF390`)
- `[RESOLVED] OQ8 -- Does it use map lighting dirties? -> no evidence in helper/lifecycle; it appends to a separate draw vector and draws directly.` (evidence: `0x005FF250`; `0x005FFFA0`; `0x006D4664`)
- `[RESOLVED] OQ9 -- Is the Warhead caller live in standard YR? -> yes, ordinary bullet detonation reaches `WarheadTypeClass__Detonate`, and stock weapons have `Bright=yes`.` (evidence: `0x00468D80`; `0x00469C41`; `rulesmd.ini`)
- `[RESOLVED] OQ10 -- Is the AnimClass caller live in standard YR? -> yes, stock art has damaging animations with `Damage`, `DamageRadius`, and `Warhead`, and `AnimClass__AI` calls the helper in that path.` (evidence: `0x00423EA0..0x00423EF8`; `artmd.ini`)
- `[RESOLVED] OQ11 -- Null warhead edge case? -> helper with force=false and null warhead returns before allocation; force=true can create with null warhead if the visual gate/flags allow.` (evidence: `0x0048A64A..0x0048A662`)
- `[RESOLVED] OQ12 -- Zero or negative damage edge case? -> default size path clamps low values to 21; negative values do not suppress creation once the creation gate is met.` (evidence: `0x0048A69F..0x0048A6B8`)
- `[RESOLVED] OQ13 -- Max CombatLightSize edge case? -> values above 1.0 are clamped to 1.0 before `*63.0`.` (evidence: `0x0048A67B..0x0048A690`)
- `[RESOLVED] OQ14 -- Detail/throttle edge case? -> zero flags require the visual/detail throttle branch; nonzero low flags bypass that first gate and draw branch also bypasses the zero-flag optimized path.` (evidence: `0x0048A62E..0x0048A64A`; `0x005FF850`)
- `[DEFERRED] OQ15 -- Exact number of presented frames in every render cadence` (category: `needs-runtime-debugger`; reason: logic aging is exact but render may not present once per logic update; next-step-if-pursued: runtime capture with a Bright weapon at fixed game speed)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `Tactical_ObjectRenderingLoop` | normal tactical draw | world objects | world viewport | normal object render | yes | base scene before flash |
| 2 | `0x005FFFA0` -> `0x005FF850` | each transient light currently in vector | prebuilt 0x100x0x80 BSurface table from `0x005FF720` setup | client coords from world coords, offset `-0x80,-0x40` | direct 16-bit surface pixel scaling, optional channel masks | yes | combat flash overlay |
| 3 | `LaserDrawClass__DrawAll`, EBolt, line trails, rad beam | subsequent tactical VFX | VFX-specific | world viewport | VFX-specific | yes | later overlays |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| combat-light BSurface masks | yes | yes | yes when helper creates entries | no | no | yes | no | no | `0x005FF720`; `0x005FF850` |
| AnimClass explosion SHPs | yes when selected | separately drawn | separate from combat flash | yes | no | yes | no | no | `0x00469C4E..0x00469C98` |
| SmudgeClass crater/scorch | out-of-scope | not by this helper | no claim | no | no | no | no | no | no Smudge allocation in `0x0048A620` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Bright combat impacts create a transient screen-space light, not a map lighting source | `0x0048A620`; `0x005FF250`; `0x006D4664` | missing | new render/VFX-side transient combat-light surface, fed by combat/world-effect events | create/draw/age flashes without mutating deterministic sim lighting grid | Grizzly/Rhino-style `Bright=yes` impact visibly flashes for a few frames over terrain and units | Do not implement as persistent `CellLightGrid` or ordinary building lamp |
| `CombatLightSize` overrides damage size; otherwise signed damage/4 clamps to `[21,63]` | `0x0048A668..0x0048A6B8`; `0x0075D490` | missing parser + missing effect | `src/rules/warhead_type.rs`; combat VFX event payload | parse `CombatLightSize` and compute native radius/index bounds | warhead with `CombatLightSize=40%` produces fixed medium flash independent of damage | Do not treat `40%` as `40.0` after the binary clamp; it should behave as a fraction |
| CLDisable masks preserve selected channels instead of tinting with RGB colors | `0x00469BF8..0x00469C1D`; `0x005FF850` | missing parser + missing draw flags | `src/rules/warhead_type.rs`; combat flash renderer | parse `CLDisableRed/Green/Blue` and apply per-channel brighten suppression | `MirageWH` leaves red brightening only because green and blue are disabled | Do not map CLDisable to additive RGB light colors or palette tint constants |
| transient lifetime is age `0..0x4F`, `+8` per logic update, free at `>=0x50` | `0x005FF250`; `0x005FF390`; `0x005FFFA0` | missing | render VFX scheduler / app tick integration | age and remove transient flashes on logic ticks before draw | repeated Bright impacts do not persist as lamps; old flashes disappear automatically | Do not attach lifetime to animation frame count or damage animation length |

Proposed Rust tests:

- `combat_light_weapon_bright_spawns_transient_flash_event`
- `combat_light_damage_size_clamps_to_21_63`
- `combat_light_size_percent_overrides_damage_size`
- `combat_light_cldisable_masks_channels_not_rgb_colors`
- `combat_light_lifetime_expires_at_age_0x50`
- `warhead_bright_alone_does_not_replace_bullet_bright_gate_for_normal_fire`

Stale Docs / Follow-up Docs:

- `docs/research/WARHEAD_DETONATE_GHIDRA_REPORT.md` replacement wording for lines that call `0x0048A620` a smudge/terrain deformation: "Combat-light helper `0x0048A620` allocates a 24-byte transient screen-space light object through `0x005FF250`, stores draw flags at `+0x14`, draws it from the tactical transient-light vector via `0x005FF850`, and ages/removes it in `0x005FF390`. It is not a `SmudgeClass`, not terrain deformation, and not an `AnimClass`."
- `docs/research/WARHEAD_DETONATE_GHIDRA_REPORT.md` replacement wording for Bright gate: "The normal Warhead detonation branch checks `BulletClass+0xE0 Bright`; normal `TechnoClass::Fire_At` passes `WeaponType+0x12F Bright` into `BulletClass::Init`. `WarheadType+0x150 Bright` is parsed and can gate helper callers that pass force=false, but the checked Warhead detonation combat-light call passes force=true after the bullet Bright branch."

## Sources

- Ghidra decompile/disassembly: `0x0048A620`, `0x005FF250`, `0x005FF390`, `0x005FF850`, `0x005FFFA0`, `0x006D4200`, `0x0055B540`, `0x004690B0`, `0x00423AC0`, `0x0075D3A0`, `0x00772080`, `0x004664C0`, `0x006FDD50`.
- Memory constants: `0x007E1748 = 0.0f`, `0x007E2AC8 = 1.0f`, `0x007E518C = 63.0f`.
- Prior docs checked: `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`, `WARHEAD_DETONATE_GHIDRA_REPORT.md`, `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`, `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`, `LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md`.
- INI files checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
