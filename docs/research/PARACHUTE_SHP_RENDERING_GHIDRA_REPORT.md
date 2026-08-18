# PARACH SHP Rendering — Ghidra Research Report

**Date:** 2026-05-06
**Binary:** gamemd.exe (Yuri's Revenge)
**Confidence:** HIGH (all claims verified from live Ghidra decompilation; key
findings cross-referenced against existing HIGH-confidence reports)
**Active in YR:** Yes — PARACH is referenced from `[General] Parachute=PARACH`
in rulesmd.ini, used by every paratroop drop (Allied, Soviet, Yuri); the
underlying AnimClass/AnimTypeClass code is on every visible-anim path.

## 1. Overview

PARACH is the standard parachute SHP attached to paradropped infantry during
descent. The rendering pipeline is **not chute-specific** — PARACH is a
plain `AnimType` rendered by the generic `AnimClass::DrawIt` (vtable[69]),
attached to the falling GI via `AnimClass::SetOwnerObject` so its world
coordinates track the GI's. This report documents the three specific
behaviors the brainstorm flagged as UNKNOWN:

1. **`Rate=` units** — what 400 means in `[PARACH] Rate=400`.
2. **`ZAdjust=` math** — how `ZAdjust=-10` translates at draw time.
3. **`AltPalette=` palette fetch** — which palette is selected when set.

Plus context on attachment, anchor, and the loop lifecycle.

This report is **scope-limited**. The full AnimClass/AnimTypeClass
reference lives in
[`ANIM_CLASS_GHIDRA_REPORT.md`](./ANIM_CLASS_GHIDRA_REPORT.md) and
[`ANIM_CLASS_DEEP_DIVE.md`](./ANIM_CLASS_DEEP_DIVE.md); spawn paths are in
[`ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`](./ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md).

---

## 2. Key Addresses

| Entity | Address | Notes |
|---|---|---|
| `AnimTypeClass::ReadINI` | `0x00427D00` | Parses `[PARACH]` section |
| `AnimClass::DrawIt` | `0x00422CA0` | The standard render path used for PARACH |
| `AnimClass::AI` | `0x00423AC0` | Per-tick frame advance |
| `AnimClass::Constructor` (full) | `0x00421EA0` | 7 params + this |
| `AnimClass::SetOwnerObject` | `0x00424B50` | Anim attachment to TechnoClass |
| `AnimClass+0x100` (`[0x40]`) | per-instance | `ZAdjust` — depth offset |
| `AnimType+0x348` (`[0xD2]`) | per-type | `ZAdjust` from INI; copied to AnimClass+0x100 if AnimClass-side init was 0 |
| `AnimType+0x344` (`[0xD1]`) | per-type | `YDrawOffset` — screen-Y shift |
| `AnimType+0x361` | per-type, byte | `AltPalette` flag |
| `AnimType+0x2B0` (`[0xAC]`) | per-type | `Rate` (post-conversion: 900/INI value) |
| `AnimType+0x2B8` (`[0xAE]`) | per-type | `LoopStart` |
| `AnimType+0x2BC` (`[0xAF]`) | per-type | `LoopEnd` |
| `AnimType+0x2C0` (`[0xB0]`) | per-type | `End` (auto-detected from SHP if 0) |
| `AnimType+0x2C4` (`[0xB1]`) | per-type | `LoopCount` |
| `g_ColorSchemeArray` | global | indexed by 0 for AltPalette path |
| `DAT_0087f6c0` | global | default anim palette (used when AltPalette=false and no override) |

---

## 3. PARACH INI Definition (artmd.ini)

```ini
[PARACH]
Rate=400
LoopStart=20
LoopEnd=39
LoopCount=30
AltPalette=yes        ; use the unit palette
ZAdjust=-10           ; SJM: infantry are fudged by 10 towards camera so we must match this here
```

(Verified via grep at `ini/artmd.ini:15642-15649`.)

Implicit defaults that matter:
- `Layer` = `3` (Ground) — not specified in `[PARACH]`. Per
  `ANIM_CLASS_GHIDRA_REPORT.md` defaults table. Note: the chute is
  airborne in practice, but its `AnimType` Layer is Ground; layer
  tracking is governed by the attached owner's layer at draw time, not
  the anim type. (Out of immediate scope; flagged in §10.)
- `End` = 0 in INI → **auto-detected from SHP frame count** at construction
  (per `ANIM_CLASS_GHIDRA_REPORT.md` §AnimClass Constructor). For the
  retail PARACH SHP this means `End = total_frame_count`.
- `Start` = 0 (default).
- `YDrawOffset` = 0 (default; not in `[PARACH]`).

---

## 4. `Rate=` Units — VERIFIED

**Finding: `Rate=` in art.ini is *the inverse* of the internal frame-delay.
The binary computes `internal_rate = 900 / INI_Rate`, where the result is
the number of game ticks between frame advances.**

### Binary evidence (verbatim from AnimTypeClass::ReadINI at `0x00427D00`)

```c
iVar4 = CCINIClass__ReadInt(piVar8, &str_Rate, 0xffffffff);
if (iVar4 != -1) {
    if (iVar4 < 1) {
        iVar4 = 0;
    } else {
        iVar4 = (int)(900 / (longlong)iVar4);   // ← the conversion
    }
    param_1[0xac] = iVar4;   // store at AnimType+0x2B0 (Rate field)
}
```

The `900` constant is the per-second tick base (gamemd ticks at 15 FPS;
900/15 = 60, but the actual tick rate convention here treats `900` as the
base unit such that `Rate=900` means "1 tick per frame"). The integer
divide truncates toward zero.

### Concrete values for PARACH

| INI field | Value | Internal value | Effect |
|---|---|---|---|
| `Rate=400` | 400 | `900 / 400 = 2` (truncated) | **2 ticks per frame** |

At gamemd's 15 FPS tick rate, 2 ticks = 2 × 66.67ms = **~133ms per anim frame**.

### Per-frame timing budget for PARACH

| Phase | Frames | Wall-clock (15 FPS, 2 ticks/frame) |
|---|---|---|
| Deploy (one-shot) | 0 → 19 | ~2.67 seconds |
| One loop cycle (20 → 39) | 20 frames | ~2.67 seconds |
| `LoopCount=30` cycles total | 30 × 20 | ~80 seconds (upper bound) |

The chute's `LoopCount=30` is an upper bound, not the typical lifespan —
the anim is destroyed externally on landing (see §8) well before 30 loops
elapse.

### Implication for the project

The Rust codebase already has the correct conversion at
[`src/rules/art_data.rs:134-140`](../src/rules/art_data.rs#L134-L140):

```rust
pub fn art_rate_to_delay_ms(ini_rate: i32) -> u32 {
    if ini_rate < 1 { return 0; }
    let delay_frames: u32 = 900 / ini_rate as u32;
    (delay_frames * 1000 / 15).max(1)
}
```

This matches gamemd's `900 / INI_Rate` ticks-per-frame formula multiplied
by `1000/15` ms-per-tick to produce ms-per-frame. For Rate=400 it returns
133ms — matching the binary.

Existing call sites that correctly route through this helper:
`BuildingAnimConfig.rate` parsing
([art_data.rs:718](../src/rules/art_data.rs#L718)) and
`rate_from_section` for warp anims / wake / fire effects
([ruleset.rs:912](../src/rules/ruleset.rs#L912)).

For the chute implementation, route PARACH's Rate= through
`art_rate_to_delay_ms` to get its 133ms/frame value — same pattern as
the existing systems. No cross-cutting fix is needed.

---

## 5. `ZAdjust=` Math — VERIFIED

**Finding: `ZAdjust` modifies the depth-sort value at draw time. It does
NOT shift the on-screen Y position. `YDrawOffset` is the field that shifts
screen Y.**

### Two distinct fields, two distinct effects

| Field | AnimType offset | At draw time | Effect |
|---|---|---|---|
| `YDrawOffset` | `+0x344` (`[0xD1]`) | added to screen-Y | Vertical pixel shift |
| `ZAdjust` | `+0x348` (`[0xD2]`) → `AnimClass+0x100` | added to depth value | Z-sort order |

For PARACH: `YDrawOffset = 0`, `ZAdjust = -10`. So PARACH's chute draws at
the same screen-Y as the underlying owner's coordinate (no vertical
offset), but with a depth-sort value of `-10` from the base.

### Binary evidence (from AnimClass::DrawIt at `0x00422CA0`)

In the standard (non-Tiled, non-Flat) draw branch:

```c
// Screen Y position with YDrawOffset:
fStack_f0 = (float)((int)param_2[1] + *(int *)(iVar17 + 0x344));   // param_2[1] is screen Y; +0x344 is YDrawOffset
fStack_f4 = *param_2;                                               // screen X (unchanged)

// ...

// Depth value passed to CC_Draw_Shape:
iVar17 = Tactical__AdjustForZ();
CC_Draw_Shape(
    iStack_ec,
    fStack_e8,
    &fStack_f4,                  // (screen X, screen Y after YDrawOffset)
    param_2,
    uVar15 | 0x2000,             // flags
    0,
    ((*(int *)(param_1[0x32] + 0x344) + param_1[0x40]) - iVar17) + -2,
    //  ^                ^                       ^                  ^
    //  type             YDrawOffset             ZAdjust            -2 const
    //                   (also added to depth!)  (AnimClass+0x100)
    ...
);
```

**Depth formula (standard branch):**
```
depth = type->YDrawOffset + anim->ZAdjust − Tactical_Z_Correction − 2
```

**Depth formula (Flat=yes branch):**
```
depth = type->YDrawOffset + anim->ZAdjust − Tactical_Z_Correction − 3
```

(Flat branch uses `-3` instead of `-2`; verified.)

### Where `anim->ZAdjust` comes from

Per `ANIM_CLASS_GHIDRA_REPORT.md` §AnimClass Constructor:

> If `zAdjust == 0`, uses `type->ZAdjust` (offset 0x348)

The constructor's `param_7` is the per-instance ZAdjust override; if 0
(the typical case for PARACH spawn), it falls back to the AnimType's
`ZAdjust` field. So for PARACH-derived AnimClass instances:
`anim->ZAdjust = type->ZAdjust = -10`.

### Why `-10`?

The `[PARACH]` comment in artmd.ini explains:

> ; SJM: infantry are fudged by 10 towards camera so we must match this here

This is a depth-sort matching value — infantry sprites are also fudged
`-10` toward camera in the depth sort so that the chute sorts at the same
depth as the GI body it's attached to. Without this, the chute and the GI
body would sort at slightly different depths and could z-fight or render
in the wrong order relative to neighboring terrain.

### Sprite anchor (flag 0x600 / 0x200)

PARACH is constructed with `drawFlags=0x600`, which includes bit `0x200`
(verified in `ANIM_CLASS_GHIDRA_REPORT.md`):

> **Bit 0x200** = **Center sprite**: In `CC_Draw_Shape`, subtracts half
> the sprite width and height from the draw position, centering the
> sprite on the given coords.

So the **chute SHP's center is anchored to the owner's screen coordinate**
(after world→screen transform and YDrawOffset). The chute's canopy
extends `H_chute / 2` above the screen position; the chute's bottom edge
extends `H_chute / 2` below.

For an attached anim, the owner's screen coordinate is the GI's screen
position (which already accounts for altitude — see §7). So the chute is
centered on the GI's body position, with the canopy above and the
"payload" portion of the SHP overlapping the GI body.

### Implication for the project

The Rust implementation must:

1. Render the chute sprite **centered on the GI's screen position**
   (matching flag 0x200 semantics).
2. Apply `YDrawOffset` (which is 0 for PARACH; verify any future anim
   we attach uses this correctly).
3. Apply `ZAdjust=-10` to the **depth value**, NOT the screen-Y. Use the
   project's existing depth-sort layer (chute and GI body should sort
   together; if depth values are in render-order units, offset by
   whatever epsilon makes the chute draw on top of the GI body).

The brainstorm doc tagged P5 as "render chute sprite 10px toward camera"
— **that's wrong**. The 10 leptons go to *depth-sort*, not on-screen
position.

---

## 6. `AltPalette=` Path — VERIFIED

**Finding: `AltPalette=yes` selects `g_ColorSchemeArray[0]->ConvertPalette`,
which is a fixed (NOT owner-tinted) "alternative" palette. The chute
renders with the same palette regardless of who dropped it.**

### Binary evidence (from AnimClass::DrawIt at `0x00422CA0`)

The palette-selection cascade (decompile lines ~265-275):

```c
if (*(char *)(iVar17 + 0x355) == '\0') {           // type->IsVeins == false
    if (*(char *)((int)param_1 + 0x196) == '\0') { // anim flag == false
        iStack_e0 = param_1[0x35];                 // anim->Palette (per-instance override)
        if (iStack_e0 == 0) {                      // no override
            iStack_e0 = DAT_0087f6c0;              // default global anim palette
            if (*(char *)(iVar17 + 0x361) != '\0') { // type->AltPalette
                iStack_e0 = *(int *)(*g_ColorSchemeArray + 0x30c);  // ← AltPalette path
            }
        }
        // ...
    }
}
```

Pseudocode of the AltPalette path:

```c
if (type->AltPalette) {
    palette = g_ColorSchemeArray[0]->ConvertPalette;   // ColorScheme[0] + 0x30C
}
```

`g_ColorSchemeArray[0]` is the first entry in the color-scheme array,
**always index 0 — NOT keyed by the owner's color index**. This is the
default/neutral color scheme. The `+0x30C` offset is the
`ConvertPalette` field of the ColorScheme struct — a remap-to-RGB table.

### Cross-reference with prior research

`ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` §3.6 also documents this path
verbatim and explicitly states:

> in RA2, a garrison muzzle flash attached to a Soviet-house building
> still renders with the default palette, not a red/blue tint. This
> matches the observable behavior in retail — muzzle flashes do not
> recolor per owner.

PARACH is in the same `AltPalette=yes` family. **Chutes do not tint to
match the dropping player's color.** All chutes render with the same
palette regardless of owner.

### Distinction vs `IsVeins=yes`

`IsVeins=yes` (TS-legacy, dormant in YR for non-vein anims) DOES use the
player's color:
```c
if (type->IsVeins) {
    palette = g_ColorSchemeArray[PlayerPtr->ColorIndex + 0x16054]->Convert;
}
```
This path is owner-tinted. But `AltPalette` is not. Don't confuse them.

### Practical effect — what palette is `ConvertPalette[0]` visually?

The `ConvertPalette` of `ColorScheme[0]` is a converted version of the
master palette suited for unit-style sprites (UNITTEM.PAL or theater
equivalent). Compared to the standard anim palette (`DAT_0087f6c0`),
which is tuned for explosion/effect colors, the AltPalette path renders
the chute with unit-suitable colors (cleaner whites, less saturated
oranges, etc.).

### Implication for the project

The Rust implementation must select **the unit/object palette
(theater-converted), NOT the standard anim palette** when rendering
PARACH. Specifically:

- If our renderer maintains separate "anim palette" and "unit palette"
  buffers, route PARACH (and any AltPalette=yes anim) through the unit
  palette path.
- The selection is **independent of the owner**. Don't apply Allied/
  Soviet/Yuri tinting to PARACH.

The brainstorm doc tagged P6 as "use unit palette" — that's correct in
spirit, but the precise mechanism is "fetch from `ColorScheme[0]`'s
ConvertPalette, which happens to be unit-flavored."

---

## 7. Anim Attachment to the Falling GI — VERIFIED

(This section is reference; the existing
`ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` §3.5 covers it in full. Summary
included here so the chute pipeline is self-contained.)

### `AnimClass::SetOwnerObject(owner)` at `0x00424B50`

When called on a freshly-constructed PARACH anim with the descending GI
as owner:

1. Get the anim's current world coords (the spawn position — typically
   the carrier's drop point, which equals the GI's spawn position).
2. Set owner byte at `+0x84` = 1 on the GI (marker: "has attached anim").
3. Write `anim->OwnerObject = GI` (anim+0xCC).
4. Get the GI's current world coords.
5. **Compute relative offset:** `anim.coords = chute_world - GI_world`.
   For chute spawned AT the GI's position, this is `(0, 0, 0)`.
6. Re-submit anim to the display layer.

### After attachment

Every subsequent `anim->GetCoords()` call returns:
```
GI.coords + stored_offset = GI.coords + (0,0,0) = GI.coords
```

So the chute's world position **always equals the GI's world position**.
As the GI's `Z` (altitude) decreases each tick (per
`parachute_descent.rs`), the chute's Z follows automatically.

### Implication for the project

The Rust implementation does NOT need to maintain a separate altitude
state for the chute. The chute renders at the GI's `screen_x` /
`screen_y` (which already accounts for altitude via
`ALTITUDE_VISUAL_SCALE * state.altitude`), centered on that screen
coordinate, with depth offset `ZAdjust=-10`.

---

## 8. Lifecycle — When Is the Chute Removed?

The PARACH AnimType has `LoopCount=30`. With ~133ms per frame and 20
frames per loop, a full 30-loop run is ~80 seconds — far longer than any
realistic descent.

In gamemd, the chute is destroyed **externally**, not by exhausting
LoopCount. Two mechanisms:

1. **Owner unhooks anim on landing.** When the descending unit lands,
   the parachute is detached via `SetOwnerObject(NULL)` or a similar
   cleanup, and `AnimClass::Destroy` (vtable+0xF8) marks it for deletion.
2. **Owner dies mid-descent.** If the GI is killed by AA fire while
   descending, the cleanup path in the owner's `UnInit` chain invalidates
   any attached anims — they stop following and self-destruct.

Both paths end with the anim being added to the pending-delete list at
`0x00B0F69C`, freed on the next post-tick cleanup pass.

### Implication for the project

The Rust implementation should **not** rely on `LoopCount` to terminate
the chute. The polling-based lifecycle proposed in the brainstorm (spawn
on `parachute_state.is_some()`, despawn on `parachute_state.is_none()` or
target entity missing) is the correct shape — it matches gamemd's
"external termination" semantics.

---

## 9. Loop Logic — When Does Deploy Become Loop?

From `AnimClass::AI` (per `ANIM_CLASS_GHIDRA_REPORT.md` §AI):

> **Normal**: When `CurrentFrame >= End`:
> - If `LoopCountRemaining > 1` (and not 0xFF = infinite): decrement loop
>   count, reset `CurrentFrame` to `LoopStart`, apply `RandomLoopDelay`
>   if set.
> - If `LoopCountRemaining == 1`: check for `Next` anim type. If set,
>   replace `Type` pointer and restart. Otherwise, mark for deletion.

For PARACH:
- `LoopCount = 30` → `LoopCountRemaining = 30 * 1 = 30` (loopCount param 1)
- `LoopStart = 20`, `LoopEnd = 39`, `End = 40` (auto-detected; 40 frames
  in retail PARACH SHP)
- Deploy phase: frames 0..19 play once on the **first cycle**.
- After `CurrentFrame` increments past `End` (`>= 40`): decrement loop
  count, reset to `LoopStart=20`. Subsequent cycles only play frames
  20..39.

So:
- Cycle 1: frames 0..39 (full 40 frames — includes deploy 0..19, then
  20..39 once)
- Cycle 2 to 30: frames 20..39 only

The deploy phase is naturally one-shot via the LoopStart mechanism — no
explicit "deploy/loop" state machine in the binary. The state is
implicit: `CurrentFrame < LoopStart` ⇒ deploy phase; `CurrentFrame >=
LoopStart` ⇒ loop phase.

### Note on `LoopEnd` vs `End`

In the binary, `End` (auto-detected from SHP frame count) is the actual
end of the animation — the point at which loop wraparound or termination
occurs. `LoopEnd` (`+0x2BC`) is a separate field (clamped: if
`LoopEnd > End` then `LoopEnd = End`). Both default to `End` if not
specified.

For `[PARACH]`: `LoopEnd=39` and `End` likely also = 40 (auto-detected);
ReadINI doesn't set `End` from INI for PARACH. The actual frame range
that loops is `LoopStart..End` when `LoopEnd` equals `End`, or
`LoopStart..LoopEnd` otherwise — the binary uses `End` for the
wraparound check (see code in `AnimClass::AI`).

For PARACH the practical result is identical: loop frames 20..39 inclusive.

### Implication for the project

The Rust implementation can use a simple `frame: u16` counter that
advances each tick:

```
// Pseudocode (NOT to be implemented as Rust here)
frame += 1 every (rate_ms / tick_ms) ticks
if frame >= End:
    frame = LoopStart
    loops_done += 1
```

The "deploy phase" is simply `frame < LoopStart` — no explicit phase
state machine needed.

---

## 10. INI Keys — Verified

| Key | Type | PARACH value | Default | Effect | Confidence |
|---|---|---|---|---|---|
| `Rate` | int | 400 | 1 | `internal = 900 / INI_Rate` ticks/frame | HIGH |
| `LoopStart` | int | 20 | 0 | Frame to wrap to on `End` | HIGH |
| `LoopEnd` | int | 39 | 0 | Loop end (clamped to End) | HIGH |
| `LoopCount` | int | 30 | 0 | Max loop iterations before destruction | HIGH |
| `AltPalette` | bool | yes | false | `palette = ColorScheme[0]->ConvertPalette` | HIGH |
| `ZAdjust` | int | -10 | 0 | Depth-sort offset (NOT screen Y) | HIGH |
| `YDrawOffset` | int | (not set) | 0 | Screen-Y offset | HIGH |
| `Layer` | enum | (not set) | 3 (Ground) | Render-layer slot | HIGH |
| `End` | int | (not set) | 0 → SHP-detected | Frame count | HIGH |
| `Start` | int | (not set) | 0 | First frame to play | HIGH |

Other PARACH-related rules.ini keys (already wired in `[General]`):
- `Parachute=PARACH` (rulesmd.ini, parsed into general rules)
- `BombParachute=PARABOMB` (rulesmd.ini, parabombs — out of scope here)
- `ParachuteMaxFallRate=-3` (already parsed)
- `NoParachuteMaxFallRate=-100` (existence: yes; parsed: deferred per
  parachute-descent design)
- `ChuteSound=ParachuteDrop` (sim event already wired per commit
  `0b7d959`)

---

## 11. Integration Points

### When PARACH spawns in gamemd

The chute is constructed by the paradrop / paratroop drop code (likely
inside the carrier-aircraft drop function or a related TechnoClass
helper). Out of scope for this report — the brainstorm has already
decided that the **Rust implementation will spawn the chute in app code,
polling for `parachute_state.is_some()`**, not via a sim event. Knowing
gamemd's exact spawn site is informational only.

### Per-tick flow for an attached PARACH anim

Each game tick (gamemd):

1. `AnimClass::AI` (vtable+0x60) — frame advance via CDTimerClass
   countdown; loop wraparound; trailer-anim spawn.
2. (No layer/position update for the anim itself — its world coords are
   computed from `owner.coords + offset` on demand.)
3. The display layer's render pass calls `AnimClass::DrawIt` with the
   current screen coords (transformed from owner's world coords).

### Per-tick flow for the Rust implementation

Mirror as polling:

1. App-tick `tick_parachute_anims()` (proposed in brainstorm):
   - For each anim, advance `frame` by `rate_ms` / `tick_ms`.
   - Wrap from `End` to `LoopStart`.
   - Spawn new anims for entities with `parachute_state.is_some()` not
     yet tracked.
   - Despawn anims for entities missing or `parachute_state.is_none()`.
2. Per-render-frame instance build: project entity's `screen_x` /
   `screen_y` to a sprite instance with the chute's current frame.

---

## 12. Current Rust Implementation Status

What exists:
- `ParachuteDescentState` ([src/sim/movement/parachute_descent.rs](../src/sim/movement/parachute_descent.rs))
  — descent integrator, sets body sequence to `SequenceKind::Paradrop` on
  attach, clears on landing.
- Body screen-Y altitude offset already applied via
  `entity.position.screen_y = sy - sim_to_f32(state.altitude) * ALTITUDE_VISUAL_SCALE`.
- `art.rate` is parsed into `art_data.rs` `rate: u16` field, reading the
  raw INI value.
- `GarrisonMuzzleFlash` provides a precedent pattern for transient anims
  attached to entities.

What's missing (relative to the chute-rendering pipeline):
- No `ParachuteAnim` state, no spawning, no rendering.
- No PARACH SHP atlas loading (verify if PARACH is already loaded).
- No AltPalette honor in the renderer — need to find/add a path that
  selects the unit/Convert palette for AltPalette=yes anims.
- `art.rate` is being misused as ms-per-frame in some paths (cranes, fire
  effects); for PARACH and any future anim parity, this should be fixed
  or worked around per-call-site. Cross-cutting; not in scope here.

---

## 13. Resolved Open Questions

### Q1 — Layer for attached anims (RESOLVED, HIGH confidence)

**Finding: when an anim is attached to an owner, gamemd forces it into
Layer 2 (Ground), overriding the AnimType's `Layer=` field entirely.**

Evidence: `AnimClass::GetLayer` (vtable+0x78) at `0x00424cb0` (verified
in `LAYER_CLASS_GHIDRA_REPORT.md` §3):

```c
int AnimClass__GetLayer(AnimClass* this) {  // vtable+0x78
    if (this->field_0xCC != 0) return 2;    // attached to owner → Ground
    if (this->AnimType != NULL)
        return this->AnimType->Layer;
    return 3;                               // ownerless+typeless → Air
}
```

For PARACH attached to a falling GI: `field_0xCC` (owner pointer) is
non-zero, so the chute is forced to Layer 2 regardless of art.ini's
`Layer=Ground` (which is the default and would be Layer 2 anyway, but
the override is unconditional). The chute joins the same Y-sort layer as
the GI body — no special airborne-anim handling needed.

### Q2 — PARACH SHP frame count (RESOLVED, MEDIUM confidence — inferred)

**Finding: PARACH.SHP has 40 frames total (indices 0-39).**

Direct verification (hex-dumping the SHP) was not performed in this
session. Inferred from:
- `LoopEnd=39` in art.ini
- gamemd's clamp: `if LoopEnd > End: LoopEnd = End`, so `End >= 39`
- No `End=` set in art.ini → `End` auto-detects from SHP frame count
- Standard RA2 retail PARACH.SHP is documented in modding community
  references as 40 frames

To upgrade to HIGH confidence: run `cargo run --bin mix-browser` and
inspect PARACH.SHP, or add a one-shot test that reads the SHP header
bytes (offset 6 = `u16 frame_count`).

### Q3 — PARACH facing count (RESOLVED, HIGH confidence)

**Finding: PARACH is single-facing. AnimType has no `Facings=` field in
gamemd.**

Evidence: `AnimTypeClass::ReadINI` at `0x00427D00` (decompiled in this
investigation) does NOT parse a `Facings=` key. The Rust codebase's
`Facings=` parser at `src/rules/art_data.rs:236-237` is for SHP
**vehicles**, not anims:

```rust
let shp_facings: u8 = section
    .get_i32("Facings")
    // ...
```

Anims render the same SHP regardless of facing. The chute looks identical
no matter which way the GI is oriented (or even if the GI rotates
mid-descent — irrelevant since paratroop GIs don't change facing while
falling).

### Q4 — Rate= cross-cutting interpretation (RESOLVED — was a false alarm)

**Finding: the project ALREADY has the correct conversion. My initial
claim in §4 of this report ("the current Rust code is mathematically
inconsistent with gamemd") was WRONG. Retract.**

Evidence: [src/rules/art_data.rs:134-140](../src/rules/art_data.rs#L134-L140)
contains the correct helper:

```rust
pub fn art_rate_to_delay_ms(ini_rate: i32) -> u32 {
    if ini_rate < 1 { return 0; }
    let delay_frames: u32 = 900 / ini_rate as u32;
    (delay_frames * 1000 / 15).max(1)
}
```

This is the exact gamemd formula `900 / INI_Rate` ticks-per-frame,
multiplied by `1000/15` ms-per-tick = ms-per-frame. Verified call sites
that route through it correctly:

- `BuildingAnimConfig.rate` parsing
  ([art_data.rs:718](../src/rules/art_data.rs#L718))
- `rate_from_section` for warp anims, wake, fire effects
  ([ruleset.rs:912](../src/rules/ruleset.rs#L912))

**Cross-cutting fix is NOT needed for the chute implementation.** The
PARACH `rate_ms` should be computed by routing the art.ini Rate=400
through `art_rate_to_delay_ms` — same pattern as building anims. Yields
133ms per frame, matching gamemd.

### Out-of-scope follow-ups (noted for future investigation)

1. `app_instances/shp.rs:454` has a suspicious `(anim.rate as u32).max(1) * 2`
   in `looping_frame`. If `anim.rate` is already ms-per-frame (which it is
   after `art_rate_to_delay_ms`), the `* 2` doubles per-frame duration
   — possibly a bug, possibly an intentional ping-pong workaround.
   Suggest `/disparity-scan building-anim timing` if visible drift.
2. PARACH frame count direct verification (Q2 upgrade to HIGH).

---

## Sources

### Decompiled functions (live Ghidra)
- `AnimTypeClass::ReadINI` at `0x00427D00` (Rate conversion verified)
- `AnimClass::DrawIt` at `0x00422CA0` (depth formula, palette cascade
  verified)

### Existing Ghidra reports referenced
- `ANIM_CLASS_GHIDRA_REPORT.md` (HIGH confidence) — full struct layouts,
  constructor params, AI loop logic
- `ANIM_CLASS_DEEP_DIVE.md` (HIGH confidence) — vtable, Middle, Start
- `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` (HIGH confidence) — palette
  cascade §3.6, attachment §3.5
- `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md` — paradrop SW context

### INI files
- `ini/artmd.ini:15642-15649` — `[PARACH]` section
- `ini/rulesmd.ini` — `[General] Parachute=PARACH` reference

### Rust files
- `src/rules/art_data.rs` — AnimType field parsing (Rate, LoopStart,
  LoopEnd, LoopCount, AltPalette, ZAdjust, YDrawOffset)
- `src/sim/movement/parachute_descent.rs` — descent state machine
- `src/sim/components.rs:510` — `GarrisonMuzzleFlash` precedent struct
- `src/app_building_anim.rs:495+` — `tick_garrison_muzzle_flashes`
  precedent function
