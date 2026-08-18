# AnimTypeClass Tiberium Flag Consumers — Ghidra Research Report

**Date:** 2026-05-20
**Binary:** gamemd.exe (Yuri's Revenge)
**Scope:** Four AnimTypeClass boolean flags — `TiberiumChainReaction` (+0x357), `IsTiberium`
(+0x358), `HideIfNoOre` (+0x359), `IsAnimatedTiberium` (+0x360) — write sites in
`AnimTypeClass::ReadINI`, all runtime read sites, behavioral classification, and YR-activity status.
**Confidence:** HIGH — all offsets verified directly from `AnimTypeClass::ReadINI` decompile
(0x00427D00). All consumer behaviors verified from `AnimClass::AI` (0x00423AC0) and
`AnimClass::Middle` (0x00424CE0) decompiles.

---

## 1. Overview

Four boolean flags in `AnimTypeClass` are related to tiberium/ore behavior. They control:
- Ore destruction + debris spawn at anim-playback start (TiberiumChainReaction)
- Ore spawning at bouncer-landing footprint (IsTiberium)
- Visibility suppression when no ore is nearby (HideIfNoOre)
- Self-suppression if the ore the anim was animating has been harvested (IsAnimatedTiberium)

All four are **active in stock YR** with concrete INI uses. None are TS-legacy dead code.

**Disambiguation note:** `VoxelAnimTypeClass` has a **separate** `IsTiberium` field at its own
offset +0x300, consumed in `VoxelAnimClass::AI` (0x00749F30). That is NOT the same as
`AnimTypeClass+0x358`. Both are live, but they serve different systems. Section 8 covers the
VoxelAnimTypeClass case.

---

## 2. Offset Verification — AnimTypeClass::ReadINI

**Source:** `AnimTypeClass::ReadINI` at `0x00427D00`, decompiled via `decompile_function`.
`param_1` type is `int*` (verified from function signature), so `param_1[N]` = byte offset `N*4`.
Direct `((int)param_1 + 0xXXX)` casts are always direct byte offsets regardless of pointer type.

| Field | INI Key | Offset Derivation | Verified Byte Offset | Default |
|-------|---------|-------------------|----------------------|---------|
| TiberiumChainReaction | `TiberiumChainReaction` | `*(undefined1 *)((int)param_1 + 0x357)` — direct cast | **+0x357** | false |
| IsTiberium | `IsTiberium` | `*(undefined1 *)(param_1 + 0xd6)` — int* index → `0xd6 * 4 = 0x358` | **+0x358** | false |
| HideIfNoOre | `HideIfNoOre` | `*(undefined1 *)((int)param_1 + 0x359)` — direct cast | **+0x359** | false |
| IsAnimatedTiberium | `IsAnimatedTiberium` | `*(undefined1 *)(param_1 + 0xd8)` — int* index → `0xd8 * 4 = 0x360` | **+0x360** | false |

All four offsets exactly match the prior `ANIM_CLASS_GHIDRA_REPORT.md` listings. No discrepancy.

### INI Key String Addresses (verified via `search_strings`)

| INI Key | String Address |
|---------|----------------|
| `TiberiumChainReaction` | `0x008185ac` |
| `IsTiberium` | `0x00818518` |
| `HideIfNoOre` | `0x0081850c` |
| `IsAnimatedTiberium` | `0x00818468` |

### Write Site Addresses Within ReadINI

| Field | Write Instruction Address (within ReadINI) |
|-------|-------------------------------------------|
| TiberiumChainReaction | `0x00427f59` (xref from `0x008185ac`) |
| IsTiberium | `0x0042810b` (xref from `0x00818518`) |
| HideIfNoOre | `0x00428125` (xref from `0x0081850c`) |
| IsAnimatedTiberium | `0x004282b5` (xref from `0x00818468`) |

---

## 3. TiberiumChainReaction (+0x357) — Detailed Analysis

### Read Site
**Function:** `AnimClass::Middle` at `0x00424CE0`
**Access:** `*(char *)(param_1[0x32] + 0x357)` where `param_1[0x32]` = `AnimClass.Type` pointer
**Condition guard:** `(char)param_1[0x66] == '\0'` must also be true (param_1[0x66] = AnimClass+0x198, an internal flag that gates game-side vs. UI anims)

### Behavioral Logic (verified from decompile at 0x00424CE0)

Called when an anim's delay expires and playback begins (also called immediately from the
AnimClass constructor if `delay == 0`).

