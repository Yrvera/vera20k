# DiskLaserClass — Ghidra Research Report

**Top-line verdict: LIVE IN YR.** `DiskLaserClass` is a first-class simulation object created every time Yuri's **Floating Disc** fires its primary weapon. It participates in the per-tick `LogicClass::PerTickUpdate` loop and drives the iconic expanding-ring laser animation that characterises the unit. It is not a TS-leftover; it is core YR content.

Confidence: HIGH for all struct offsets, vtable layout, lifecycle, and call graph.

---

## Purpose

`DiskLaserClass` is a short-lived per-shot sim entity that drives the rotating multi-segment laser ring effect used exclusively by weapons with `DiskLaser=yes`. Rather than creating a standard `BulletClass` trajectory, a firing disc allocates one `DiskLaserClass` (0x40 bytes) that lives for ~10 sim frames. Each frame it spawns visual `LaserDrawClass` segments (green rotating beam pairs) and, on the final step, applies warhead damage to the target and plays the report.

Why it exists as a dedicated class (not a bullet):
- The ring-up animation is synthesised procedurally from a 16-entry rotation table — there is no per-frame SHP like a normal bullet trail.
- Damage application is deferred to a specific ring-expansion step (not on bullet impact), which requires custom state, not bullet trajectory state.
- Unlike lasers (instantaneous LaserDrawClass), the disc laser holds a MULTI-FRAME wind-up before the killing beam.

---

## Struct layout (0x40 bytes)

Inherits `AbstractClass` (4 vtable pointers + common abstract fields). Verified against `DiskLaserClass::Constructor` at `0x004A7A30` and the AI function at `0x004A7340`.

| Offset | Size | Type | Name | Notes |
|---|---|---|---|---|
| 0x00 | 4 | vtable* | vtable (primary) | `vtable__DiskLaserClass` @ `0x007E5FB8` |
| 0x04 | 4 | vtable* | vtable (secondary +4) | `vtable__DiskLaserClass__secondary_4` @ `0x007E5F9C` |
| 0x08 | 4 | vtable* | vtable (secondary +8) | `vtable__DiskLaserClass__secondary_8` @ `0x007E5F94` |
| 0x0C | 4 | vtable* | vtable (secondary +12) | `vtable__DiskLaserClass__secondary_12` @ `0x007E5F8C` |
| 0x10 | 0x14 | — | AbstractClass body (ID, flags, heap slot, CoordStruct base, etc.) | inherited, not DiskLaser-specific |
| 0x24 | 4 | TechnoClass* | `Source` | The firing disc (Floating Disc instance) |
| 0x28 | 4 | ObjectClass* | `Target` | Target object |
| 0x2C | 4 | WeaponTypeClass* | `Weapon` | Originating weapon (e.g. `[DiskLaser]` / `[DiskLaserE]`) |
| 0x30 | 4 | int | `State` | `-1` = mark-for-removal; `0` = step-this-tick; `>0` = countdown frames |
| 0x34 | 4 | int | `InitialFacing` | Packed 4-bit facing index used as the ring's base angle |
| 0x38 | 4 | int | `StepCounter` | Ring expansion index (0..8), increments each visual step |
| 0x3C | 4 | int | `Flags` | Set by `BulletAnimTracker::Register`, passed through from `Fire_At` |

The constructor explicitly zero-fills `param_1[9..0xF]` which covers 0x24..0x3C. The ring state is then populated by `BulletAnimTracker::Register` at `0x004A71A0` using the trajectory angle derived from `atan2` between source and target.

---

## Vtable layout (24 slots, 0x60 bytes) — `vtable__DiskLaserClass` @ 0x007E5FB8

Derived by reading the 0x60 bytes at 0x7E5FB8 and resolving each target. AbstractClass common slots are labelled where they match the inherited implementations.

