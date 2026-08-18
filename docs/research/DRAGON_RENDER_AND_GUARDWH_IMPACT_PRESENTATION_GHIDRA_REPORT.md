# DRAGON Render and GUARDWH Impact Presentation - Ghidra Research Report

**Address(es):** `BulletTypeClass__ReadINI`, `ObjectTypeClass__ReadINI`, `ObjectClass__Reveal`, `ObjectClass__DrawIt`, `LineTrail__*`, `BulletClass__AI`, `BulletClass__BulletDetonation`, `WarheadTypeClass__Detonate`, `Warhead__SelectExplosionAnim`
**Investigation Mode:** exhaustive-slice; updated with follow-up draw-slot findings
**Claimed Scope:** AAHeatSeeker2 `Image=DRAGON` presentation, DRAGON line trail/trailer/shadow/rotation parse path, and GUARDWH impact animation/sound source.
**Non-Scope:** homing physics, damage math, targeting legality, full ObjectClass/TechnoClass rendering architecture, and all non-DRAGON projectile art.
**Confidence:** High for parsing, line trail lifecycle, trailer absence, shadow parse, object draw dispatch, and GUARDWH impact AnimList timing; Medium for render-layer implications; Low/Deferred for exact DRAGON facing-to-SHP-frame arithmetic.
**Active in YR:** Yes for the verified AAHeatSeeker2/DRAGON/GUARDWH paths; see each finding.

## 1. Overview

`AAHeatSeeker2` is a visible non-Inviso bullet using `Image=DRAGON`; `BulletTypeClass__ReadINI` delegates inherited image/art parsing to `ObjectTypeClass__ReadINI`, then reads bullet-specific DRAGON art keys from the image section. In standard YR, DRAGON does not use the commented `Trailer=SMOKEY2`; its visible tail is the `UseLineTrail=yes` object-attached line trail. At detonation, `BulletClass__BulletDetonation` calls `WarheadTypeClass__Detonate`, and the impact presentation is selected from `GUARDWH`'s `AnimList` at impact time rather than at fire time.

## 2. Key Offsets

| Class / field | Offset | Meaning | Active in YR | Evidence |
|---|---:|---|---|---|
| ObjectClass | `+0xA8` | attached `LineTrail*` | Yes | `ObjectClass__Reveal`; `LINE_TRAIL_CLASS_GHIDRA_REPORT.md` |
| ObjectClass | `+0x9C..+0xA4` | owner world coords used by LineTrail update | Yes | `LineTrail__Update` |
| ObjectTypeClass | `+0x1F8` | inherited `Image=` / art section name | Yes | `ObjectTypeClass__ReadINI`; `BulletTypeClass__ReadINI` |
| ObjectTypeClass | `+0x23A` | `UseLineTrail` | Yes | `ObjectTypeClass__ReadINI`, string `UseLineTrail` |
| ObjectTypeClass | `+0x23B..+0x23D` | `LineTrailColor` RGB | Yes | `ObjectTypeClass__ReadINI` |
| ObjectTypeClass | `+0x240` | `LineTrailColorDecrement` | Yes | `ObjectTypeClass__ReadINI` |
| BulletTypeClass | `+0x29A` | `Shadow` | Yes | `BulletTypeClass__ReadINI`; `rulesmd.ini:25680` |
| BulletTypeClass | `+0x2A1` | inverted `Rotates` storage | Yes | `BulletTypeClass__ReadINI`; `artmd.ini:14760` |
| BulletTypeClass | `+0x2D8` | `Trailer` AnimType pointer | Conditional: only if active `Trailer=` exists | `BulletTypeClass__ReadINI`; DRAGON has only `;Trailer=SMOKEY2` |
| BulletTypeClass | `+0x2E4` | trailer `SpawnDelay` | Conditional: only if `Trailer` non-null | `BulletClass__AI`; `BULLET_CLASS_AI_GHIDRA_REPORT.md` |
| BulletClass | `+0x12C/+0x12D` | sprite animation frame/timer | Conditional: only for `AnimLow/High/Rate` ranges | `BULLET_CLASS_AI_GHIDRA_REPORT.md` |
| WarheadTypeClass | `+0x104..+0x11F` | `AnimList` vector | Yes | `Warhead__SelectExplosionAnim`; `rulesmd.ini:26909` |
| WarheadTypeClass | `+0x154` | random AnimList flag / EMEffect selector | Conditional | `Warhead__SelectExplosionAnim` |