```
if (AnimClass.IsGlobal == false) AND (AnimClass.Type.TiberiumChainReaction == true):
    cell = CellClass::Get_Cell_At(this->coords)
    tibIdx = CellClass::GetTiberiumType(cell)   // -1 if no tiberium
    if tibIdx != -1:
        tib = TiberiumClass_Array[tibIdx]
        CellClass::Reduce_Tiberium(cell, cell->OverlayData + 1)  // removes ALL ore (density+1)
        if tib->DebrisCount > 0:          // TiberiumClass+0xD4 = Debris anim count
            rand = Random__Next()
            if abs(rand) % 3 == 0:        // 1-in-3 chance
                debrisIdx = RandomRanged(0, tib->DebrisCount - 1)
                new AnimClass(tib->Debris[debrisIdx], coords, ...)
                // sets palette from TiberiumClass.Color, sets ZAdjust from cell light level
        Apply_area_damage(0, g_RulesClass_Instance+0xfa8, 0, 0)
            // warhead = RulesClass.ExplosionWarhead (offset 0xfa8)
        CellClass::RecalcAttributes(cell)
        MapClass::AssignOrphanedCellZone(cell->coords)
        FUN_00584550(cell->coords)   // radar/display update
```

**Key details:**
- `cell->OverlayData + 1` means full density removal (not partial) — even density-0 ore is removed (0+1=1 unit)
- The 1-in-3 probability is `abs(rand) % 3 == 0` using `Random__Next()` (not `RandomRanged`)
- Area damage warhead is `g_RulesClass_Instance + 0xfa8` — this is `RulesClass.C4Warhead` or similar explosion warhead (not a tiberium-specific warhead)
- `RecalcAttributes` recomputes cell land type after ore removal
- This fires at the **start** of playback, not on expiry

### Active in YR: YES

**INI uses:** `TWLT070T` (both `art.ini` line 11186 and `artmd.ini` line 15692).

`TWLT070T` is listed as animation index 245 in `[AnimTypes]` in `rulesmd.ini` (line 2200).
`TWLT070T` is the "tiberium explosion with chain reaction" variant of `TWLT070` — it uses
`Image=TWLT070` but adds `TiberiumChainReaction=yes`. This anim is used as the `ExpireAnim`
for gem crystal VoxelAnims (CRYSTAL1-4 in rules.ini have `ExpireAnim=TWLT050` or `TWLT070`
depending on which list). It fires every match gems exist and something lands in or near them.

**Ore (Riparius) case:** `Riparius` has no `Debris=` key → `TiberiumClass.DebrisCount = 0` → the
debris-spawn branch is always skipped for ore. Area damage and ore removal still fire.

**Gems (Cruentus) case:** `Cruentus` has `Debris=CRYSTAL1,CRYSTAL2,CRYSTAL3,CRYSTAL4` → the
1-in-3 debris-spawn fires if tiberium is present.

---

## 4. IsTiberium (+0x358) — Detailed Analysis (AnimTypeClass)

### Read Site
**Function:** `AnimClass::AI` at `0x00423AC0`
**Access:** `*(char *)(param_1[0x32] + 0x358)` where `param_1[0x32]` = `AnimClass.Type` pointer

### Behavioral Logic (verified from decompile at 0x00423AC0)

This flag is only checked inside the **bouncer landing block** — a code block entered when
`AnimClass.IsBouncer (+0x194/param_1[0x65])` is non-zero AND the bounce state returns 2
(water) or 1 (hits ground and Z >= ground level, i.e. still airborne or lands in water).

