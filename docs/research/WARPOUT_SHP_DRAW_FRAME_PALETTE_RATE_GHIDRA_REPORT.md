# WARPOUT SHP Draw Frame Palette Rate - Ghidra Research Report

**Address(es):** `0x00427B50`, `0x00427D00`, `0x00427530`, `0x00421EA0`, `0x00423AC0`, `0x00422CA0`, `0x00424CE0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Player-visible `WARPOUT` `AnimClass` rendering for verified TeleportLocomotion rows: stock rules/art source, active lifecycle fields, first visible frame, frame advance/destruction timing, palette/remap flags, and `DrawIt` path for constructor row flags `0x600`.  
**Non-Scope:** TeleportLocomotion row producer census, VXL/SHP techno warp translucency, temporal `WarpAttachClass`, `WARPAWAY`, `WARPIN`, runtime framebuffer capture, and full `CC_Draw_Shape` blitter leaf pixel math.  
**Confidence:** High for binary reader/lifecycle/draw-path behavior and the verified 21-frame retail asset.  
**Active in YR:** Yes, conditional on active TeleportLocomotion rows already verified to construct `[General] WarpOut`.

Working notes gate:
- `Target question`: What exact player-visible `WARPOUT` `AnimClass` frame, timing, palette, and draw-path behavior follows the verified TeleportLocomotion constructor row `delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0`?
- `Non-goals`: Do not redo TeleportLocomotion row census; do not analyze VXL/SHP unit warp translucency; do not analyze temporal weapon visuals.
- `Evidence needed to mark COMPLETE`: INI/default source plus binary reader address for metadata; decompile plus disassembly range for constructor, first-AI guard, AI frame boundary, Middle, and DrawIt flag/depth path; Rust surface scan for current deltas.
- `Stop conditions`: Stop once `WARPOUT` art metadata, active native lifecycle, first visible frame, draw flags/palette path, and remaining asset/framecount uncertainty are all resolved or explicitly deferred.

## 1. Overview

TeleportLocomotion rows create ordinary free `AnimClass` objects whose type is `[General] WarpOut`, stock `WARPOUT`. Stock `[WARPOUT]` is a flat, translucent, ground-layer animation with `Rate=120` and `YSortAdjust=-64`, but it does not define `Start`, `End`, `LoopStart`, `LoopEnd`, `LoopCount`, `Next`, `AltPalette`, `AnimPalette`, `YDrawOffset`, or `ZAdjust`.

Stock metadata omits `End` and `LoopEnd`, but those values do not remain zero. After reading `Shadow=false`, `AnimTypeClass::ReadINI @ 0x00427D00` calls the image loader at `0x00427B50`; the loader fills zero `End` from the signed SHP header count and then copies it to zero `LoopEnd`. Retail `WARPOUT.SHP` has 21 frames, so the active bounds are `End=21`, `LoopEnd=21`, and drawable frames are `0..20`. The constructor still calls `Middle()` immediately for `delay=0`, and the first AI visit still clears the constructor first-AI guard before later rate-driven advancement.

## 2. Class Layout / Key Offsets

| Offset | Owner | Meaning for stock `WARPOUT` | Value/effect | Active in YR | Evidence |
|---|---|---|---|---|---|
| `+0x2B0` | `AnimTypeClass` | stored frame delay | `Rate=120` -> `900 / 120 = 7` logic frames | Yes | `AnimTypeClass::ReadINI @ 0x00427D00`; `ini/artmd.ini:15485..15491` |
| `+0x2B4` | `AnimTypeClass` | `Start` | absent -> constructor default `0` | Yes | `0x00427530`; `0x00427D00` |
| `+0x2B8` | `AnimTypeClass` | `LoopStart` | absent -> `0` | Yes | `0x00427530`; `0x00427D00` |
| `+0x2BC` | `AnimTypeClass` | `LoopEnd` | absent -> loader copies resolved `End=21` | Yes | `0x00427B50`; `0x00427D00` |
| `+0x2C0` | `AnimTypeClass` | `End` | absent -> loader reads signed SHP count `21` because current value is zero and `Shadow=false` | Yes | `0x00427B50`; `0x00427D00` |
| `+0x2C4` | `AnimTypeClass` | `LoopCount` | absent -> `0`; constructor clamp makes remaining loop byte `1` | Yes | `0x00427530`; `0x00421EA0` |
| `+0x2C8` | `AnimTypeClass` | `Next` | absent -> null | Yes | `0x00427D00`; `ini/artmd.ini:15485..15492` |
| `+0x340` | `AnimTypeClass` | `YSortAdjust` | `-64`; copied to `AnimClass+0x104` | Yes | `0x00427D00`; `0x00421EA0`; `ini/artmd.ini:15491` |
| `+0x344` | `AnimTypeClass` | `YDrawOffset` | absent -> `0`; used in DrawIt screen/depth expression | Yes | `0x00427D00`; `0x00422CA0` |
| `+0x348` | `AnimTypeClass` | type `ZAdjust` fallback | absent -> `0`; row `zAdjust=0` selects this fallback | Yes | `0x00421EA0`; `0x00422CA0` |
| `+0x361` | `AnimTypeClass` | `AltPalette` | absent -> false | Yes | `0x00427D00`; `ini/artmd.ini:15485..15492` |
| `+0x364` | `AnimTypeClass` | `Layer` | `ground` parsed as native ground layer | Yes | `0x00427D00`; `ini/artmd.ini:15488` |
| `+0x369` | `AnimTypeClass` | `Flat` | `yes`; selects flat draw branch | Yes | `0x00427D00`; `0x00422CA0`; `ini/artmd.ini:15487` |
| `+0x36A` | `AnimTypeClass` | `Translucent` | `yes`; ORs translucency low bits from lifetime/frame state | Yes | `0x00427D00`; `0x00422CA0`; `ini/artmd.ini:15489` |
| `+0x190` | `AnimClass` | constructor draw flags | row `0x600`; `DrawIt` later ORs `0x2000` | Yes | `0x00421EA0`; `0x00422CA0`; Teleport row census |
| `+0x19C` | `AnimClass` | first-AI guard | constructor sets `1`; first AI clears and returns | Yes | `0x00421EA0`; `0x00423AC0` |

## 3. Core Logic

### 3.1 INI source and reader

Stock YR and base RA2 agree:

```text
[General]
WarpOut=WARPOUT;WAKE2