## 3. Core Findings

### 3.1 DRAGON SHP/art loading

`AAHeatSeeker2` has `Image=DRAGON` in both base and YR rules, and `Inviso` remains false. `BulletTypeClass__ReadINI` first calls `ObjectTypeClass__ReadINI`, then reads `Image` into the inherited image buffer and calls the SHP loader when `Inviso==0`.

Active in YR: Yes. Evidence: `rulesmd.ini:25678-25687`, `BulletTypeClass__ReadINI`, `ObjectTypeClass__ReadINI`, `BULLETTYPECLASS_GHIDRA_REPORT.md`.

### 3.2 `Rotates=yes` frame mapping is resolved by follow-up report

`[DRAGON] Rotates=yes` is read from the art/image section, not from the rules projectile section. The binary stores it inverted: `Rotates=yes` makes `BulletType+0x2A1` false, while `Rotates=no` makes it true. The Object draw pipeline calls `ObjectClass__DrawIt` through vtable `+0x104`, converts world coords to screen coords, clips, then dispatches to BulletClass vtable `+0x114`.

Follow-up report `DRAGON_BULLET_DRAW_SLOT_FRAME_MAPPING_GHIDRA_REPORT.md` resolves the concrete draw slot as `0x00468090` with SHP frame selection routed through `0x00468000`. For rotating bullet art, gamemd maps BulletClass facing BAM to frame as:

```text
index = ((((uint16)bam >> 10) + 1) >> 1) & 0x1F
frame = (28 - index) & 31
```

Active in YR: Yes. Evidence: `BulletTypeClass__ReadINI`, `ObjectClass__DrawIt`, `DRAGON_BULLET_DRAW_SLOT_FRAME_MAPPING_GHIDRA_REPORT.md`, `OBJECTCLASS_DRAW_LIMBO_CELLLIST.md:55-101`, `artmd.ini:14760`.

### 3.3 UseLineTrail lifecycle and cadence

`[DRAGON] UseLineTrail=yes` is read by `ObjectTypeClass__ReadINI` into ObjectType `+0x23A`. `ObjectClass__Reveal` allocates a 0x210-byte LineTrail when the object reveals and the type flag is set, stores it at ObjectClass `+0xA8`, chooses the per-type RGB unless `Rules.LineTrailColorOverride` is nonzero, and applies `LineTrailColorDecrement`.

`LineTrail__UpdateAndDrawAll` runs through the global trail vector during tactical draw, not through BulletClass AI. It updates and draws per rendered frame. Update inserts a new ring-buffer point only when owner coordinates changed, then ages every slot by `ColorDecrement` clamped at zero. Draw converts each point to screen coords and calls the alpha line rasterizer with z-adjusted endpoints.

Active in YR: Yes. Evidence: `ObjectTypeClass__ReadINI`, `ObjectClass__Reveal`, `LineTrail__Constructor`, `LineTrail__Update`, `LineTrail__Draw`, `LineTrail__UpdateAndDrawAll`, `artmd.ini:14757-14759`.

### 3.4 No active DRAGON `Trailer=SMOKEY2`

`BulletTypeClass__ReadINI` only resolves a trailer if `ReadString(art_section, "Trailer", ...)` returns a non-empty value. DRAGON's `Trailer=SMOKEY2` line is commented in both `art.ini` and `artmd.ini`, so the pointer at BulletType `+0x2D8` remains null for stock DRAGON. `BulletClass__AI` trailer spawning is gated on that pointer, so no recurring SMOKEY2 trailer anim is spawned for AAHeatSeeker2 in stock YR.

