# DRAGON Bullet Draw Slot / Frame Mapping - Ghidra Research Report

**Address(es):** `BulletClass` vtable `0x007E46E4`, draw body `0x00468090`, frame helper `0x00468000`, layer helper `0x00468B90`, parser `0x0046BEE0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** visible `AAHeatSeeker2` / `Image=DRAGON` BulletClass SHP draw slot, `Rotates=yes` frame mapping, layer/depth, and `Shadow=no` suppression.  
**Non-Scope:** full Tactical render pass ordering, full line-trail rasterization, voxel projectile draw path, homing physics beyond the velocity vector consumed for frame selection, and GUARDWH impact presentation.  
**Confidence:** High for vtable slot binding, parser offsets, SHP frame formula, layer helper, and shadow gate. Medium for naming of some inherited helper slots where Ghidra labels remain generic.  
**Active in YR:** Yes. This is the standard non-Inviso BulletClass object draw path used by stock `[AAHeatSeeker2] Image=DRAGON`; one scenario flag visibility branch is conditional and not required for normal stock YR.

## 1. Overview

`AAHeatSeeker2` draws as a normal `ObjectClass`-pipeline object. The primary draw dispatcher is inherited `ObjectClass::DrawIt` at BulletClass vtable `+0x104`, and the visible BulletClass SHP body is the BulletClass-specific vtable `+0x114` target at `0x00468090`.

`[DRAGON] Rotates=yes` is not a rules-projectile key. `BulletTypeClass::ReadINI @ 0x0046BEE0` reads it from the `Image=` art section and stores the inverse at `BulletType+0x2A1`. In the BulletClass frame helper, `+0x2A1 == 0` enables velocity-derived 32-frame selection; `+0x2A1 != 0` leaves the frame at zero unless `AnimLow` or `AnimHigh` forces the bullet animation-frame override.

## 2. Key Offsets

| Class | Offset | Meaning | Active in YR | Evidence |
|---|---:|---|---|---|
| BulletClass vtable | `+0x104` | inherited `ObjectClass::DrawIt` dispatcher | Yes | vtable memory `0x007E47E8 -> 0x005F4B10` |
| BulletClass vtable | `+0x114` | BulletClass visible SHP/voxel draw body | Yes | vtable memory `0x007E47F8 -> 0x00468090` |
| BulletClass vtable | `+0x1E8` | frame helper | Yes | vtable memory `0x007E48CC -> 0x00468000`; assembly at `0x00468000` |
| BulletClass vtable | `+0x78` | display layer helper | Yes | vtable memory `0x007E475C -> 0x00468B90`; assembly at `0x00468B90` |
| BulletClass | `+0x9C/+0xA0/+0xA4` | world location used for draw/shadow/height | Yes | `0x00468090` |
| BulletClass | `+0xAC` | `BulletTypeClass*` | Yes | `0x00468090`, `0x00468000`, `0x00468B90` |
| BulletClass | `+0xE8/+0xF0` | velocity X/Y, consumed for rotating SHP frame | Yes | `0x00468000` |
| BulletClass | `+0x12C` | runtime animation frame override | Conditional | `0x00468000`; only used when `AnimLow` or `AnimHigh` is nonzero |
| BulletTypeClass | `+0x29A` | `Shadow` | Yes | parser `0x0046BEE0`; draw branch `0x00468308-0x00468316` |
| BulletTypeClass | `+0x29E` | `Inviso` draw skip | Yes | draw skip `0x004680E2-0x004680F0` |
| BulletTypeClass | `+0x2A1` | inverted `Rotates`; zero means rotate | Yes | parser `0x0046BEE0`; frame helper `0x0046800C-0x00468014` |
| BulletTypeClass | `+0x2F4/+0x2F5` | `AnimLow`/`AnimHigh` frame override gate | Conditional | frame helper `0x0046806A-0x00468086` |
| BulletTypeClass | `+0x2F7` | `Flat`; layer selector | Conditional | parser `0x0046BEE0`; layer helper `0x00468B90` |

## 3. Core Logic

### 3.1 Draw-slot binding

The BulletClass primary vtable at `0x007E46E4` binds:

| Slot | Function | Finding | Active in YR |
|---:|---:|---|---|
| `+0x104` | `0x005F4B10` | inherited `ObjectClass::DrawIt`; visibility/redraw/limbo gate, coordinate conversion, then dispatches `+0x114` | Yes |
| `+0x114` | `0x00468090` | BulletClass body draw; handles scenario visibility gate, Inviso skip, voxel branch, SHP shadow, palette, and main SHP draw | Yes |
| `+0x1E8` | `0x00468000` | frame helper called by `0x00468090` before `CC_Draw_Shape` | Yes |

Evidence: Ghidra `read_memory` at `0x007E47E8` returned dwords `0x005F4B10`, `0x00466660`, `0x00426440`, `0x00426450`, `0x00468090`; `get_assembly_context` at `0x00468000`; decompile of `0x00468090`.

### 3.2 `Rotates=yes` consumption and frame formula

The parser writes:

```text
0x2A1 = !ReadBool(art_image_section, "Rotates", current +0x2A1 == 0)
```

So stock `[DRAGON] Rotates=yes` stores `BulletType+0x2A1 = 0`.

The frame helper starts with `EAX = 0`, then checks `BulletType+0x2A1`:

```text
if BulletType.NotRotates != 0:
    frame = 0