[WARPOUT]
;Theater=yes
Flat=yes
Layer=ground
Translucent=yes
Rate=120
YSortAdjust=-64
;Report=ChronoLegionTeleport
```

Active in YR: Yes. `[General] WarpOut` is read into `RulesClass+0x33C` by the active rules reader and TeleportLocomotion rows use that field. The stock art section is present in both `ini/art.ini:10979..10986` and `ini/artmd.ini:15485..15492`.

`AnimTypeClass::ReadINI @ 0x00427D00` parses the active art keys:

- `Rate`: `Rate > 0` stores integer `900 / Rate`; for stock `120`, stored delay is `7`.
- `Start`, `LoopStart`, and `LoopCount` remain at their constructor defaults. After `Shadow=false` is read, loader `0x00427B50` resolves omitted `End` to the full signed SHP count `21` and omitted `LoopEnd` to `21`; the later absent INI keys preserve those loader values.
- `AltPalette`: absent, so `AnimType+0x361` remains false.
- `Flat`: true, `Translucent`: true, `Layer`: ground, `YSortAdjust`: `-64`.

Handoff-critical evidence: decompile `0x00427B50` and `0x00427D00`; disassembly range inspected `0x00427E40..0x00428034`; retail `WARPOUT.SHP` probe; INI lines above. The independent `AnimType+0x298 = raw_count/2` cache is not the resolved `End` field.

### 3.2 Constructor and first visible frame

Active in YR: Yes. TeleportLocomotion rows construct a normal free `AnimClass` with `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.

`AnimClass::Constructor @ 0x00421EA0` sets:

- `AnimClass+0xAC CurrentFrame = 0`.
- `AnimClass+0xC4 FrameStep = 1`.
- `AnimClass+0xC8 Type = WARPOUT`.
- `AnimClass+0x184 Delay = constructor delay`, so `0`.
- `AnimClass+0x190 DrawFlags = 0x600`.
- `AnimClass+0x100 ZAdjust = type.ZAdjust` when constructor `zAdjust==0`, so stock `0`.
- `AnimClass+0x104 = type.YSortAdjust`, stock `-64`.
- `AnimClass+0x19C first-AI guard = 1`.
- loop remaining byte is `(byte)type.LoopCount * (byte)ctorLoop`, clamped to at least `1`; stock absent `LoopCount=0` and ctor loop `1` produce remaining `1`.
- because `Delay==0`, constructor calls `AnimClass::Middle()` immediately.

