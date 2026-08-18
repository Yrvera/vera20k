# LineTrail Class — Ghidra Report

**Status:** HIGH confidence. All five LineTrail routines and the full 0x210-byte layout were extracted directly from the binary and cross-checked with the caller site in `ObjectClass::Reveal`.

**YR-live:** Yes. `UseLineTrail=yes` is actively set on standard YR projectiles such as `[MEDUSA]` (Aegis missile) and `[DRAGON]` (Patriot / IFV missile) in `ini/artmd.ini` lines 14749 and 14757. The update/draw pipeline runs every render frame via `TacticalClass_Draw`.

**Not standalone — but not just a flag either.** `LineTrail` is its own heap-allocated helper object (MSVC RTTI `.?AVLineTrail@@`, stored in a global `DynamicVectorClass<LineTrail*>`), but it has **no vtable, no base class, and no independent lifetime beyond its owner**. It is created by `ObjectClass::Reveal` when the owner's `ObjectTypeClass->UseLineTrail` is true, and it is destroyed by `ObjectClass`'s destructor. Every `LineTrail` holds a back-pointer to exactly one `ObjectClass`. Conceptually it is a "ring-buffer trail component attached to an Object," not an entity.

---

## 1. Key addresses

All labeled in the Ghidra project (saved).

| Address | Name | Purpose |
|---------|------|---------|
| `0x00556a20` | `LineTrail__Constructor` | Allocate, init ring buffer, register in global vector |
| `0x00556b30` | `LineTrail__DetachFromOwner` | Sets owner->+0xA8 = 0 and clears own owner ptr |
| `0x00556b50` | `LineTrail__SetColorDecrement` | Sets field `+0x08`; doubles the argument if the 16-bit-color global flag is 0 |
| `0x00556b70` | `LineTrail__Update` | Per-frame ring-buffer advance + brightness decrement |
| `0x00556c00` | `LineTrail__Draw` | Walks the ring forward drawing thin lines between successive non-sentinel points |
| `0x00556d40` | `LineTrail__UpdateAndDrawAll` | Iterates global vector, calls Update, calls Draw if live, else deletes |
| `0x00556df0` | `LineTrail__ClearAll` | Shutdown path: detach all, free all, reset vector |

**Caller sites:**
- `ObjectClass::Reveal` (`0x005F4EC0`) — creation (line where `operator_new(0x210)` runs)
- `ObjectClass` destructor (`0x005F3B80`) — deletion via `LineTrail__DetachFromOwner` (note: detach, not free — the global vector owns the free)
- `TacticalClass_Draw` (`0x006D3D10`) — per-frame Update+Draw via `LineTrail__UpdateAndDrawAll`
- `FUN_00534450` — shutdown path calling `LineTrail__ClearAll`

**RTTI strings (binary):**
- `0x00845E38` is **NOT** LineTrail — it is `BounceClass` (see other report).
- `0x00829D50` — `.?AV?$VectorClass@PAVLineTrail@@@@`
- `0x00829D80` — `.?AV?$DynamicVectorClass@PAVLineTrail@@@@`
- The `LineTrail` class itself has no independent RTTI TypeDescriptor with cross-references; it is emitted only as a template argument, consistent with having no virtual methods.

---

## 2. Struct layout (`sizeof = 0x210` / 528 bytes)

Verified by `operator_new(0x210)` in `ObjectClass::Reveal` and the full field enumeration in `LineTrail__Constructor` at `0x00556a20`.