| Slot | Offset | Target addr | Function |
|---|---|---|---|
| 0 | 0x00 | 0x00410260 | AbstractClass::QueryInterface |
| 1 | 0x04 | 0x00410300 | AbstractClass::AddRef |
| 2 | 0x08 | 0x00410310 | AbstractClass::Release |
| 3 | 0x0C | 0x004A7C30 | **DiskLaserClass::GetClassID** (returns CLSID at 0x7E9890) |
| 4 | 0x10 | 0x00410450 | AbstractClass getter (inherited) |
| 5 | 0x14 | 0x004A7B90 | **DiskLaserClass::Load** (AbstractClass::Load + rewrite vtables + resolve 3 SwizzleIDs) |
| 6 | 0x18 | 0x004A7C10 | **DiskLaserClass::Save** (AbstractClass::Save passthrough) |
| 7 | 0x1C | 0x004103E0 | AbstractClass::GetSizeMax (inherited) |
| 8 | 0x20 | 0x004A7C90 | **DiskLaserClass::Destructor** (scalar-deleting) — removes self from `g_DiskLaserClass_Array` |
| 9 | 0x24 | 0x00410470 | AbstractClass::GetRefCount (inherited) |
| 10 | 0x28 | 0x00410480 | AbstractClass helper (inherited) |
| 11 | 0x2C | 0x004A7C80 | **DiskLaserClass::RTTI_Type** (returns enum 0x49 = DiskLaser AbstractType) |
| 12 | 0x30 | 0x004A7C70 | **DiskLaserClass::HeapID / Category getter** (returns constant) |
| 13 | 0x34 | 0x004A7B80 | **DiskLaserClass::ComputeCRC** (AbstractClass::ComputeCRC passthrough) |
| 14 | 0x38 | 0x00410490 | AbstractClass getter (inherited) |
| 15 | 0x3C | 0x004104A0 | AbstractClass getter (inherited) |
| 16 | 0x40 | 0x004104B0 | AbstractClass getter (inherited) |
| 17 | 0x44 | 0x00410440 | AbstractClass GetCoords (inherited) — returns default 0,0,0 |
| 18 | 0x48 | 0x004104C0 | AbstractClass::GetCoords2 (inherited) |
| 19 | 0x4C | 0x004104F0 | AbstractClass::GetCoords3 (inherited) |
| 20 | 0x50 | 0x00410520 | AbstractClass helper (inherited) |
| 21 | 0x54 | 0x00410530 | AbstractClass helper (inherited) |
| 22 | 0x58 | 0x00410540 | AbstractClass helper (inherited) |
| **23** | **0x5C** | **0x004A7340** | **DiskLaserClass::AI** — per-tick update+fire+draw (THE core function) |

Notes:
- Slot 23 (offset 0x5C) is called every simulation tick by `LogicClass::PerTickUpdate` for every DiskLaser in the global array.
- Most non-DiskLaser slots are trivial abstract-class wrappers; none of them do anything game-specific.
- The three secondary vtables at `+4/+8/+12` are MI/COM-style thunks into the same method set (not examined in depth — they are standard MSVC COM interface stubs).

---

## Lifecycle

### Creation

`TechnoClass::Fire_At` (`0x006FDD50`), when firing a weapon with `weapon+0x14A` (`DiskLaser=yes`) set, has this branch near `0x006FE47E`:

```c
if (*(char *)(weapon + 0x14A) != '\0'
 && (pvVar12 = operator_new(0x40)) != NULL
 && (iVar9 = DiskLaserClass__Constructor(pvVar12), iVar9 != 0))
{
    this->CurrentBurstIndex++;
    ...                                  // update burst/rof state on firer
    BulletAnimTracker__Register(this, target, weapon, /*flags*/ uVar18);
    return (int *)0x0;                   // DiskLaser is NOT a bullet — Fire_At returns NULL
}
```