Active in YR: No for SMOKEY2 on DRAGON; Conditional for other projectile art sections with active `Trailer=`. Evidence: `artmd.ini:14755-14760`, `BulletTypeClass__ReadINI`, `BulletClass__AI`, `BULLET_CLASS_AI_GHIDRA_REPORT.md:340-358`.

### 3.5 `Shadow=no` is consumed as a BulletType key

`AAHeatSeeker2` sets `Shadow=no`; `BulletTypeClass__ReadINI` reads `Shadow` from the rules projectile section into BulletType `+0x29A`. This overrides the constructor default `true`. The follow-up draw-slot report resolves the draw branch: shadow drawing requires `height > 0` and `BulletType+0x29A != 0`, so stock DRAGON/AAHeatSeeker2 suppresses the projectile shadow.

Active in YR: Yes. Evidence: `rulesmd.ini:25678-25680`, `BulletTypeClass__ReadINI`, `BULLETTYPECLASS_GHIDRA_REPORT.md`, `DRAGON_BULLET_DRAW_SLOT_FRAME_MAPPING_GHIDRA_REPORT.md`.

### 3.6 Altitude and layer implications

The object draw path is altitude-aware at the Object/LineTrail layer: `ObjectClass__DrawIt` gets render coords via vtable `+0xAC`, converts them with `TacticalClass__CoordsToClient2`, and `LineTrail__Draw` adjusts per-point depth through `Tactical__AdjustForZ` before drawing alpha lines. Display submission sorts objects by layer first and Y-sort within layers. Follow-up report `DRAGON_BULLET_DRAW_SLOT_FRAME_MAPPING_GHIDRA_REPORT.md` resolves `BulletClass::GetLayer @ 0x00468B90`; stock non-`Flat` DRAGON draws on layer `3`.

Active in YR: Yes. Evidence: `ObjectClass__DrawIt`, `LineTrail__Draw`, `DRAGON_BULLET_DRAW_SLOT_FRAME_MAPPING_GHIDRA_REPORT.md`, `OBJECTCLASS_DRAW_LIMBO_CELLLIST.md:127-146`, `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md:1151-1158`.

### 3.7 GUARDWH impact anim/sound timing

`BulletClass__BulletDetonation` calls `WarheadTypeClass__Detonate` when the bullet detonates. In `WarheadTypeClass__Detonate`, the impact coordinate is prepared at detonation time, `Warhead__SelectExplosionAnim` selects from the warhead `AnimList`, and an `AnimClass` is constructed with draw flags `0x2600`. For `GUARDWH`, the stock `AnimList` is `XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070`.

Explosion sound is not a dedicated warhead field in the verified docs; it comes from the chosen explosion animation's `StartSound=` / `Report=` if present. Therefore timing is impact-time, through the impact AnimClass construction, not fire-time and not DRAGON-specific.

Active in YR: Yes. Evidence: `BulletClass__BulletDetonation`, `WarheadTypeClass__Detonate`, `Warhead__SelectExplosionAnim`, `AnimClass__Constructor`, `rulesmd.ini:26902-26912`, `ANIMATION_SOUNDS_GHIDRA_REPORT.md:231-263`.

## 4. INI Keys

| Section | Key | Stock YR value | Effect | Active in YR |
|---|---|---|---|---|
| `[AAHeatSeeker2]` | `Image` | `DRAGON` | inherited image/art section and SHP load | Yes |
| `[AAHeatSeeker2]` | `Shadow` | `no` | writes BulletType `+0x29A` false | Yes |
| `[DRAGON]` | `UseLineTrail` | `yes` | object reveal creates LineTrail | Yes |
| `[DRAGON]` | `LineTrailColor` | `216,216,255` | trail RGB unless rules override nonzero | Yes |
| `[DRAGON]` | `LineTrailColorDecrement` | `16` | per-frame brightness decrement | Yes |
| `[DRAGON]` | `Rotates` | `yes` | inverted write to BulletType `+0x2A1` | Yes |
| `[DRAGON]` | `Trailer` | commented `;Trailer=SMOKEY2` | no active trailer pointer | No for DRAGON |
| `[GUARDWH]` | `AnimList` | explosion list above | impact anim pool | Yes |
| `[GUARDWH]` | `CellSpread` | `.5` | damage spread, not presentation except effect scale context | Yes |
| `[GUARDWH]` | `PercentAtMax` | `.5` | damage falloff, not presentation | Yes |