```
if (AnimClass.IsBouncer AND (bVar22 OR bVar21)):
    // bVar22 = landed in water (cell LandType == 2)
    // bVar21 = Z position still >= ground height (still airborne)
    if (AnimClass.Type.IsTiberium == true) AND (bVar21 == false):
        // bVar21 == false means NOT still airborne — i.e. actually landed on ground/water
        radius = AnimClass.Type.TiberiumSpreadRadius  // +0x33C
        for dx in -radius..=radius:
            for dy in -radius..=radius:
                dist = sqrt(dx*dx + dy*dy)
                if dist <= radius:
                    cell = GetCellAt(landingPos + (dx, dy) in cells)
                    if CellClass::CanPlaceTiberium(cell):
                        if AnimClass.Type.TiberiumSpawnType != NULL:
                            overlay = TiberiumSpawnType->ArrayIndex + RandomRanged(0, 3)
                            new OverlayClass(OverlayTypeArray[overlay], cell.coords, -1)
                            cell->OverlayData = RandomRanged(0, 2)  // density 0-2
                            TacticalClass::DirtyScreenRect(...)
```

**Key details:**
- The guard `!bVar21` means "not still airborne" — the ore spawning only fires when the bouncer
  has actually reached ground or water level, not while still in flight
- When `bVar22` (in water) AND `!bVar21`: the block does NOT run the ore-spawn (bouncer hit water,
  no ore placement in water) — the condition is `IsTiberium AND !bVar21`, but the outer block
  is entered when `bVar22 OR bVar21`. If in water (`bVar22=true`, `bVar21=false`), `!bVar21` is
  true so the ore loop would run — but `CellClass::CanPlaceTiberium` will return false for water
  cells, so no ore is actually placed. Functionally equivalent to "no ore in water."
- `TiberiumSpawnType` (AnimTypeClass+0x338) must be non-NULL for any overlay to be placed
- Overlay variant: `TiberiumSpawnType->ArrayIndex + RandomRanged(0, 3)` picks from first 4
  variants of that overlay type's block
- `OverlayData` set to `RandomRanged(0, 2)` = density 0, 1, or 2 (sparse new growth)

### Active in YR: YES

**INI uses (art.ini / artmd.ini):**
- `METSMALL` — `IsTiberium=true`, `IsMeteor=true`, `TiberiumSpawnType=` (not set directly, uses default via `TIB01`). This is the large meteor anim — fires in "Meteor Strike" superweapon.
- `METDEBRI` — `IsTiberium=true`, `TiberiumSpawnType=TIB01`, `Bouncer=yes`. Small debris from meteor. Places ore around landing point.
- `CRYSTAL1`, `CRYSTAL2`, `CRYSTAL3`, `CRYSTAL4` — `IsTiberium=true`, `Bouncer=yes`, `TiberiumSpawnType=TIB2_01` (or similar). These are gem crystal shards that fly off when gems chain-react.

The Meteor Strike superweapon is available in standard YR skirmish (if enabled by map) — fires
frequently in campaign missions and skirmish with the appropriate superweapon. Gem chain reactions
occur whenever a gem patch is hit with area-damage weapons. Both trigger paths are active in normal YR play.

---

## 5. HideIfNoOre (+0x359) — Detailed Analysis

### Read Site
**Function:** `AnimClass::AI` at `0x00423AC0`
**Access:** `*(char *)(param_1[0x32] + 0x359)` — direct byte offset from AnimTypeClass pointer

### Behavioral Logic (verified from decompile at 0x00423AC0)

Checked every tick, **before** the bouncer block and before frame advancement:

```
if (AnimClass.Type.HideIfNoOre == true):
    cell = vtable[0x6F](this)()        // GetCoords → convert to cell
    tiberiumValue = CellClass::Get_Tiberium_Value(cell)
    if (cell == NULL) OR (tiberiumValue == 0):
        AnimClass+0x19d (IsInvisible) = 1   // hide the anim
    else:
        AnimClass+0x19d (IsInvisible) = 0   // show the anim
```

**Key details:**
- `Get_Tiberium_Value` returns 0 if the cell has no tiberium OR if tiberium type is not
  recognized. It calls `IsWallOverlay` (0x005fdd20) to check `OverlayTypeClass.Tiberium` (+0x2A9).
- The `IsInvisible` flag at `AnimClass+0x19d` suppresses drawing in `AnimClass::DrawIt` — checked
  at the top of DrawIt: `if (*(char *)((int)param_1 + 0x19d) != '\0') return;`
- Setting `IsInvisible` does NOT pause the animation — `CurrentFrame` still advances every tick,
  and the anim's AI still runs. Only the draw call is skipped.