else:
    bam16 = ftol((atan2(-VelocityY, VelocityX) - pi/2) * (-32768/pi))
    index = ((((uint16)bam16 >> 10) + 1) >> 1) & 0x1F
    frame = DWORD_TABLE_007F4890[index]

if BulletType.AnimLow != 0 or BulletType.AnimHigh != 0:
    frame = BulletClass.AnimFrame
```

`DWORD_TABLE_007F4890` is:

```text
index:  0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15
frame: 28 27 26 25 24 23 22 21 20 19 18 17 16 15 14 13

index: 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31
frame: 12 11 10  9  8  7  6  5  4  3  2  1  0 31 30 29
```

Equivalently, for non-animated rotating bullets:

```text
frame = (28 - index) & 31
```

Active in YR: Yes. Evidence: `BulletTypeClass::ReadINI @ 0x0046BEE0`; assembly `0x00468000-0x0046805D`; lookup table memory `0x007F4890`; stock `artmd.ini:14760`.

### 3.3 DRAGON animation override status

The velocity-derived frame is overridden only if `AnimLow` or `AnimHigh` is nonzero. Stock `[DRAGON]` in `artmd.ini` has `Rotates=yes` but no `AnimLow`/`AnimHigh`, so `AAHeatSeeker2` uses velocity-derived DRAGON frames, not `BulletClass+0x12C` animation cycling.

Active in YR: Yes for DRAGON. Evidence: frame-helper branch at `0x0046806A-0x00468086`; `artmd.ini:14755-14760`.

### 3.4 Main SHP draw and depth

For non-Inviso, non-voxel bullets, `0x00468090` resolves the SHP through vtable `+0x6C`, calls the frame helper at `+0x1E8`, then calls `CC_Draw_Shape`.

The main sprite draw uses:

- frame from `+0x1E8`
- draw flags `0x2E00`
- palette selected from normal/unit palette, `AnimPalette`, or firer palette
- depth/z argument `-0x1E - Tactical__AdjustForZ(vtable+0x1D0())`

For BulletClass in this vtable, `+0x1D0 -> 0x005F5F30`, which returns `Location.Z` (`this+0xA4`). Therefore the main DRAGON SHP uses a `-30 - AdjustForZ(Location.Z)` draw-depth adjustment.

Active in YR: Yes. Evidence: `0x00468090`, assembly `0x004683D7-0x0046841D`, vtable memory `0x007E48B4 -> 0x005F5F30`.

### 3.5 Display layer

BulletClass vtable `+0x78` points at `0x00468B90`. That helper reads `BulletType+0x2F7` (`Flat`):

```text
if BulletType.Flat != 0:
    return 1
else:
    return 3