`AnimClass::Middle @ 0x00424CE0` calls display/mark update, then checks start/report sound. Stock `WARPOUT` has `Report` commented out and no `StartSound`; no start sound is played. It then calls `AnimClass::Start()` because type `Start=0`.

First visible frame: `DrawIt` computes the shape frame as `AnimType.Start + AnimClass.CurrentFrame`. Stock `Start=0` and constructor `CurrentFrame=0`, so the first visible frame is frame `0`. Nothing in `Middle()` advances it.

Handoff-critical evidence: decompile `0x00421EA0`, `0x00424CE0`, `0x00422CA0`; disassembly range inspected `0x00422385..0x00422464`.

### 3.3 First AI guard and active sequence length

Active in YR: Yes for every newly constructed `AnimClass` reaching AI. In `AnimClass::AI @ 0x00423AC0`, the first-AI guard runs before delay countdown and before `CDTimerClass__GetTimeRemaining()` frame advancement:

```text
if (AnimClass+0x19C != 0) {
    AnimClass+0x19C = 0;
    return;
}
```

Thus a `delay=0` `WARPOUT` starts immediately for display, but the first AI visit cannot advance or destroy it. The frame timer was initialized from stock `Rate=120` to `7` logic frames; later timer expiries advance through the loader-resolved sequence. With `End=LoopEnd=21`, frames `0..20` are the active retail body range; the next boundary exhausts the constructor-clamped single loop and destroys the anim because `Next` is null.

Handoff-critical evidence: `AnimClass::AI @ 0x00423AC0` decompile and disassembly range inspected `0x004242F0..0x0042477F`; constructor/read defaults above.

### 3.4 Draw flags, flat path, translucency, and depth

Active in YR: Yes for visible stock `WARPOUT` rows.

`AnimClass::DrawIt @ 0x00422CA0` starts from `AnimClass+0x190`, so the Teleport row's `0x600` survives into draw. Since low bit `0x1` is not set, DrawIt ORs `0x800`. Since stock `Translucent=yes`, the normal translucency branch ORs low `0x2`, `0x4`, or `0x6` based on lifetime/frame state. With the resolved 21-frame range, source alpha is `1.0` on frames `0..4`, `0.75` on `5..8`, `0.5` on `9..12`, and `0.25` on `13..20`; exact destination RGB565 word math remains a separate blitter concern.

For non-tiled stock `Flat=yes`, DrawIt uses the flat branch:

- calls the shape's draw setup helper,
- computes screen Y as `screen_y + AnimType.YDrawOffset`, stock `0`,
- computes the depth/z argument as `(AnimType.YDrawOffset + AnimClass.ZAdjust) - Tactical__AdjustForZ(anim_z) - 3`,
- ORs `0x2000` into the flags before `CC_Draw_Shape`.

For the verified constructor row, the material-key family is therefore based on row flags `0x600`, native-added `0x800`, native-added `0x2000`, and translucency low bits. A future Rust material should preserve the expanded native draw flags rather than only carrying a boolean `translucent`.

Handoff-critical evidence: `AnimClass::DrawIt @ 0x00422CA0` decompile; draw branch around `Type+0x369`, `Type+0x36A`, `AnimClass+0x190`, and `CC_Draw_Shape`; disassembly range inspected through the function.

### 3.5 Palette/remap behavior

Active in YR: Yes. Stock `WARPOUT` does not set `AltPalette=yes`, `AnimPalette=yes`, or `Palette=...`; `AnimTypeClass::ReadINI` only proves `AltPalette` is false for this stock section. In DrawIt, when no explicit `AnimClass+0xD4` palette/remap is set and `AltPalette` is false, the path derives the color/remap context from cell/theater state; if `AltPalette` were true it would use the player's color scheme pointer instead. Stock `WARPOUT` is therefore not unit-remap/house-color drawn.