So:
1. `new` allocates 0x40 bytes.
2. `DiskLaserClass::Constructor` (`0x004A7A30`) initialises vtables, zero-fills 0x24..0x3C, and **inserts itself** into `g_DiskLaserClass_Array` via `DynamicVectorClass::Add` logic using the four globals:
   - `g_DiskLaserClass_Array_Vtable   @ 0x008A0208`
   - `g_DiskLaserClass_Array         @ 0x008A020C`
   - `g_DiskLaserClass_Array_Capacity @ 0x008A0210`
   - `g_DiskLaserClass_Array_Count    @ 0x008A0218`
3. `BulletAnimTracker::Register` (`0x004A71A0`) fills in `this[0x24..0x3C]`:
   - `+0x24 = source`, `+0x28 = target`, `+0x2C = weapon`, `+0x3C = flags`
   - `+0x34 = InitialFacing = ((atan2(dy, dx) >> 11) + 1) >> 1 & 0xF + 8` (4-bit ring-start angle)
   - `+0x30 = 0` (state = firing), `+0x38 = 0` (step counter = 0)
   - It also appends `this` to a SECONDARY tracker array `g_0x00B0F6A0..` shared with particle-systems and techno-cell-action cleanup. That array's purpose is object-tracker / pending-delete integration (used by `TagClass::Constructor`, `ObjectClass::UnInit`, etc.), so when the firing disc dies mid-attack the DiskLaser gets cleaned up too.

### Update (per tick)

`LogicClass::PerTickUpdate` at `0x0055B5A1` contains:

```c
iVar6 = DAT_008a0218;                    // g_DiskLaserClass_Array_Count
while (iVar6 = iVar6 + -1, -1 < iVar6) {
    (**(code **)(**(int **)(DAT_008a020c + iVar6 * 4) + 0x5C))();  // vtable[23] = AI
}
```

Every live DiskLaserClass is iterated in reverse and its slot-23 virtual method is called — this is `DiskLaserClass::AI` at `0x004A7340`.

The AI function:

1. **Phase gate** on `state` (+0x30):
   - `state < 0` → mark for removal (`FUN_0x004A7FE0` pushes a deferred delete slot), return.
   - `state > 0` → decrement and return (waiting between ring steps).
   - `state == 0` → run one ring step this tick.

2. **Validity checks:**
   - Re-read source and target coordinates; call `CoordStruct::Distance3D` to compute range.
   - If target is a building (AbstractType == 6), subtract a foundation-proportional bias (`FoundationHeight + FoundationWidth` × 0x40).
   - If distance > `weapon+0xB4` (Range) → set state=-1, mark for deletion.
   - If `source+0x425` (InLimbo / destroyed flag) → set state=-1, mark for deletion.

3. **Compute ring offsets** from a 16-entry rotation table at `0x008A0180` (dx/dy pairs, initialised at game start):
   - Outer angle A = `(InitialFacing + StepCounter) & 0xF`
   - Outer angle B = `(InitialFacing + StepCounter + 1) & 0xF`
   - Inner angle A = `(InitialFacing - StepCounter + 0x10) & 0xF`
   - Inner angle B = `(InitialFacing - StepCounter + 0xF) & 0xF`

4. **Branch on ring completion** (`uVar10 == uVar17 && iVar7 != 0`):
   - **Completed (final step — fire):**
     - Snap to target coord.
     - Spawn ONE `LaserDrawClass` beam from the ring edge to the target. Color = weapon inner/outer/outerspread (`weapon+0x120..0x128`), alpha ~0x3F8 (`3f800000h`).
     - Call `Apply_area_damage(center, weapon->Warhead /*weapon+0xAC*/, damage /*weapon+0xA8*/, source)`.
     - Play `weapon+0xCC` (Report) via `VocClass::PlayAt` if non-zero.
     - Set `state = -1` (will be deleted next tick).
   - **Not completed (still ringing-up):**
     - Play `Rules+0x28C` (some ring-start voice) if it's step 0.
     - Spawn TWO `LaserDrawClass` beams forming the expanding-ring pair.
     - Set `state = 1` (wait 1 frame).
     - `StepCounter++`.

### Color selection (`weapon+0x14D` = IsHouseColor)