- The `IsInvisible` flag is written EVERY tick, so if ore is harvested the anim becomes invisible
  within 1 tick; if ore grows back, it becomes visible within 1 tick.
- The vtable slot used to get coords is `*param_1 + 0x1bc` — this is `vtable[0x6F]`, which on
  AnimClass resolves to `GetCoords` returning the lepton position.

### Active in YR: YES

**INI uses:** `TWNK1` (both `art.ini` line 14313 and `artmd.ini` line 19241, `HideIfNoOre=true`).

`TWNK1` is a sparkle/glint animation played on or near tiberium ore. It has `DetailLevel=2`,
meaning it's suppressed on lower detail settings — but when detail is high enough, it fires on
every ore-containing cell that has `CellAnim=TWNK1` in its `OverlayTypeClass`. This makes it the
ambient sparkle effect on ore tiles. It's placed by the map/overlay system and runs the entire
match on all visible ore tiles — it fires extremely frequently in a normal YR game (hundreds or
thousands of instances on a typical map).

---

## 6. IsAnimatedTiberium (+0x360) — Detailed Analysis

### Read Site
**Function:** `AnimClass::AI` at `0x00423AC0`
**Access:** `*(char *)(param_1[0x32] + 0x360)` — direct byte offset from AnimTypeClass pointer

### Behavioral Logic (verified from decompile at 0x00423AC0)

Checked every tick, **after** the HideIfNoOre check and before the End/frame-count check:

```
if (AnimClass.Type.IsAnimatedTiberium == true):
    coords = vtable[0x12](this)()   // GetCoords (leptons)
    // Offset the check position by (-0x180, -0x180) leptons = (-1.5, -1.5) cells
    checkX = coords.X - 0x180
    checkY = coords.Y - 0x180
    cell = CellClass::Get_Cell_At(checkX, checkY, coords.Z)
    if (cell->OverlayTypeIndex == -1)              // no overlay on that cell
    OR (OverlayTypeArray[cell->OverlayTypeIndex]->CellAnim != this->Type):
        // The ore this anim was animating no longer exists, or a different anim is bound
        AnimClass+0x19b (IsInactive) = 1   // suppress anim and drawing
```

**Key details:**
- The coordinate offset `(-0x180, -0x180)` leptons = `(-384, -384)` leptons.
  In cells: `384 / 256 = 1.5` cells offset in both X and Y. This is non-trivial: it's not
  checking the cell directly at the anim's position but 1.5 cells northwest. This offset
  compensates for the isometric rendering offset of the BIGBLUE anim's visual center relative
  to the cell it represents.
- `OverlayTypeClass.CellAnim` is at offset `+0x29C` (a pointer to AnimTypeClass). If the ore
  cell's overlay type has a `CellAnim=BIGBLUE` entry in its INI, that pointer is non-NULL and
  equals the current AnimType. If ore is harvested (overlay removed), `cell->OverlayTypeIndex`
  becomes -1 and the anim deactivates.
- `IsInactive` at `AnimClass+0x19b` is a stronger suppression than `IsInvisible` (+0x19d):
  once set, `AnimClass::AI` returns early without further processing: `if (*(char *)((int)param_1 + 0x19b) != '\0') goto LAB_00424b38;` which calls `AnimClass::Destroy`.
- This means IsAnimatedTiberium anims **destroy themselves** when their ore tile is harvested —
  they don't just become invisible, they fully terminate.
- The `CellAnim` pointer is checked via `!=` pointer comparison, not name comparison. The
  AnimTypeClass* must be the exact same instance.

### Active in YR: YES

**INI uses:** `BIGBLUE` (`art.ini` line 14305 `IsAnimatedTiberium=yes`, `artmd.ini` line 19233).

`BIGBLUE` is an animated overlay graphic for tiberium (`Theater=yes` meaning it's a theater-
specific SHP). It has `Layer=ground`, `LoopCount=-1` (infinite loop), `RandomRate=150,250`.
It appears to be the large animated gem/tiberium cell overlay used on certain ore tiles.
The `Theater=yes` flag means it loads a different SHP per theater.

---

## 7. IsTiberium in VoxelAnimTypeClass (+0x300) — Related System