Remaining pixel-level palette uncertainty: this report does not close the exact `CC_Draw_Shape` palette pointer and translucency blitter leaf for a framebuffer sample. That needs a software blit fixture or runtime capture.

## 4. INI Keys

| Key | Section | Stock value | Native field/effect | Active in YR | Evidence |
|---|---|---|---|---|---|
| `WarpOut` | `[General]` | `WARPOUT;WAKE2` -> `WARPOUT` | `RulesClass+0x33C`, TeleportLocomotion row type | Yes | `ini/rulesmd.ini:549`; prior row census |
| `Flat` | `[WARPOUT]` | `yes` | `AnimType+0x369=1`, flat draw branch | Yes | `ini/artmd.ini:15487`; `0x00427D00`; `0x00422CA0` |
| `Layer` | `[WARPOUT]` | `ground` | `AnimType+0x364` ground layer | Yes | `ini/artmd.ini:15488`; `0x00427D00` |
| `Translucent` | `[WARPOUT]` | `yes` | `AnimType+0x36A=1`, native translucency flag family | Yes | `ini/artmd.ini:15489`; `0x00427D00`; `0x00422CA0` |
| `Rate` | `[WARPOUT]` | `120` | `AnimType+0x2B0=7` logic frames | Yes | `ini/artmd.ini:15490`; `0x00427D00` |
| `YSortAdjust` | `[WARPOUT]` | `-64` | `AnimType+0x340`, copied to `AnimClass+0x104` | Yes | `ini/artmd.ini:15491`; `0x00427D00`; `0x00421EA0` |
| `Start` | `[WARPOUT]` | absent | default `0` | Yes | `0x00427530`; `0x00427D00` |
| `End` | `[WARPOUT]` | absent | loader-populated from signed SHP count: `21` | Yes | `0x00427B50`; `0x00427D00`; retail asset probe |
| `LoopStart/LoopEnd/LoopCount` | `[WARPOUT]` | absent | `0/21/0`; remaining loop clamps to `1` | Yes | `0x00427B50`; `0x00427D00`; `0x00421EA0` |
| `AltPalette` | `[WARPOUT]` | absent | default false; no player remap palette path | Yes | `0x00427530`; `0x00427D00` |
| `Report` | `[WARPOUT]` | commented out | no start/report sound | Yes | `ini/artmd.ini:15492`; `0x00424CE0` |

## 5. Integration Points

`TELEPORTLOCOMOTION_GENERIC_VISUAL_ROW_CENSUS_GHIDRA_REPORT.md` owns the row producers and proves four TeleportLocomotion constructor sites use `RulesClass+0x33C`. This report consumes those rows and follows only the resulting `AnimClass` metadata/render path.

Runtime order in the active object system:

1. TeleportLocomotion allocates and constructs `AnimClass`.
2. `ObjectClass::Reveal` submits it to display/logic membership.
3. Constructor calls `Middle()` immediately for `delay=0`.
4. The first render pass that sees the object draws frame 0.
5. First `AnimClass::AI` clears the first-AI guard and returns.
6. Later AI visits count down the `Rate=120 -> 7` timer and advance through frames `1..20`; the boundary after the 21-frame range exhausts the single loop.

The exact render-vs-AI ordering inside the same newly-spawned global frame was not runtime-captured here; the binary state makes frame 0 the first drawable frame either way.

## 6. Current Rust Implementation Status

Current Rust is a partial bridge, not native `AnimClass`:

- `src/sim/movement/teleport_movement.rs` preserves the constructor row fields and spawns a `WorldEffect`.
- `src/rules/ruleset.rs::resolve_art_rates` converts `[WARPOUT] Rate=120` to milliseconds via the art-rate helper.
- `src/sim/components.rs::WorldEffect` advances by app/render milliseconds and `total_frames`, not by the complete native `AnimClass::AI` first-guard/loop scheduler.
- `src/rules/art_data.rs` parses the generic runtime metadata (`End`, `LoopCount`, `Flat`, `Translucent`, `YSortAdjust`, etc.), but `WorldEffect::from_anim_spawn` does not consume it.