- If `IsHouseColor = false` (normal `[DiskLaser]`): color bytes come from `weapon+0x120` (3 bytes RGB per laser component). Inner, outer, outer-spread ingredients live at `weapon+0x120`, `+0x123`, `+0x126`.
- If `IsHouseColor = true`: color comes from the firing house struct at `house+0x56FC`..`+0x56FE`, with each channel halved (`>> 1`) to soften it. This matches YR's "disc fires in house color" behaviour when modders enable IsHouseColor.

### Draw path

DiskLaserClass does NOT draw itself. It emits `LaserDrawClass` segments. Each one is a separate 0x5C-byte allocation constructed via `LaserDrawClass::Constructor` at `0x0054FE60`, which appends to:
- `g_LaserDraw_Array_Vtable`     `@ 0x00ABC878`
- `g_LaserDraw_Array`            `@ 0x00ABC87C`
- `g_LaserDraw_Array_Capacity`   `@ 0x00ABC880`
- `g_LaserDraw_Count`            `@ 0x00ABC88C`

Those LaserDrawClass instances are what the tactical renderer actually rasterises (screen-space lines with fade/thickness) — separate rendering subsystem, not covered here.

### Destruction

Two disposal routes, both eventually invoking `DiskLaserClass::Destructor` (`0x004A7C90`):

1. **Natural end-of-animation** (`state = -1` set by `AI`): the helper `FUN_0x004A7FE0` pushes `this` onto a deferred-free list; a later cleanup pass calls the scalar-deleting destructor.
2. **Source destroyed mid-animation**: `TechnoClass::ProcessCellAction` / `ObjectClass::UnInit` walks `g_DiskLaserClass_Array` and calls `DiskLaserClass__DetachFromObject` (`0x004A7900`), which sets `state=-1` if the removed object was this DiskLaser's source or target, then flags it for cleanup via the same helper.

The destructor:
- Linearly scans `g_DiskLaserClass_Array` for this pointer and shifts all later entries left by one.
- Decrements `g_DiskLaserClass_Array_Count`.
- Calls `AbstractClass::Destructor_ResetVtables` then, if `scalar_flag != 0`, frees via `operator delete`.

---

## Call graph (per-caller YR-reachability verdict)

| Caller | Live in YR? | Evidence |
|---|---|---|
| `TechnoClass::Fire_At @ 0x006FDD50` (via `0x006FE47E`) | **YES** | Triggered whenever a weapon with `DiskLaser=yes` fires. YR's `[DiskLaser]` (Floating Disc Primary) and `[DiskLaserE]` (ElitePrimary) both set `DiskLaser=yes`. |
| `LogicClass::PerTickUpdate @ 0x0055B5A1` (iterates array) | **YES** | Called once per sim tick unconditionally. |
| `MapClass__ShutdownCleanup @ 0x00534450` (destroys remaining DiskLasers at shutdown) | **YES** | Called during game teardown. |
| `ObjectClass::UnInit / TechnoClass::ProcessCellAction` → `DiskLaserClass::DetachFromObject` | **YES** | Runs when any ObjectClass is removed; always iterates the DiskLaser array. |
| `FUN_0x007258D0` (ObjectClass removal dispatcher) | **YES** | Runs for every TechnoClass removal; iterates DiskLasers to detach references. |

No caller is behind a `SpecialFlags` gate or a TS-only code path. There are no dormant call sites.

---

## INI keys (verified)