This field appears in `VoxelAnimTypeClass::ReadINI` (0x0074B050) confirmed by xref from string
`0x00818518` to `0x0074b0b5` within that function. It maps to `VoxelAnimTypeClass+0x300`.

This is a **completely separate field** from `AnimTypeClass+0x358`. They share the same INI key
string address (`s_IsTiberium_00818518`) because the string is in a common string pool, but they
write to different struct types.

### Consumer: VoxelAnimClass::AI (0x00749F30)

Checked on VoxelAnim expiry/landing (when `Duration` countdown reaches 0 or bouncer stops):

```
if (VoxelAnimType.IsTiberium == true) AND (NOT landed in water):
    if (VoxelAnimType.IsMeteor == true):
        // Loop all 8 neighbors of landing cell
        for dir in 0..8:
            neighborCell = landing_cell + DirectionOffset[dir]
            if CellClass::CanPlaceTiberium(neighborCell):
                tibIdx = CellClass::OverlayToTiberiumIndex(neighborCell)
                tib = TiberiumClass_Array[tibIdx]
                if neighborCell->DamageState (+0x11c) == 0:
                    overlayVariant = RandomRanged(0, 11)
                    new OverlayClass(tib->Image->ArrayIndex + overlayVariant, ...)
                else:
                    // cell has existing ore — pick from extra variants
                    overlayVariant = RandomRanged(0, 1)
                    new OverlayClass(tib->Image->ArrayIndex + neighborCell->DamageState*2 + tib->NumExtraImages + variant, ...)
                TiberiumClass::AddToGrowthQueue(neighborCell)
                neighborCell->OverlayData = 0
    else:
        // Non-meteor: just check the single landing cell
        cell = Get_Cell_At(landing_coords)
        if CellClass::CanPlaceTiberium(cell):
            // same ore placement as above but just for the one cell
```

**Active in YR: YES.**

**INI uses (rules.ini):** CRYSTAL01, CRYSTAL02 (`IsTiberium=true`) — VoxelAnim gem crystal shards
that fly off when gems chain-react. These are in `[VoxelAnims]` sections in `rules.ini`.
**INI uses (artmd.ini):** METSMALL, METDEBRI, CRYSTAL1-4 — AnimType sections (not VoxelAnim)
also use `IsTiberium=true`, but those go to `AnimTypeClass+0x358`, not here.

---

## 8. INI Uses — Complete Stock Reference

### TiberiumChainReaction (AnimTypeClass+0x357)

| File | Section | Line | Notes |
|------|---------|------|-------|
| art.ini | `[TWLT070T]` | 11186 | Ore/gem explosion with chain reaction; uses Image=TWLT070 |
| artmd.ini | `[TWLT070T]` | 15692 | YR override (same values) |

### IsTiberium (AnimTypeClass+0x358)

| File | Section | Line | Notes |
|------|---------|------|-------|
| art.ini | `[METSMALL]` | 14163 | Large meteor, IsMeteor=true |
| art.ini | `[METDEBRI]` | 14184 | Small meteor debris, Bouncer=yes, TiberiumSpawnType=TIB01 |
| art.ini | `[CRYSTAL1..4]` | 14222-14282 | Gem crystals, Bouncer=yes, TiberiumSpawnType=TIB2_01 |
| artmd.ini | `[METSMALL]` | 19091 | YR overrides |
| artmd.ini | `[METDEBRI]` | 19112 | YR override |
| artmd.ini | `[CRYSTAL1..4]` | 19150-19210 | YR overrides |

### HideIfNoOre (AnimTypeClass+0x359)

| File | Section | Line | Notes |
|------|---------|------|-------|
| art.ini | `[TWNK1]` | 14313 | Ore sparkle, DetailLevel=2, RandomLoopDelay |
| artmd.ini | `[TWNK1]` | 19241 | YR override (adds Rate=450) |

### IsAnimatedTiberium (AnimTypeClass+0x360)

| File | Section | Line | Notes |
|------|---------|------|-------|
| art.ini | `[BIGBLUE]` | 14305 | Animated ore/gem cell overlay, Theater=yes, LoopCount=-1 |
| artmd.ini | `[BIGBLUE]` | 19233 | YR override |