```

Stock `[DRAGON]` does not set `Flat=yes`, so `AAHeatSeeker2` DRAGON bullets return display layer `3` (the Air/high-altitude layer in existing render pipeline docs).

Active in YR: Yes. Evidence: assembly `0x00468B90-0x00468BA5`; `BulletTypeClass::ReadINI @ 0x0046BEE0` reads art `Flat` into `+0x2F7`; `artmd.ini:14755-14760`; `OBJECTCLASS_DRAW_LIMBO_CELLLIST.md` layer table.

### 3.6 Shadow suppression

The shadow draw branch in `0x00468090` requires both:

```text
height_above_ground > 0
BulletType.Shadow != 0
```

Only then does it call `CC_Draw_Shape` with flags `0x2601` for the shadow pass. Stock `[AAHeatSeeker2] Shadow=no` is read into `BulletType+0x29A = 0`, so the shadow branch is skipped even when the missile is above ground and visible.

Active in YR: Yes. Evidence: parser `0x0046BEE0` reads `Shadow` into `+0x29A`; draw branch `0x00468304-0x00468316`; `rulesmd.ini:25680`.

### 3.7 Visibility and TS/special gating

Before the SHP path, `0x00468090` has a scenario flag check using `g_ScenarioClass_Instance & 0x1000` and a visibility helper. This can early-out before drawing. This branch is conditional; it is not needed to explain stock standard YR DRAGON missile drawing when the flag is unset. The normal stock YR path continues through the Inviso/voxel/SHP branches described above.

Active in YR: Conditional. Evidence: draw-body entry branch `0x0046809C-0x004680DC`; project rule caution that TS-style fog/special visibility must not be assumed as standard YR default.

## 4. INI Keys

| File | Section | Key | Stock value | Binary effect | Active in YR |
|---|---|---|---|---|---|
| `rulesmd.ini:25678-25686` | `[AAHeatSeeker2]` | `Image` | `DRAGON` | selects art image section and SHP | Yes |
| `rulesmd.ini:25680` | `[AAHeatSeeker2]` | `Shadow` | `no` | writes `BulletType+0x29A = 0`; skips shadow branch | Yes |
| `artmd.ini:14760` | `[DRAGON]` | `Rotates` | `yes` | writes `BulletType+0x2A1 = 0`; enables velocity-derived frame mapping | Yes |
| `artmd.ini:14755-14760` | `[DRAGON]` | `Flat` | absent/default false | layer helper returns `3` | Yes |
| `artmd.ini:14755-14760` | `[DRAGON]` | `AnimLow/AnimHigh` | absent/default zero | no animation-frame override; use velocity frame | Yes |
| `artmd.ini:14757` | `[DRAGON]` | `UseLineTrail` | `yes` | line-trail path, not the SHP frame slot | Yes; out of scope here |

## 5. Integration Points

| Integration point | Verified behavior | Active in YR |
|---|---|---|
| `ObjectClass::DrawIt @ 0x005F4B10` | inherited BulletClass vtable `+0x104`; handles redraw/limbo/coords and calls `+0x114` | Yes |
| `BulletClass draw body @ 0x00468090` | actual visible bullet SHP drawing for non-Inviso, non-voxel bullets | Yes |
| `BulletClass frame helper @ 0x00468000` | maps velocity vector and inverted `Rotates` byte to SHP frame | Yes |
| `BulletClass layer helper @ 0x00468B90` | `Flat ? 1 : 3`; DRAGON defaults to layer 3 | Yes |
| `BulletTypeClass::ReadINI @ 0x0046BEE0` | reads `Image`, `Shadow`, art `Rotates`, art `Flat`, and animation frame keys | Yes |

## 6. Current Rust Implementation Status

Observed, not modified:

| Area | Path | Status |
|---|---|---|
| Projectile parse | `src/rules/projectile_type.rs:62-77`, `187-197` | Parses `Shadow` and stores inverted art `Rotates` consistently with binary naming, but the field name can still mislead consumers. |
| App visual frame | `src/app_fire_effects.rs:216-224`, `268-287` | Current render-only projectile visual picks a frame from origin-to-destination direction and frame count; it does not use live BulletClass velocity or the binary lookup table. |
| App projectile lifetime | `src/app_instances/overlays.rs:617-665` | Draws app-owned visual interpolation, not sim-owned BulletClass state. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| BulletClass vtable `+0x104` | verified | `read_memory 0x007E47E8 -> 0x005F4B10`; decompile `0x005F4B10` | none |
| BulletClass vtable `+0x114` | verified | `read_memory 0x007E47F8 -> 0x00468090`; decompile `0x00468090` | none |
| BulletClass vtable `+0x1E8` frame helper | verified | `read_memory 0x007E48CC -> 0x00468000`; assembly context `0x00468000` | none |
| BulletClass vtable `+0x78` layer helper | verified | `read_memory 0x007E475C -> 0x00468B90`; assembly context `0x00468B90` | none |
| `Rotates=yes` parser | verified | `BulletTypeClass::ReadINI @ 0x0046BEE0`; `artmd.ini:14760` | none |
| Velocity-to-frame formula | verified | assembly `0x00468016-0x0046805D`; table `0x007F4890` | none |
| Animation frame override | verified | assembly `0x0046806A-0x00468086`; no DRAGON keys in `artmd.ini:14755-14760` | none |
| `Shadow=no` suppression | verified | parser `0x0046BEE0`; branch `0x00468304-0x00468316`; `rulesmd.ini:25680` | none |
| Layer/depth | verified | layer helper `0x00468B90`; main draw assembly `0x004683D7-0x0046841D`; vtable `+0x1D0 -> 0x005F5F30` | broader render-pass ordering out of scope |
| Scenario flag visibility early-out | touched-not-exhausted | branch `0x0046809C-0x004680DC` | exact scenario flag semantics out of scope; treat as conditional |
| Voxel projectile branch | deferred | `0x00468104` tests `BulletType+0x236` | out-of-scope; DRAGON is SHP path |

## 8. Open Questions - Final State

[RESOLVED] OQ-DRAGON-FRAME-001 - Which vtable slot draws the visible BulletClass SHP? `+0x104` is inherited `ObjectClass::DrawIt`; it dispatches to BulletClass `+0x114 = 0x00468090` for the actual visible bullet body. Evidence: vtable memory `0x007E47E8/0x007E47F8`, decompile `0x005F4B10`, decompile `0x00468090`.

[RESOLVED] OQ-DRAGON-FRAME-002 - How is inverted `Rotates` consumed? `BulletType+0x2A1 != 0` skips velocity mapping and leaves frame zero; `+0x2A1 == 0` computes a velocity-derived lookup-table frame. Evidence: parser `0x0046BEE0`; assembly `0x0046800C-0x00468064`.

[RESOLVED] OQ-DRAGON-FRAME-003 - How does facing map to SHP frame? The draw helper derives a BAM angle from `atan2(-VelocityY, VelocityX)`, converts to a rounded 32-sector index, then indexes table `0x007F4890`; for non-animated DRAGON, `frame = (28 - index) & 31`. Evidence: assembly `0x00468016-0x0046805D`; table bytes at `0x007F4890`.

[RESOLVED] OQ-DRAGON-FRAME-004 - Which display layer is used? BulletClass `GetLayer @ 0x00468B90` returns `1` for `Flat=yes`, otherwise `3`; DRAGON has no `Flat=yes`, so AAHeatSeeker2 uses layer `3`. Evidence: assembly `0x00468B90-0x00468BA5`; `artmd.ini:14755-14760`.

[RESOLVED] OQ-DRAGON-FRAME-005 - How does `Shadow=no` suppress shadow? The shadow pass is gated by `height > 0 && BulletType+0x29A != 0`; `Shadow=no` writes `+0x29A = 0`, so no shadow shape draw occurs. Evidence: parser `0x0046BEE0`; draw branch `0x00468304-0x00468316`; `rulesmd.ini:25680`.

[DEFERRED] OQ-DRAGON-FRAME-006 - What exactly does scenario flag `0x1000` mean in this draw path? Reason: branch is not needed for stock DRAGON frame mapping and may be TS/special visibility behavior. Category: out-of-scope. Next step: separate visibility/fog investigation if needed.

[DEFERRED] OQ-DRAGON-FRAME-007 - How does the voxel projectile branch map orientation for `BulletType+0x236` projectiles? Reason: DRAGON uses SHP, not the voxel branch. Category: out-of-scope.

## Sources

- Ghidra MCP read-only:
  - `read_memory 0x007E46E4` BulletClass vtable.
  - `decompile_function 0x005F4B10` inherited ObjectClass draw dispatcher.
  - `decompile_function 0x00468090` BulletClass draw body.
  - `get_assembly_context 0x00468000` BulletClass frame helper.
  - `get_assembly_context 0x00468B90` BulletClass layer helper.
  - `decompile_function 0x0046BEE0` BulletTypeClass parser.
  - `read_memory 0x007F4890` frame lookup table.
- INI:
  - `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:25678-25687`
  - `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:14755-14760`
- Prior reports:
  - `GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md`
  - `DRAGON_RENDER_AND_GUARDWH_IMPACT_PRESENTATION_GHIDRA_REPORT.md`
  - `BULLETTYPECLASS_GHIDRA_REPORT.md`
  - `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`
  - `OBJECTCLASS_DRAW_LIMBO_CELLLIST.md`
- Rust scan only:
  - `src/rules/projectile_type.rs:62-77`, `187-197`
  - `src/app_fire_effects.rs:216-224`, `268-287`
  - `src/app_instances/overlays.rs:617-665`