| Key | Section | Struct | Offset | Type | Status |
|---|---|---|---|---|---|
| `DiskLaser` | (any weapon) | WeaponTypeClass | `0x14A` | bool | **LIVE** — the ONLY switch to activate DiskLaserClass |
| `IsHouseColor` | (any weapon) | WeaponTypeClass | `0x14D` | bool | LIVE — toggles house-color vs laser-fields color in the ring |
| `LaserInnerColor` | (any weapon) | WeaponTypeClass | `0x120` | RGB | LIVE — inner ring color |
| `LaserOuterColor` | (any weapon) | WeaponTypeClass | `0x123` | RGB | LIVE — outer ring color |
| `LaserOuterSpread` | (any weapon) | WeaponTypeClass | `0x126` | RGB | LIVE — "fade / spread" channel |
| `DiskLaserChargeUp` | (any weapon?) | (TBD) | — | SoundType | String `"DiskLaserChargeUp"` at `0x0083A670` is xref'd from `RulesClass::ReadAudioVisual` @ `0x0066A366`. Stored in `Rules+?` and played at ring start. INI example: `DiskLaserChargeUp=FloatingDiscChargeUp` in `rulesmd.ini:737`. |
| `Damage` / `Warhead` / `Range` / `Report` / `ROF` etc. | weapon | WeaponTypeClass | std offsets | mixed | normal weapon fields read by `DiskLaserClass::AI` during the fire step |

The RTTI literal `DiskLaser` at `0x00817138` is xref'd by `WeaponTypeClass::ReadINI` at `0x00772645` — this is the definitive confirmation that `0x14A` is the parse target.

Default for `DiskLaser=` is `false` (i.e. normal bullet behaviour). Setting `DiskLaser=yes` makes the weapon skip bullet allocation entirely and use the DiskLaser pipeline.

### Observed YR INI usage

```ini
; rulesmd.ini
[DiskLaser]                              ; Floating Disc Primary
...
LaserInnerColor=216,0,184
LaserOuterColor=80,0,88
LaserOuterSpread=0,0,0
LaserDuration=15
;IsLaser=true                            ; (commented out — use DiskLaser path instead)
DiskLaser=yes                            ; new ring draw laser

[DiskLaserE]                             ; Floating Disc ElitePrimary (same schema)
...
DiskLaser=yes
```

Both `DiskLaser` and `DiskLaserE` weapons are used by `[DISK]` (Floating Disc).

---

## Rendering path

- DiskLaserClass itself does NOT write to the tactical draw queue. It produces visuals indirectly by constructing `LaserDrawClass` objects.
- Each `LaserDrawClass` is a line-segment draw primitive (start/end 3D coords, two RGB colors, duration, fade curve).
- The renderer iterates `g_LaserDraw_Array` during the main map draw pass.
- Layer: lasers draw ABOVE terrain and units (z-buffer bypass with additive/alpha blending depending on weapon flags). The ring segments are drawn in world space, so they respect scroll but not isometric depth sorting.

For the Rust engine, recommended implementation:
- Keep the sim half (DiskLaserClass) separate from the render half (LaserDrawClass).
- Sim: a small `DiskLaserFx` struct holding `{ source_id, target_id, weapon_id, state: DiskLaserState, initial_facing: u8, step: u8, flags: u8 }`; advanced once per tick by the simulation.
- Render: emit short-lived laser segments each step into the existing laser draw queue; the disc laser does not need its own render type.

---

## Open questions

1. **Exact contents of the rotation table at `0x008A0180`.** It is BSS (zero-initialised) at load time and populated by a setup routine (most likely a trig table filled with `(cos(angle)*R, sin(angle)*R)` pairs for 16 evenly-spaced angles with some inner/outer radius). Worth one extra decompile pass on whichever function writes to `0x008A0180` at game startup to extract the exact radius. Low priority — we can match visually by generating our own table.
2. **Voice at ring start (`Rules+0x28C`).** The AI function calls `VocClass::PlayAt` using Rules+0x28C when `iVar7 == 0` (first visual step). The field name is not confirmed; based on context and the `DiskLaserChargeUp` INI key, this is likely the sound that plays BEFORE the final beam (start of charge-up), distinct from the weapon's own Report that plays on FIRE.
3. **Secondary tracker-array registration from `BulletAnimTracker::Register`.** Why are DiskLasers additionally added to `g_0x00B0F6A0..`? Current hypothesis: that array is a generic "active effects that must be torn down when their source/target dies" list, shared with ParticleSystemClass and TagClass. The constructor-side bookkeeping is already captured by `DiskLaserClass::DetachFromObject`; this secondary array is likely redundant for our purposes.
4. **Behaviour with `AreaFire` / `Suicide` / `Burst` flags.** The AI fires exactly ONE terminal beam per DiskLaserClass; burst behaviour is handled by `TechnoClass::Fire_At` re-entering the weapon cycle (it increments `CurrentBurstIndex` and starts a fresh DiskLaserClass for the next shot). Confirmed — no multi-shot handling inside the AI.