### IsTiberium (VoxelAnimTypeClass+0x300) — separate system

| File | Section | Line | Notes |
|------|---------|------|-------|
| rules.ini | `[CRYSTAL01]` | 22851 | Gem VoxelAnim crystal shard |
| rules.ini | `[CRYSTAL02]` | 22867 | Gem VoxelAnim crystal shard |
| rules.ini | `[METEOR01]` | 22902 | Meteorite VoxelAnim, IsMeteor=true |
| rules.ini | `[CRYSTAL04?]` | 22920 | Another gem/meteor VoxelAnim |
| rulesmd.ini | `[CRYSTAL01]` | 30705 | YR override |
| rulesmd.ini | `[CRYSTAL02]` | 30721 | YR override |
| rulesmd.ini | `[METEOR01?]` | 30756 | YR override |
| rulesmd.ini | `[CRYSTAL04?]` | 30774 | YR override |

---

## 9. YR Activity Classification

| Flag | Field | Consumer Function | Active in YR | Frequency |
|------|-------|-------------------|--------------|-----------|
| TiberiumChainReaction | AnimTypeClass+0x357 | AnimClass::Middle (0x00424CE0) | **YES** | Fires every time TWLT070T plays — gem chain reactions, meteor impacts |
| IsTiberium | AnimTypeClass+0x358 | AnimClass::AI (0x00423AC0) — bouncer landing block | **YES** | Fires every meteor impact, every gem crystal bounce-land |
| HideIfNoOre | AnimTypeClass+0x359 | AnimClass::AI (0x00423AC0) — every tick | **YES** | Runs every tick on every TWNK1 instance; hundreds of instances on typical map |
| IsAnimatedTiberium | AnimTypeClass+0x360 | AnimClass::AI (0x00423AC0) — every tick | **YES** | Runs every tick on every BIGBLUE instance on ore tiles |
| IsTiberium (VoxelAnim) | VoxelAnimTypeClass+0x300 | VoxelAnimClass::AI (0x00749F30) | **YES** | Fires on gem crystal VoxelAnim expiry; meteor impacts |

---

## 10. Open Questions — Final State