| Byte offset | Size | Type | Field | Default (constructor) | Notes |
|---|---|---|---|---|---|
| `0x00` | 1 | u8 | `Color.R` | `0x80` | Overwritten by caller from `ObjectTypeClass->LineTrailColor.R` (`+0x23B`) or by the `LineTrailColorOverride` global if set |
| `0x01` | 1 | u8 | `Color.G` | `0x80` | Same |
| `0x02` | 1 | u8 | `Color.B` | `0x80` | Same |
| `0x03` | 1 | u8 | (pad / alignment) | 0 | |
| `0x04` | 4 | `ObjectClass*` | `OwnerObject` | 0 | Set by `ObjectClass::Reveal` immediately after construction: `line[0x04] = owner` |
| `0x08` | 4 | i32 | `ColorDecrement` | `0x10` (16) | Overwritten by `LineTrail__SetColorDecrement(type->LineTrailColorDecrement)`. That setter doubles its input if the global 16-bit-color flag `DAT_00a8eb78` is 0 (the normal case), so the stored value is typically `2 × Type->LineTrailColorDecrement` |
| `0x0C` | 4 | i32 | `HeadIndex` | 0 | Ring-buffer write head (0..31). Advanced backwards (decrement) when a new point is inserted |
| `0x10` | 512 | `Point[32]` | `Ring` | `(0,0,0,0)` for every entry | 32 entries × 16 bytes. See Point layout below |

**Point (16 bytes per ring entry):**

| Offset | Size | Type | Field | Initial |
|---|---|---|---|---|
| `+0x00` | 4 | i32 | `X` (leptons) | 0 |
| `+0x04` | 4 | i32 | `Y` (leptons) | 0 |
| `+0x08` | 4 | i32 | `Z` (leptons) | 0 |
| `+0x0C` | 4 | i32 | `Brightness` (0..255) | 0 (sentinel = empty slot) |

**Sentinel:** an empty ring slot is `(X=0, Y=0, Z=0, Brightness=0)`. The constructor fills all 32 slots with this sentinel by copying `(DAT_00abcb50, DAT_00abcb54, DAT_00abcb58, 0)`, which are all zero (read directly from memory to confirm).

Total: `0x04 × 4` (header) + `0x10 × 32` (ring) = `0x10 + 0x200 = 0x210` bytes. Matches the `operator_new(0x210)`.

---

## 3. Global state

| Address | Type | Purpose |
|---|---|---|
| `0x00ABCB50..0x00ABCB58` | 12 bytes | Global "invalid coord" sentinel used to fill empty slots and to mark empty during Draw. Contents are `(0,0,0)` at game start. |
| `0x00ABCB78` | `VectorClass*` vtable slot | Vector vtable pointer (used for capacity grow) |
| `0x00ABCB7C` | `LineTrail**` | `DynamicVectorClass<LineTrail*>::Items` — the array |
| `0x00ABCB80` | i32 | Vector capacity |
| `0x00ABCB85` | u8 | `IsInitialized` flag of the vector |
| `0x00ABCB88` | i32 | **Active LineTrail count** — drives the update loop |
| `0x00ABCB8C` | i32 | Grow-by amount |
| `0x00A8EB78` | i32 | Global bit-depth flag. If 0 (default), `LineTrail__SetColorDecrement` doubles its argument before storing |
| `g_RulesClass + 0x1863` | 3 bytes | `LineTrailColorOverride.R/G/B` from `[AudioVisual]` |
| `g_RulesClass + 0x1865` | 1 byte | 4th byte of override — used as "override is active" detection: the check is `(R==0 && G==0 && B==0)` → use type's color; otherwise use override |

The global vector is neither initialized nor freed by any of the LineTrail functions in a lazy way — `LineTrail__ClearAll` does the final tear-down at `DAT_00ABCB88 = 0`, frees `DAT_00ABCB7C` if `IsInitialized`, and resets capacity.

---

## 4. ObjectTypeClass INI fields (source of creation)

All three keys are read in `ObjectTypeClass::ReadINI` at `0x005F92D0` from the per-object-type section (matches `TypeName` / `Image` of that section — standard ObjectTypeClass rules). **Keys live in `art(md).ini`.** Struct offsets confirmed both in the ReadINI function and in `ObjectTypeClass::Constructor` (`0x005F7090`) where defaults are written.