## 5. Integration Points

| Integration point | Verified behavior | Active in YR |
|---|---|---|
| Rules/art parse | `BulletTypeClass__ReadINI` reads projectile keys and DRAGON art keys | Yes |
| Object reveal | `BulletClass::Fire`/reveal path submits object; `ObjectClass__Reveal` attaches LineTrail if type flag set | Yes |
| Render frame | `LineTrail__UpdateAndDrawAll` updates/draws every tactical draw, not every sim tick | Yes |
| Bullet AI | Recurring trailer anim spawn only runs if `BulletType+0x2D8` is non-null | Conditional; no for DRAGON |
| Detonation | `BulletClass__BulletDetonation` calls `WarheadTypeClass__Detonate` | Yes |
| Impact anim | `Warhead__SelectExplosionAnim` selects from warhead `AnimList`; `AnimClass__Constructor` spawns at impact | Yes |

## 6. Current Rust Implementation Status

Observed implementation points, not audited for correctness in this slot:

| Area | Rust path | Status |
|---|---|---|
| Projectile art parse | `src/rules/projectile_type.rs` | Has `Image`, `Shadow`, art-side `Rotates`, `Trailer` fields/tests |
| DRAGON render instances | `src/app_render/build_instances.rs:223` | Mentions in-flight projectile sprites |
| GUARDWH impact effects | `src/sim/combat/mod.rs` | Emits warhead `AnimList` effects; details not audited here |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `rulesmd.ini [AAHeatSeeker2]` | verified | `rulesmd.ini:25678-25687` | none |
| `artmd.ini [DRAGON]` | verified | `artmd.ini:14755-14760` | none |
| `rulesmd.ini [GUARDWH]` | verified | `rulesmd.ini:26902-26912` | none |
| `ObjectTypeClass__ReadINI` image and LineTrail keys | verified | Ghidra decompile, string anchors `Image`, `UseLineTrail` | none |
| `BulletTypeClass__ReadINI` Image/Shadow/Trailer/Rotates | verified | Ghidra decompile | none |
| SHP load for non-Inviso bullet image | verified | `BulletTypeClass__ReadINI`, `ObjectTypeClass__ReadINI` call to `FUN_005f9070` | exact MIX filename fallback not re-expanded |
| `ObjectClass__Reveal` LineTrail attach | verified | Ghidra decompile and prior line-trail report | none |
| `LineTrail__Constructor/Update/Draw/UpdateAndDrawAll` | verified | Ghidra decompile | none for requested scope |
| `BulletClass__AI` trailer gate | verified | Ghidra decompile and prior bullet AI report | none |
| `ObjectClass__DrawIt` dispatch | verified | Ghidra decompile and follow-up draw report | none |
| BulletClass facing-to-frame mapping | verified | `DRAGON_BULLET_DRAW_SLOT_FRAME_MAPPING_GHIDRA_REPORT.md` | none |
| BulletClass display layer | verified for DRAGON | `DRAGON_BULLET_DRAW_SLOT_FRAME_MAPPING_GHIDRA_REPORT.md` | none for AAHeatSeeker2/DRAGON |
| `BulletClass__BulletDetonation` | verified | Ghidra decompile | none |
| `WarheadTypeClass__Detonate` impact anim spawn | verified | Ghidra decompile | none for GUARDWH presentation |
| `Warhead__SelectExplosionAnim` | verified | Ghidra decompile | none |
| Impact sound source | verified from prior sound doc, not newly decompiled | `ANIMATION_SOUNDS_GHIDRA_REPORT.md` | exact sounds for each GUARDWH selected anim not enumerated |