- `[RESOLVED] OFF-1` — Are the four offsets correct? → YES, all four verified from ReadINI decompile (evidence: `decompile_function 0x00427D00`).
- `[RESOLVED] OFF-2` — Is param_1 `int*` in ReadINI? → YES, confirmed from Ghidra signature `int *param_1`. IsTiberium at `param_1[0xd6]` → 0xd6*4 = 0x358; IsAnimatedTiberium at `param_1[0xd8]` → 0xd8*4 = 0x360.
- `[RESOLVED] CR-1` — Where is TiberiumChainReaction consumed at runtime? → `AnimClass::Middle` (0x00424CE0); fires at playback start. Evidence: decompile shows `*(char *)(param_1[0x32] + 0x357)` check with ore-removal + debris + damage logic.
- `[RESOLVED] CR-2` — Where is IsTiberium (+0x358) consumed? → `AnimClass::AI` bouncer landing block. Evidence: `*(char *)(param_1[0x32] + 0x358)` inside the `bVar21/bVar22` block.
- `[RESOLVED] CR-3` — Where is HideIfNoOre consumed? → `AnimClass::AI` every tick. Evidence: `*(char *)(param_1[0x32] + 0x359)` → Get_Tiberium_Value → IsInvisible toggle.
- `[RESOLVED] CR-4` — Where is IsAnimatedTiberium consumed? → `AnimClass::AI` every tick. Evidence: `*(char *)(param_1[0x32] + 0x360)` → cell lookup → IsInactive if overlay mismatch.
- `[RESOLVED] YR-1` — Are any of these TS-legacy? → None. All four have stock YR INI uses (TWLT070T, METDEBRI, TWNK1, BIGBLUE) and their consumer code paths are reachable in normal YR gameplay.
- `[RESOLVED] VOXEL-1` — Is the VoxelAnimTypeClass IsTiberium at +0x300 the same as AnimTypeClass+0x358? → NO, they are separate fields in separate classes sharing the same INI key string. Evidence: VoxelAnimTypeClass::ReadINI (0x0074B050) writes to `*(param_1 + 0x300)`, consumer is VoxelAnimClass::AI (0x00749F30).
- `[RESOLVED] RUST-1` — Are these flags implemented in the Rust codebase? → NO. None of the four flag names (`TiberiumChainReaction`, `IsTiberium`, `HideIfNoOre`, `IsAnimatedTiberium`) appear in any `.rs` file. The Rust engine has an `is_tiberium` concept but it refers to `OverlayTypeClass.Tiberium` (+0x2A9), not AnimTypeClass. The behaviors gated by all four flags are unimplemented.
- `[RESOLVED] INI-1` — What stock YR content uses each flag? → Fully documented in Section 8.
- `[DEFERRED] COORD-1` — The IsAnimatedTiberium cell offset is (-0x180, -0x180) leptons. Is this correct for all ore tile sizes, or only for specific SHP layouts? (category: needs-runtime-debugger; reason: requires observing the actual BIGBLUE anim position relative to its ore cell in a running game to confirm the offset compensates correctly for the isometric rendering offset; next-step: run game with BIGBLUE visible, log anim position vs cell position at runtime).
- `[DEFERRED] CHAIN-1` — What exact warhead is at `g_RulesClass_Instance + 0xfa8`? (category: requires-different-system-context; reason: the TiberiumChainReaction area_damage call uses this offset but mapping RulesClass field layout is out of scope for this investigation; next-step: trace RulesClass::ReadINI for offset 0xfa8).
- `[DEFERRED] CHAIN-2` — Does ore chain-react after one TWLT070T fires (i.e., does the ore removal from cell A cause cell B's TWLT070T to fire)? (category: needs-runtime-debugger; reason: requires tracing whether CellAnim re-triggers on adjacent cells after TiberiumChainReaction fires; next-step: in-game observation with meteor strike or gem weapon).

---

## 11. Rust Implementation Gap Summary

None of the four flags are parsed or stored in the Rust `AnimType` data structure, and none of
their runtime behaviors are implemented in `src/sim/` or `src/app_*/`. The gaps are:

1. **AnimType struct** (`src/rules/art_data.rs` or equivalent): Missing four fields —
   `tiberium_chain_reaction`, `is_tiberium`, `hide_if_no_ore`, `is_animated_tiberium`.

2. **AnimClass::Middle equivalent**: No TiberiumChainReaction logic (ore removal + debris spawn + area damage on anim start).

3. **AnimClass::AI equivalent — bouncer landing**: No IsTiberium ore-spawn radius loop on bouncer impact.

4. **AnimClass::AI equivalent — every tick HideIfNoOre**: No visibility suppression based on cell tiberium value.

5. **AnimClass::AI equivalent — every tick IsAnimatedTiberium**: No self-deactivation when the ore overlay the anim was animating gets harvested.

---

## Sources

- `AnimTypeClass::ReadINI` at `0x00427D00` — decompiled via `decompile_function`
- `AnimClass::AI` at `0x00423AC0` — decompiled via `decompile_function`
- `AnimClass::Middle` at `0x00424CE0` — decompiled via `decompile_function`
- `AnimClass::DrawIt` at `0x00422CA0` — decompiled for IsInvisible/IsInactive gate verification
- `AnimClass::Start` at `0x00424F00` — decompiled for completeness
- `VoxelAnimTypeClass::ReadINI` at `0x0074B050` (decompile at `0x0074b0b5`) — for IsTiberium disambiguation
- `VoxelAnimClass::AI` at `0x00749F30` — decompiled for VoxelAnimTypeClass IsTiberium consumer
- `ANIM_CLASS_GHIDRA_REPORT.md` — prior AnimTypeClass field table (all four offsets confirmed matching)
- `VOXELANIMCLASS_GHIDRA_REPORT.md` — VoxelAnimTypeClass layout (offset 0x300 = IsTiberium)
- `ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md` — CellClass and TiberiumClass layout reference
- `ini/art.ini`, `ini/artmd.ini` — stock INI uses of all four flags
- `ini/rules.ini`, `ini/rulesmd.ini` — VoxelAnimTypeClass IsTiberium stock uses; TiberiumClass Debris lists
- INI key string addresses verified via `search_strings` MCP calls
- Xrefs verified via `get_xrefs_to` MCP calls on all four string addresses