| INI key | Type | Byte offset in ObjectTypeClass | Default | Source (rules/art) | Confirmed at |
|---|---|---|---|---|---|
| `UseLineTrail` | bool | `0x23A` | `false` | `art(md).ini` per-[Image] | `ObjectTypeClass__ReadINI` call to `CCINIClass__ReadBool(…, s_UseLineTrail, …)` |
| `LineTrailColor` | R,G,B (3 bytes) | `0x23B`, `0x23C`, `0x23D` | `0x80,0x80,0x80` | `art(md).ini` per-[Image] | Same function, `CCINIClass__ReadColorRGB` |
| `LineTrailColorDecrement` | int | `0x240` | `0x10` (= 16) | `art(md).ini` per-[Image] | `param_1[0x90]` in the `int*`-typed ReadINI; `*(undefined4*)(iVar4 + 0x240)` in Reveal |

**Note on 0x23E / 0x23F:** there is a 2-byte gap between LineTrailColor (ends at `0x23D`) and LineTrailColorDecrement (starts at `0x240`). The gap is alignment padding so the next `int` is 4-byte aligned.

**Pitfall confirmed:** `ObjectTypeClass::ReadINI` is declared `int *param_1`, so its `param_1[0x90]` expression maps to byte offset `0x90 * 4 = 0x240`. But the same function also uses `*(undefined1 *)((int)param_1 + 0x23A)` — direct byte offsets when the C++ field is a byte. Always check the cast before trusting a numeric literal.

### INI examples from the repo

`ini/artmd.ini:14747-14760`:
```
[MEDUSA]                    ; Aegis AA missile
UseLineTrail=yes
LineTrailColor=208,208,208
LineTrailColorDecrement=12

[DRAGON]                    ; IFV / Patriot missile
UseLineTrail=yes
LineTrailColor=216,216,255
LineTrailColorDecrement=16
```

### RulesClass INI field

| INI key | Section | Offset in RulesClass | Default |
|---|---|---|---|
| `LineTrailColorOverride` | `[AudioVisual]` | `+0x1863..+0x1865` (R,G,B) | `0,0,0` |

Read at `0x0066B789` in `RulesClass__ReadAudioVisual` (`0x006691E0`). Note from `rules(md).ini`:
```
LineTrailColorOverride=0,0,0   ; For use in maps only! Leave this at 0,0,0 in Rules.INI.
```

---

## 5. Lifecycle

### 5a. Spawn — `ObjectClass::Reveal` (`0x005F4EC0`)

Full path in the caller (simplified from the decompiled Reveal):

```text
1. Object transitions limbo → on-map (Mark(PUT) succeeds, layer submit, alpha shape).
2. Read type = vtable[0x88](this).
3. If type->UseLineTrail (+0x23A) != 0:
     ptr = operator_new(0x210)
     if ptr != NULL:
         LineTrail__Constructor(ptr)       // all defaults, ring filled with (0,0,0,0)
         this->LineTrailer (+0xA8) = ptr
     else:
         this->LineTrailer = NULL          // allocation failure is silently tolerated

     // Color selection — uses the override ONLY if any of R/G/B are non-zero
     if (RulesClass+0x1863 == 0 && RulesClass+0x1864 == 0 && RulesClass+0x1865 == 0):
         ptr->Color.R/G/B = type->LineTrailColor (+0x23B, +0x23C, +0x23D)
     else:
         ptr->Color.R/G/B = RulesClass->LineTrailColorOverride (+0x1863..+0x1865)

     LineTrail__SetColorDecrement(type->LineTrailColorDecrement at +0x240)
         // setter may double the value; see §3
     ptr->OwnerObject (+0x04) = this
```

The trail is attached *after* the alpha-shape creation but inside the same `if (IsAlive)` branch, so an object that Reveals while dead never gets a trail.

### 5b. Per-frame update — `LineTrail__Update` (`0x00556b70`)

Called from `LineTrail__UpdateAndDrawAll` which is in turn called from `TacticalClass_Draw`. **This runs per rendered frame, not per game tick.** It is therefore non-deterministic and must never feed sim state.