---

## Ghidra functions labeled (this session)

| Address | New name | Meaning |
|---|---|---|
| `0x004A7340` | `DiskLaserClass__AI` | Per-tick update; previously `FUN_004A7340` |
| `0x004A7900` | `DiskLaserClass__DetachFromObject` | Removes references when a linked source/target dies; previously `FUN_004A7900` |
| `0x004A79D0` | `DiskLaserClass__MarkForRemoval` | Sets state=-1 and pushes onto cleanup queue; previously `FUN_004A79D0` |
| `0x004A7B80` | `DiskLaserClass__ComputeCRC` | vtable slot 0x34 (already inherited from AbstractClass) |
| `0x004A7B90` | `DiskLaserClass__Load` | vtable slot 0x14 (Load / swizzle resolve) |
| `0x004A7C10` | `DiskLaserClass__Save` | vtable slot 0x18 (Save passthrough) |
| `0x004A7C30` | `DiskLaserClass__GetClassID` | vtable slot 0x0C (returns CLSID bytes at 0x7E9890) |
| `0x004A7C70` | `DiskLaserClass__GetHeapID` | vtable slot 0x30 (returns constant) |
| `0x004A7C80` | `FUN_004A7C80_temp` → (left; returns 0x49 = RTTI abstract-type enum for DiskLaser) | vtable slot 0x2C |
| `0x004A7C90` | `DiskLaserClass__Destructor` | vtable slot 0x20 (scalar-deleting dtor; un-registers from array) |
| `0x0054FE60` | `LaserDrawClass__Constructor` | 0x5C-byte laser segment object used by DiskLaser's AI |
| `0x008A0208` | `g_DiskLaserClass_Array_Vtable` | DynamicVectorClass<DiskLaserClass*> vtable ptr |
| `0x008A020C` | `g_DiskLaserClass_Array` | `DiskLaserClass**` buffer |
| `0x008A0210` | `g_DiskLaserClass_Array_Capacity` | int |
| `0x008A0218` | `g_DiskLaserClass_Array_Count` | int |

Plate comments added to `DiskLaserClass__Constructor` (`0x004A7A30`) with full struct layout and to `DiskLaserClass__AI` (`0x004A7340`) with phase-by-phase algorithm.

`save_program` completed successfully at end of session.

---

## Implementation guidance (for the Rust engine)

- This is required for Floating Disc parity. Treat it as a standard sim-object lifecycle:
  - Create at `Fire_At` when the resolved weapon has `disk_laser = true`. Do NOT produce a bullet.
  - Persist in a small `Vec<DiskLaserFx>` or in the `EntityStore` keyed by u64 entity id.
  - Per-tick: phase-gated step; compute ring offsets from a pre-baked 16-entry trig table (R_inner, R_outer are visible constants once step 4 open question is resolved).
  - Emit laser beams as data rows consumed by the renderer's existing laser pass — no need for a separate render type.
  - On final step: call the damage pipeline with the weapon's warhead/damage/cell-spread; play the Report sound; enqueue the effect for removal next tick.
  - On source death: detach / mark for removal.
- Determinism: DiskLaser state is small (source_id, target_id, weapon_id, state, facing, step, flags — ~24 bytes). Serialise in snapshots. Math must use fixed-point `Math::atan2`-equivalent just like other ballistic math.
- Rendering fidelity: the green rotating ring look comes from (a) rapid succession of two laser segments per frame, (b) the rotation table offsets, (c) a fade curve on each laser segment's alpha. Implementing it requires matching the 16-step period; otherwise the visual will look "wrong".