## 8. Open Questions - Final State

[RESOLVED] OQ-DRAGON-001 - Is DRAGON loaded as the AAHeatSeeker2 visible image? Yes; `Image=DRAGON`, `Inviso` false, loader called from `BulletTypeClass__ReadINI`. Evidence: `rulesmd.ini:25686`, `BulletTypeClass__ReadINI`.

[RESOLVED] OQ-DRAGON-002 - Is `UseLineTrail` live for DRAGON? Yes; parsed on ObjectType and attached in `ObjectClass__Reveal`. Evidence: `ObjectTypeClass__ReadINI`, `ObjectClass__Reveal`, `artmd.ini:14757`.

[RESOLVED] OQ-DRAGON-003 - Does DRAGON spawn SMOKEY2 as a recurring trailer? No in stock YR; the line is commented, so the `Trailer` pointer remains null. Evidence: `artmd.ini:14756`, `BulletTypeClass__ReadINI`, `BulletClass__AI`.

[RESOLVED] OQ-DRAGON-004 - Does GUARDWH drive impact animation timing? Yes; detonation calls warhead detonate, which selects/spawns from the warhead AnimList at impact. Evidence: `BulletClass__BulletDetonation`, `WarheadTypeClass__Detonate`, `Warhead__SelectExplosionAnim`.

[RESOLVED] OQ-DRAGON-005 - Does the explosion sound come from GUARDWH directly? No dedicated warhead sound field was verified; sound comes from the selected AnimType's `StartSound=`/`Report=` if present. Evidence: `ANIMATION_SOUNDS_GHIDRA_REPORT.md:231-263`.

[RESOLVED] OQ-DRAGON-006 - Exact `Rotates=yes` facing-to-DRAGON-frame arithmetic. Follow-up report resolves BulletClass vtable `+0x114` as `0x00468090` and the rotating SHP frame formula as `frame = (28 - (((((uint16)bam >> 10) + 1) >> 1) & 0x1F)) & 31`. Evidence: `DRAGON_BULLET_DRAW_SLOT_FRAME_MAPPING_GHIDRA_REPORT.md`.

[RESOLVED] OQ-DRAGON-007 - Exact BulletClass display layer returned by `0x00468B90`. Follow-up report resolves stock non-`Flat` DRAGON layer as `3`. Evidence: `DRAGON_BULLET_DRAW_SLOT_FRAME_MAPPING_GHIDRA_REPORT.md`.

[DEFERRED] OQ-DRAGON-008 - Exact audio names for each GUARDWH-selected explosion anim. Reason: impact sound source is resolved, but enumerating every selected AnimType art entry is outside this slot. Category: out-of-scope.

## Sources

- Ghidra MCP read-only decompiles: `ObjectTypeClass__ReadINI`, `BulletTypeClass__ReadINI`, `ObjectClass__Reveal`, `ObjectClass__DrawIt`, `LineTrail__Constructor`, `LineTrail__Update`, `LineTrail__Draw`, `LineTrail__UpdateAndDrawAll`, `BulletClass__AI`, `BulletClass__BulletDetonation`, `WarheadTypeClass__Detonate`, `Warhead__SelectExplosionAnim`, `AnimClass__Constructor`.
- Ghidra string anchors: `UseLineTrail`, `Rotates`, `Shadow`.
- INI files: `ini/rulesmd.ini`, `ini/artmd.ini`, checked against base `rules.ini`/`art.ini` for the same DRAGON/AAHeatSeeker2 slice.
- Prior reports: `GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md`, `LINE_TRAIL_CLASS_GHIDRA_REPORT.md`, `BULLETTYPECLASS_GHIDRA_REPORT.md`, `BULLET_CLASS_AI_GHIDRA_REPORT.md`, `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`, `OBJECTCLASS_DRAW_LIMBO_CELLLIST.md`, `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`, `ANIMATION_SOUNDS_GHIDRA_REPORT.md`.