```c
// this = LineTrail*
// owner = *(ObjectClass**)(this + 4)
if (owner != NULL) {
    coord3D pos = { owner->Location.X, owner->Location.Y, owner->Location.Z };   // +0x9C..+0xA4
    int headIdx = this->HeadIndex;                                                // +0x0C
    Point* nextSlot = &this->Ring[(headIdx + 1) % 32];

    // Only insert a new point if the owner has moved since the last recorded head
    if (pos != nextSlot->xyz) {
        headIdx = (headIdx == 0) ? 31 : (headIdx - 1);       // decrement, wrap
        this->HeadIndex = headIdx;
        Point* newHead = &this->Ring[(headIdx + 1) % 32];
        newHead->Brightness = 0xFF;                          // fresh point = full brightness
        newHead->xyz = pos;
    }
}

// Age every point: subtract ColorDecrement from each slot's Brightness, clamped at 0
int i = this->HeadIndex;
do {
    i = (i + 1) % 32;
    Point* p = &this->Ring[i];
    p->Brightness -= this->ColorDecrement;
    if (p->Brightness < 0) p->Brightness = 0;
} while (i != this->HeadIndex);
```

Two behaviours worth noting:
- **Movement-gated insertion.** A stationary owner does not consume ring slots; only slots already present age out. This is why a hovering/paused projectile still shows a shrinking tail.
- **Empty owner ⇒ aging-only.** If `OwnerObject` is NULL (detached via destructor), the insert branch is skipped and the trail fades in place.

### 5c. Per-frame draw — `LineTrail__Draw` (`0x00556c00`)

```c
int i = (this->HeadIndex + 1) % 32;                  // start one past the head (oldest-of-current-cycle)
Point* p = &this->Ring[i];

if (p->xyz == (0,0,0)) return;                       // head-adjacent slot empty → nothing to draw

int brightness = p->Brightness;
while (brightness != 0) {
    i = (i + 1) % 32;
    Point* q = &this->Ring[i];
    if (q->xyz == (0,0,0)) return;                   // encountered a sentinel — trail ends here

    screenA = TacticalClass__CoordsToClient2(p);
    screenB = TacticalClass__CoordsToClient2(q);
    zA = AdjustForZ();                               // per-point depth offset
    zB = AdjustForZ();

    // FUN_004beac0 is the clipped Bresenham line rasterizer used by the tactical layer.
    // Args: viewport base, screenA, screenB, LineTrail* (context), brightness, -2-zA, -2-zB
    FUN_004beac0(&g_RadarViewportOffsetX, screenA, screenB,
                 (void*)this, p->Brightness, -2 - zA, -2 - zB);

    if (i == this->HeadIndex) return;                // walked the whole ring
    p = q;
    brightness = q->Brightness;
}
```

**Color.** The line rasterizer reads the LineTrail's `Color.R/G/B` (offsets 0..2) via the pointer passed in the 4th arg; the per-point `Brightness` modulates opacity. Empirically, the color is the RGB from INI; the brightness linearly fades with age.

### 5d. Shutdown — `ObjectClass` destructor (`0x005F3B80`)

```c
if (this->LineTrailer (+0xA8) != NULL) {
    LineTrail__DetachFromOwner(this->LineTrailer);   // zeros owner->+0xA8 and trail->+0x04
    this->LineTrailer = NULL;
}
```

Detach only. The trail object itself is not freed here — it stays in the global vector with `OwnerObject==NULL` and ages out. `LineTrail__UpdateAndDrawAll` sees this state (see §5e).

### 5e. Update-and-draw-all gatekeeper — `LineTrail__UpdateAndDrawAll` (`0x00556d40`)