The frame-bound sourcing delta is closed: the sprite-atlas handoff now applies the verified Shadow-only half rule, registers WARPOUT frames `0..20`, publishes count `21` to simulation, and drives the matching progressive-alpha boundaries. Complete generic `AnimClass` first-AI/timer/loop scheduling and exact destination RGB565 blending remain separate residuals.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `[General] WarpOut` source | verified | `ini/rulesmd.ini:549`; row census report | none |
| `[WARPOUT]` stock art metadata | verified | `ini/artmd.ini:15485..15492`; `ini/art.ini:10979..10986` | none |
| `AnimTypeClass` defaults | verified | `0x00427530` decompile | none |
| `AnimTypeClass::ReadINI` relevant keys | verified | `0x00427D00` decompile; disassembly `0x00427E40..0x00428034` | none |
| Constructor delay 0 and loop clamp | verified | `0x00421EA0` decompile; disassembly `0x00422385..0x00422464` | none |
| `Middle()` immediate start/no stock sound | verified | `0x00424CE0` decompile; stock commented `Report` | none |
| First-AI guard and boundary destruction | verified | `0x00423AC0` decompile; disassembly `0x004242F0..0x0042477F` | exact live frame counter sample deferred |
| DrawIt flat/translucent/depth path | verified | `0x00422CA0` decompile | exact `CC_Draw_Shape` blitter leaf pixel sample deferred |
| Raw `WARPOUT.SHP` frame count | verified | retail asset probe: 21 frames, indices `0..20` | none |
| Rust bridge status | verified | `src/sim/movement/teleport_movement.rs`, `src/sim/components.rs`, `src/rules/art_data.rs`, `src/rules/ruleset.rs` | native generic AnimClass implementation |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-WARPOUT-001 - Which stock rules key and type back the row? -> [General] WarpOut -> WARPOUT.` (evidence: `ini/rulesmd.ini:549`; `TELEPORTLOCOMOTION_GENERIC_VISUAL_ROW_CENSUS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-WARPOUT-002 - Which art keys are active on stock WARPOUT? -> Flat=yes, Layer=ground, Translucent=yes, Rate=120, YSortAdjust=-64 only; Report is commented.` (evidence: `ini/artmd.ini:15485..15492`)
- `[RESOLVED] OQ-WARPOUT-003 - Where is Rate read and how is it converted? -> `AnimType+0x2B0 = 900 / Rate`; stock stores `7`.` (evidence: `0x00427D00`)
- `[RESOLVED] OQ-WARPOUT-004 - Does absent End resolve from the SHP? -> Yes. With a non-null SHP and current `End==0`, loader `0x00427B50` copies the signed header count and halves it only for `Shadow=true`; WARPOUT resolves to 21.` (evidence: `0x00427B50`; `0x00427D00`; retail asset probe)
- `[RESOLVED] OQ-WARPOUT-005 - What is the first visible frame? -> frame `Start + CurrentFrame = 0`.` (evidence: `0x00421EA0`; `0x00422CA0`)
- `[RESOLVED] OQ-WARPOUT-006 - Does `delay=0` still wait for first AI before display start? -> No; constructor calls `Middle()` immediately.` (evidence: `0x00421EA0`; `0x00424CE0`)
- `[RESOLVED] OQ-WARPOUT-007 - What does first AI do? -> Clears `AnimClass+0x19C` and returns before countdown/advance.` (evidence: `0x00423AC0`)
- `[RESOLVED] OQ-WARPOUT-008 - Does stock WARPOUT normally show frame 1? -> Yes; loader-resolved `End=LoopEnd=21` admits frames `0..20`.` (evidence: `0x00427B50`; `0x00427D00`; `0x00423AC0`)
- `[RESOLVED] OQ-WARPOUT-009 - Does stock WARPOUT use AltPalette/unit remap? -> No; `AltPalette` absent and default false.` (evidence: `ini/artmd.ini:15485..15492`; `0x00427D00`; `0x00422CA0`)
- `[RESOLVED] OQ-WARPOUT-010 - Which draw path does Flat=yes take? -> Non-tiled flat branch in `AnimClass::DrawIt`.` (evidence: `0x00422CA0`)
- `[RESOLVED] OQ-WARPOUT-011 - How are row flags expanded? -> starts from `0x600`, adds `0x800` when low bit 1 is clear, adds translucency low bits, and ORs `0x2000` before `CC_Draw_Shape`.` (evidence: `0x00422CA0`)
- `[RESOLVED] OQ-WARPOUT-012 - Which depth expression is used for stock flat branch? -> `(YDrawOffset + instance ZAdjust) - Tactical__AdjustForZ(z) - 3`; stock YDrawOffset/ZAdjust are 0.` (evidence: `0x00422CA0`)
- `[RESOLVED] OQ-WARPOUT-013 - Is Report/StartSound active? -> No for stock WARPOUT; Report line is commented and no StartSound key exists.` (evidence: `ini/artmd.ini:15492`; `0x00424CE0`)
- `[RESOLVED] OQ-WARPOUT-014 - What current Rust surface consumes the row? -> `TeleportVisuals` -> `WorldEffect::from_anim_spawn`, not native `AnimClass`.` (evidence: `src/sim/movement/teleport_movement.rs`; `src/sim/components.rs`)
- `[RESOLVED] OQ-WARPOUT-015 - What is the raw retail `WARPOUT.SHP` frame count? -> 21, with valid indices 0..20.` (evidence: retail asset probe)
- `[DEFERRED] OQ-WARPOUT-016 - What exact 16-bit framebuffer pixels result from the stock DrawIt translucency branch?` (category: `needs-runtime-debugger`; reason: `CC_Draw_Shape` blitter leaf and destination-dependent blend were out of scope; next-step-if-pursued: software blit fixture over known terrain)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | TeleportLocomotion constructor site, consumed from prior census | active teleport row | `[General] WarpOut` -> `WARPOUT` | row coords, center subcell, row z | none yet | Yes/Conditional | creates free `AnimClass` |
| 2 | `AnimClass::Constructor @ 0x00421EA0` | `delay=0` | type `WARPOUT`, `CurrentFrame=0` | `ObjectClass::Reveal(coords,0)` | no owner remap | Yes | register/reveal/Middle |
| 3 | `AnimClass::Middle @ 0x00424CE0` | immediate because delay zero | frame unchanged | current coords | no stock sound | Yes | starts anim without frame advance |
| 4 | `AnimClass::DrawIt @ 0x00422CA0` | not hidden, detail gates pass, `Flat=yes`, `Translucent=yes` | `WARPOUT` frames `0..20` | screen coords plus `YDrawOffset=0`; depth `(0+0)-AdjustForZ-3` | no AltPalette; native cell/theater palette/remap path | Yes | visible flat translucent ground overlay |
| 5 | first `AnimClass::AI @ 0x00423AC0` | `AnimClass+0x19C=1` | no frame advance | n/a | n/a | Yes | clears guard and returns |
| 6 | later `AnimClass::AI @ 0x00423AC0` | rate timer expires | advances through loader-resolved frames `1..20`, then exhausts at the sequence boundary | n/a | n/a | Conditional on reaching timer expiry | lifecycle advance/removal |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `WARPOUT` | Yes | Yes | Yes, frames `0..20` under stock metadata plus loader resolution | No | No | Yes | Yes | No | `Rules+0x33C`; art section; `0x00427B50`; DrawIt |
| `WARPIN` | Yes | No for this target | No | No | No | No | Yes elsewhere | Yes for TeleportLocomotion rows | prior row census |
| `WARPAWAY` | Yes | No for this target | No | No | No | No | Yes elsewhere | Yes for TeleportLocomotion rows | prior row census |
| `CHRONOSK` | Yes | No for this target | No | No | No | No | Yes elsewhere | Yes for TeleportLocomotion rows | prior row census |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock Teleport `WARPOUT` starts visible on frame 0 immediately after constructor `delay=0`; first AI clears guard and cannot advance. | `0x00421EA0`, `0x00424CE0`, `0x00423AC0`, `0x00422CA0`; `ini/artmd.ini:15485..15492` | Missing: `WorldEffect` has no native first-AI guard or `Middle()` semantics | `src/sim/components.rs`, `src/sim/movement/teleport_movement.rs`, future generic AnimClass runtime | Distinguish construction/Middle visibility from first AI advancement; first drawable frame is 0 | Spawn a teleport row and assert the first rendered frame is 0, with no advancement on the first AI visit | `warpout_delay_zero_middle_draws_frame0_before_first_ai_advancement`; do not delay first visibility until the first tick |
| Stock `WARPOUT` omits `End`, but loader `0x00427B50` resolves it to all 21 SHP frames because `Shadow=false`; `LoopEnd` copies 21. | `0x00427B50`, `0x00427D00`, `0x00423AC0`; retail asset probe | Closed for frame availability/count and progressive-alpha boundaries; complete generic AnimClass scheduler remains residual | `src/render/sprite_atlas.rs`, `src/sim/components.rs::WorldEffect` | Keep unshadowed non-scheduler count 21 and atlas keys `0..20`; halve only `Shadow=true` non-scheduler assets | WARPOUT publishes count 21 and alpha boundaries at frames 4/5, 8/9, 12/13 | do not reuse the independent unconditional half-count cache at `AnimType+0x298` as `End` |
| Draw flags for stock flat `WARPOUT` start from row `0x600`, then native DrawIt adds `0x800`, translucency low bits, and `0x2000`; flat branch depth uses `YDrawOffset + ZAdjust - AdjustForZ - 3`. | `0x00422CA0`; constructor row census; stock `Flat=yes`, `Translucent=yes` | Missing: `WorldEffect` stores only boolean translucency and render path lacks native material/depth key | `src/app_instances/*`, `src/render/batch.rs`, `src/render/sprite_voxel_shader.wgsl`, future AnimClass render bridge | Preserve native draw-flag material family and flat depth expression for `WARPOUT` | A `WARPOUT` row batches with expanded native flags and depth equal to stock formula over a known z/terrain sample | `warpout_flat_drawit_expands_0600_to_native_translucent_shape_flags`; do not render as plain RGBA alpha with generic Y sort |

### Stale Docs / Follow-up Docs

- Any downstream document that repeats the earlier "omitted End stays zero / frame 0 only" conclusion is stale. Replace it with the verified loader order: `Shadow` is read, loader `0x00427B50` resolves `End=LoopEnd=21`, and explicit later `End=`/`LoopEnd=` keys would override that value.

## Negative Facts / Do Not Do

- Do not treat `WarpOut=WARPOUT;WAKE2` as two visual assets. Active in YR: No; `;WAKE2` is comment text in standard INI parsing.
- Do not leave absent stock `End` at zero. Active in YR: the image loader fills it when current `End==0`; the gate is not `End==-1`.
- Do not halve WARPOUT's 21-frame SHP. Active in YR: only `Shadow=true` halves loader-derived `End`; stock WARPOUT omits Shadow.
- Do not use unit `AltPalette`/house remap for stock `WARPOUT`. Active in YR: No; `AltPalette` is absent/default false.
- Do not collapse native DrawIt flags to `translucent=true`. Active in YR: No; `0x600` is expanded through native flag and flat/translucency paths.

## Remaining Uncertainty

- Exact 16-bit pixel output of the final `CC_Draw_Shape` translucency branch over a known destination remains deferred to a software blit/runtime capture.
- Exact render-vs-AI order in the same global frame as constructor was not runtime-sampled. Binary state still proves first drawable frame is frame 0.

## Sources

- Ghidra decompiled/read-only: `AnimTypeClass::Constructor @ 0x00427530`; `AnimTypeClass::ReadINI @ 0x00427D00`; `AnimClass::Constructor @ 0x00421EA0`; `AnimClass::Middle @ 0x00424CE0`; `AnimClass::AI @ 0x00423AC0`; `AnimClass::DrawIt @ 0x00422CA0`.
- Ghidra disassembly ranges inspected: `0x00427E40..0x00428034`; `0x00422385..0x00422464`; `0x004242F0..0x0042477F`.
- INI checked: `ini/rulesmd.ini:549`; `ini/rules.ini:541`; `ini/artmd.ini:15485..15492`; `ini/art.ini:10979..10986`.
- Prior report consumed: `docs/research/TELEPORTLOCOMOTION_GENERIC_VISUAL_ROW_CENSUS_GHIDRA_REPORT.md`.
- Rust scanned: `src/sim/movement/teleport_movement.rs`; `src/sim/components.rs`; `src/rules/ruleset.rs`; `src/rules/art_data.rs`; `src/sim/world/mod.rs`.