```c
for (int i = DAT_00ABCB88 - 1; i >= 0; --i) {
    LineTrail* t = Items[i];
    LineTrail__Update(t);

    // "Is the trail completely empty?" — (OwnerObject == NULL) AND (head-adjacent point brightness == 0)
    // Head-adjacent slot address: Ring[(HeadIndex + 1) % 32].Brightness
    if (t->OwnerObject == NULL &&
        t->Ring[(t->HeadIndex + 1) % 32].Brightness == 0) {
        // remove from vector, operator_delete(t)
        Items[vector.find(t)] = shifted-left-fill;
        DAT_00ABCB88--;
        free(t);
    } else {
        LineTrail__Draw(t);
    }
}
```

The empty-check is slightly odd — it inspects only one slot's brightness rather than scanning all 32 — but combined with the aging loop in Update, a trail with no owner will eventually zero every slot and the head-adjacent slot's brightness is the last to reach 0 in exactly the same tick all the others do (they all decrement at the same rate). So the check is equivalent to "all slots are zero."

Iteration is back-to-front to make in-loop deletion safe.

### 5f. Game shutdown — `LineTrail__ClearAll` (`0x00556DF0`)

Walks from index 0 forward, detaches each trail from its owner, and frees it; then frees the vector array and resets all capacity / count / init fields.

---

## 6. Call graph

```
ObjectClass::Reveal (0x005F4EC0)
 └─ operator_new(0x210)
 └─ LineTrail__Constructor (0x00556A20)
      └─ DynamicVectorClass::Add via DAT_00ABCB78 vtable
 └─ LineTrail__SetColorDecrement (0x00556B50)

TacticalClass_Draw (0x006D3D10)
 └─ LineTrail__UpdateAndDrawAll (0x00556D40)
      ├─ LineTrail__Update (0x00556B70)
      ├─ LineTrail__Draw (0x00556C00)      [when trail still has content]
      │    └─ TacticalClass__CoordsToClient2
      │    └─ AdjustForZ
      │    └─ FUN_004BEAC0  (line rasterizer: ClipRect, ZBuffer_scanline_ptr, CircBuf_GetScanlinePtr)
      └─ operator_delete (+ vector.Remove)  [when trail is dead and ownerless]

ObjectClass::~ObjectClass (0x005F3B80)
 └─ LineTrail__DetachFromOwner (0x00556B30)   [owner cleared; trail self-reaps next frame]

FUN_00534450 (game tear-down)
 └─ LineTrail__ClearAll (0x00556DF0)
```

---

## 7. TS-legacy status

**Live in YR.** Verified both by INI usage (MEDUSA/DRAGON use `UseLineTrail=yes` in the current `ini/artmd.ini`) and by the callsite being `TacticalClass_Draw`, which runs unconditionally every frame in YR skirmish. No `SpecialFlags` gating; no TS-only `g_RulesClass` guards on the update path.

There is one TS-era artefact worth noting: the `FUN_00556B50` setter doubles the decrement if `DAT_00A8EB78` (a 16-bit-vs-hi-color flag written by the DirectDraw/rendering init) is 0. This is a Tiberian Sun remnant of supporting palette vs high-color surfaces. In a modern re-implementation working in 32-bit color, we can either: (a) drop the doubling and read the decrement straight from INI (simpler, visually slightly different from retail), or (b) always double to match retail visuals at 32-bit color. Retail runs in whichever mode the Direct3D init picked; in practice YR almost always runs with this flag at 0 (the doubling branch). Recommend option (b) for fidelity.

---

## 8. Rendering path summary

- Trails live in **tactical-layer screen space**, drawn via `FUN_004BEAC0`, which is a clipped line rasterizer that writes to the scene's off-screen buffer and consults the Z-buffer scanline (`ZBuffer_scanline_ptr`) for per-pixel depth.
- Each point carries its own `-2 - AdjustForZ(...)` Z offset; the line between two points is drawn with whatever occlusion/blending the rasterizer applies.
- Color is a single RGB per trail (no per-point color); brightness fades linearly per frame.
- Line thickness is 1 pixel (no width parameter).

---

## 9. Open questions (low priority)

1. **~~`FUN_004BEAC0` blend mode.~~** RESOLVED in round 2 — see §11 below.
2. **~~`DAT_00A8EB78` setter source.~~** RESOLVED in round 2 — it is the `[Options]
   DetailLevel` knob, not a color-depth flag. See §11 and the PixelFX report Q2.
3. **The head-adjacent empty-slot check in `UpdateAndDrawAll`.** Works because all
   slots age at the same rate, but it is not commented in the binary. If we ever
   change per-point aging to be position-dependent, the check would need to become
   a full scan.

---

## 11. Follow-up investigation (round 2) — 2026-04-21

### Q3: `FUN_004BEAC0` blend mode — RESOLVED

**Verdict: per-pixel modulated-alpha blend against the existing scanline, not replace
or additive. Brightness is the alpha scalar. No lookup table — straight integer math.**

Decompiling `FUN_004BEAC0 @ 0x004BEAC0` end-to-end reveals a clipped Bresenham line
rasterizer with the following per-pixel core (from all three octant-dispatch branches,
which are otherwise identical):

```c
uVar1 = *(ushort *)(surface + offset);     // existing dest pixel (RGB565/555)
uint dest_G = (uVar1 >> DD_GShift) << DD_GLoss;   // unpack existing G
uint dest_B = (uVar1 >> DD_BShift) << DD_BLoss;   // unpack existing B
uint dest_R = (uVar1 >> DD_RShift) << DD_RLoss;   // unpack existing R

// line_color (from LineTrail*) pre-scaled by line_brightness (param_3):
iVar9 = (line_R * param_3) >> 8;            // R * line_alpha
iVar6 = (line_G * param_3) >> 8;            // G * line_alpha
iVar8 = (line_B * param_3) >> 8;            // B * line_alpha
iVar17 = 0x100 - param_3;                   // one-minus-line-alpha = (256 - brightness)

// zbuf_alpha = *puVar11 (per-pixel value from A-buffer / alpha-mask buffer)
uint zbuf_alpha = (uint)*puVar11;           // 0..255 per-pixel, fetched per-scanline

if (param_5 < *puVar10 && zbuf_alpha != 0) {     // depth test + mask test
    ushort out_pix =
        ((((dest_G * iVar17 >> 8) + iVar6) * zbuf_alpha >> 7) >> DD_GLoss) << DD_GShift
      | ((((dest_B * iVar17 >> 8) + iVar8) * zbuf_alpha >> 7) >> DD_BLoss) << DD_BShift
      | ((((dest_R * iVar17 >> 8) + iVar9) * zbuf_alpha >> 7) >> DD_RLoss) << DD_RShift;
    *(ushort*)(surface + offset) = out_pix;
}
```

**Blend formula per channel.** Let `Cd` = destination-channel value (0..255),
`Cs` = line-color-channel value (0..255), `a` = line brightness (0..255, passed as
`param_3` / LineTrail's per-point brightness), `m` = per-pixel A-buffer value (0..255):
```
  out_channel = ((Cd * (256 - a) / 256 + Cs * a / 256) * m / 128) >> DD_Loss
              ≈ clamp_to_mask( m/128 × lerp(Cd, Cs, a/256) )
```

This is a **two-stage modulated alpha-over blend**:
1. **Stage 1 (brightness alpha):** standard source-over blend between the existing
   pixel and the trail colour, using `a = brightness / 256` as the interpolation
   factor. `a = 0` → destination unchanged; `a = 255` → nearly pure trail colour.
2. **Stage 2 (A-buffer multiply):** the result is multiplied by `m / 128` where `m`
   is the per-pixel A-buffer value on the same scanline. `m = 128` is "neutral"
   (pass-through). `m = 0` skips the write entirely. `m = 255` boosts the result by
   ~2× (will saturate via the bit-shift packing). This A-buffer provides the
   scanline's "translucency mask" — it's the same buffer used by shroud/fog
   rendering for soft edges.

**Key observations:**
- **NOT additive.** There is no `dst += src` pattern; it's strictly an
  alpha-over-style lerp.
- **NOT replace.** The destination channel always contributes `(256 - a) / 256` of
  its value.
- **NO lookup tables.** All blending is integer multiply+shift math. Safe to
  reproduce directly in wgpu with a standard alpha-blend pipeline.
- **Depth + mask gated.** Two per-pixel tests: `param_5 < *puVar10` is the z-depth
  test (read from `ZBuffer_scanline_ptr`); `zbuf_alpha != 0` is the "visible through
  mask" test (the A-buffer). Both must pass or the pixel is skipped entirely.

**Rendering parity plan.** The Rust engine should implement line trails as
**alpha-over blended lines** (standard `OVER` blend mode) with brightness as alpha,
plus a depth test against the terrain z-buffer. The secondary A-buffer multiply can
be approximated with a separate fullscreen mask texture or omitted for simplicity
(the result will be slightly brighter than retail at soft shroud edges).

**Why this matters for LineTrail specifically.** The per-point `Brightness` field
IS the alpha scalar. It's decremented by `ColorDecrement` each tick. So older points
in the ring naturally fade via alpha, exactly as described in §5b — and now
verified at the rasterizer level.

### Q4: `DAT_00A8EB78` setter source — RESOLVED (cross-reference)

See `PIXEL_FX_CLASS_GHIDRA_REPORT.md §Follow-up Q2` for the full investigation.

**Summary:** `DAT_00A8EB78` is the **`[Options] DetailLevel` knob** (values 0..2),
written by two dialog-handler functions:
- `OptionsClass__ApplyFromLauncherDialog @ 0x0055FAA0` (pre-game launcher)
- `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0` (in-game ESC → Options)

Both apply the formula `(dlgValue != 0 ? 2 : 0)` from the Detail combo-box at dialog
control ID `0x52B`. The full 0..2 range is accepted via `OptionsClass__ReadFromINI`
when parsing `RA2MD.INI`.

**Impact on §7 ("TS-legacy status") of this report:**

The round-1 report characterised `DAT_00A8EB78` as "a Tiberian Sun remnant of
supporting palette vs high-color surfaces" — **that characterisation was wrong.**
Corrected interpretation:

- The doubling branch in `LineTrail__SetColorDecrement` (`0x00556B50`) when
  `DAT_00A8EB78 == 0` is actually the **"low-detail" branch**, not a color-depth
  branch. When the player sets `DetailLevel=0` (the YR default!), line trails fade
  twice as fast, presumably to reduce visible on-screen activity for
  lower-spec machines.
- In the **default YR configuration** (`DetailLevel=0`), trails fade at
  `2 × Type->LineTrailColorDecrement` rate — this IS the normal retail behaviour.
- When the player opts into high detail (`DetailLevel=2`), trails fade at the
  as-specified `Type->LineTrailColorDecrement` rate — longer, more visible trails.

**Recommendation for Rust engine:** keep the round-1 recommendation (always double
for retail fidelity at default settings), but expose it as a user-facing option tied
to the same `[Options] DetailLevel` setting. If `DetailLevel >= 2`, use the raw
per-type decrement; otherwise double it.

### Ghidra labels applied (round 2)

See `PIXEL_FX_CLASS_GHIDRA_REPORT.md §Follow-up` — five OptionsClass dialog
functions labeled in that pass. No LineTrail-specific labels applied in round 2
(all seven LineTrail functions were already labeled in round 1).

Program saved.

---

## 10. Functions labeled this session

All renamed to `ClassName__MethodName` form. `save_program` called at end.

| Address | New name |
|---|---|
| `0x00556A20` | `LineTrail__Constructor` |
| `0x00556B30` | `LineTrail__DetachFromOwner` |
| `0x00556B50` | `LineTrail__SetColorDecrement` |
| `0x00556B70` | `LineTrail__Update` |
| `0x00556C00` | `LineTrail__Draw` |
| `0x00556D40` | `LineTrail__UpdateAndDrawAll` |
| `0x00556DF0` | `LineTrail__ClearAll` |
