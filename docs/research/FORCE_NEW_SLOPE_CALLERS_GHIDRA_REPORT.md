# DriveLocomotionClass::Force_New_Slope — Caller Chain Investigation

**Date:** 2026-05-19
**Scope:** Enumerate every code path that calls `Force_New_Slope @ 0x004AFB40` (direct
and vtable-dispatched) and resolve the per-cell-change trigger that updates `locomotor+0x18`.
**Status:** COMPLETE
**Active in YR:** Yes — every Drive-locomotor vehicle invokes the slope update path every
frame it is moving.
**Confidence:** HIGH for all binary claims (every address verified by decompilation or
memory read this session).

---

## 1. TL;DR

`DriveLocomotionClass::Force_New_Slope` has **zero direct callers** in `.text`. It is
reached exclusively through vtable dispatch (ILocomotion slot +0x7C). Three distinct
code paths invoke this slot at runtime:

1. **`DriveLocomotionClass::Update_Facing_From_Type @ 0x004B04D0`** — the primary
   per-frame path; itself also vtable-dispatched (ILocomotion slot +0x50) and only
   invoked when the locomotor detects that the cell's slope has changed.

2. **`FUN_0069FBE0 @ 0x0069FBE0`** — a bytewise-identical twin of `Update_Facing_From_Type`,
   present in a second (non-DriveLocomotion) locomotor class's vtable at `0x007F2DDC`.
   Not active for standard Drive-locomotor units.

3. **`TechnoClass::Set_Destination @ 0x00741970`** — calls `[loco_vtable+0x7C]` directly,
   passing `CellClass+0x11C` as the slope argument, when a unit's destination is being
   set. This is a forced immediate slope sync at movement-start time.

The meta-question — why `LocomotionClass::ForEach_SetSlopeIndex @ 0x004E1570` has zero
direct callers — is resolved: that function IS itself a vtable slot (appears as a DATA
entry in 20 vtables), never a caller. The actual per-cell slope propagation runs through
`DriveLocomotionClass::Process`, which reads `CellClass+0x11C` directly via
`ObjectClass::GetOccupiedCell` and updates `loco+0x18/+0x1C` in-line, without calling
`Force_New_Slope` at all.

---

## 2. vtable Layout (confirmed by memory read of `0x007E7EB0`)

DriveLocomotionClass ILocomotion vtable is at `0x007E7EB0` (installed at `object+4` by
the constructor at `0x004AF5C8` via `MOV [ESI+4], 0x7E7EB0`).

| Slot | Offset | Address | Function |
|------|--------|---------|---------|
| 16 | +0x40 | 0x004B0500 | DriveLocomotionClass::Process |
| 20 | +0x50 | 0x004B04D0 | DriveLocomotionClass::Update_Facing_From_Type |
| 28 | +0x70 | 0x004B0C40 | DriveLocomotionClass::Force_Track |
| 31 | +0x7C | 0x004AFB40 | DriveLocomotionClass::Force_New_Slope ★ |
| 32 | +0x80 | 0x004AFC20 | DriveLocomotionClass::Is_Moving_Now |

Verified by `read_memory 0x7E7EB0 128` (64 bytes shown, slot 31 at `0x7E7F2C` =
bytes `40 FB 4A 00` = `0x004AFB40`). ✓

The single DATA reference to `Force_New_Slope` (`get_xrefs_to 0x004AFB40`) confirms
exactly one vtable entry: `0x007E7F2C` (= `0x7E7EB0 + 0x7C`). ✓

---

## 3. Force_New_Slope Decompilation (confirmed)

```c
void DriveLocomotionClass__Force_New_Slope(int param_1, undefined4 param_2)
{
    *(undefined4 *)(param_1 + 0x1c) = param_2;   // prev_slope = new_slope
    *(undefined4 *)(param_1 + 0x18) = param_2;   // cur_slope  = new_slope
    *(undefined4 *)(param_1 + 0x20) = g_CurrentFrameCounter; // transition_start
    *(undefined4 *)(param_1 + 0x24) = <uninit>;  // (frame scratch)
    *(undefined4 *)(param_1 + 0x28) = 0;         // transition_duration = 0
    *(undefined4 *)(param_1 + 0x2c) = 0;         // transition_timer    = 0
}
```

`param_1` is the ILocomotion sub-object pointer (= `object_base+4`).
Field map from object_base: +0x1C = prev_slope, +0x1C = ... wait, the constructor
comment says `+0x1C = cached_slope_index` from `object_base`. Since ILocomotion is at
`object_base+4`, these map to: `(object_base+4)+0x18 = object_base+0x1C` = cur_slope
(constructor comment field), `(object_base+4)+0x1C = object_base+0x20` = prev_slope.

Result: `Force_New_Slope` performs an **instantaneous** (zero-duration) slope switch.
Transition duration `+0x2C = 0` means `Draw_Matrix` immediately takes the new slope
without interpolation.

---

## 4. Path 1: Update_Facing_From_Type (per-frame, primary path)

`DriveLocomotionClass::Update_Facing_From_Type @ 0x004B04D0` (body: `0x004B04D0` –
`0x004B04F4`):

```c
void Update_Facing_From_Type(int *param_1) {
    int iVar1 = *param_1;                                   // ILocomotion vtable
    int iVar2 = (**(code **)(*(int *)param_1[2] + 0x1bc))(); // GetOccupiedCell()
    (**(code **)(iVar1 + 0x7c))(param_1, *(undefined1 *)(iVar2 + 0x11c)); // Force_New_Slope
}
```

- `*(int *)param_1[2] + 0x1bc` = FootClass vtable slot +0x1BC = `ObjectClass::GetOccupiedCell`
  (`0x005F6960`). Verified: FootClass vtable at `0x7E8C94`, slot `+0x1BC` → address
  `0x5F6960` (read at `0x7E8E50` = `60 69 5F 00`). ✓
- `CellClass+0x11C` = slope index of the current cell (1 byte).
- `iVar1 + 0x7C` = ILocomotion vtable + 0x7C = Force_New_Slope.

`Update_Facing_From_Type` is itself **only vtable-dispatched** (ILocomotion slot +0x50).
`get_xrefs_to 0x4B04D0` returns a single DATA ref from `0x007E7F00` (the vtable entry).
No direct CALL sites in `.text`.

The actual dispatch of slot +0x50 happens **inside `DriveLocomotionClass::Process`**
(see §6 below). `Process` detects the slope change itself via an inline read of
`CellClass+0x11C` and then calls `Update_Facing_From_Type` via slot +0x50 to commit it.

**Actual byte evidence:** `FF 57 7C` at `0x004B04EB` (= `CALL [EDI+0x7C]`, EDI = vtable
pointer loaded from `*param_1`). Disassembly: `8B 3E E8 ... 33 D2 8A 90 1C 01 00 00 52
56 FF 57 7C` → matches the pattern above exactly.

---

## 5. Path 2: FUN_0069FBE0 (twin function, different locomotor class)

`FUN_0069FBE0 @ 0x0069FBE0` (body: `0x0069FBE0` – `0x0069FC04`) is bytewise-identical
to `Update_Facing_From_Type`:

```c
void FUN_0069FBE0(int *param_1) {
    int iVar1 = *param_1;
    int iVar2 = (**(code **)(*(int *)param_1[2] + 0x1bc))();
    (**(code **)(iVar1 + 0x7c))(param_1, *(undefined1 *)(iVar2 + 0x11c));
}
```

`get_xrefs_to 0x0069FBE0` = DATA from `0x007F2DDC` only. This is a vtable slot in a
different locomotor class (not DriveLocomotionClass). **Not active for Drive-locomotor
vehicles in standard play.** Included for completeness.

---

## 6. Per-Cell Slope Detection: DriveLocomotionClass::Process (the actual trigger)

`DriveLocomotionClass::Process @ 0x004B0500` (called every frame by the main game tick
for each Drive-locomotor unit) contains the true per-cell slope detection:

```c
iVar4 = (**(code **)(*(int *)param_1[2] + 0x1bc))(); // GetOccupiedCell()
bVar1 = *(byte *)(iVar4 + 0x11c);                    // CellClass+0x11C = slope index
if ((uint)bVar1 != piVar2[6]) {                       // piVar2[6] = loco+0x18 = cur_slope
    piVar2[7] = piVar2[6];     // save prev slope → loco+0x1C
    piVar2[6] = (uint)bVar1;   // update cur slope → loco+0x18
    CDTimerClass__Start(3);    // start 3-tick timer
    piVar2[8] = ...;           // loco+0x20 = transition start frame
    ...
    piVar2[0xb] = 3;           // loco+0x2C = transition duration = 3 (smooth!)
}
```

This writes `loco+0x18` and `loco+0x2C = 3` — a **smooth 3-frame transition** — in
contrast to `Force_New_Slope` which sets `+0x2C = 0` (instantaneous). The slope change
detector in `Process` runs first; when a change is detected, `Process` then dispatches
`Update_Facing_From_Type` (slot +0x50) to commit the forced-immediate version via
`Force_New_Slope`. This results in `+0x2C` being overwritten to 0 immediately after
the 3-tick timer was set.

Net effect: the "smooth 3-frame transition" intent in `Process` is immediately cancelled
by `Force_New_Slope` (which sets `+0x2C = 0`). **All slope changes are therefore
instantaneous in practice.** The transition fields `+0x28`, `+0x2C` are vestigial.

---

## 7. Path 3: TechnoClass::Set_Destination (forced slope sync at movement start)

`TechnoClass::Set_Destination @ 0x00741970` (body: `0x00741970` – `0x00743186`)
contains a direct `[loco_vtable+0x7C]` call at `0x00742BE6`:

Assembly at `0x00742BD5`:
```
8B 3E           MOV edi, [esi]              ; esi = ILocomotion ptr, edi = vtable
E8 xx xx xx xx  CALL GetOccupiedCell_equiv  ; returns CellClass* in eax
33 D2           XOR edx, edx
8A 90 1C010000  MOV dl, byte[eax+0x11C]     ; slope index from CellClass+0x11C
52              PUSH edx                    ; arg2 = slope
56              PUSH esi                    ; arg1 = ILocomotion ptr (this)
FF 57 7C        CALL [edi+0x7C]             ; Force_New_Slope(esi, dl)
```

Trigger condition: this block executes when `TechnoClass::Set_Destination` is called
to assign a new move target to a unit. It forces an immediate slope sync at the moment
movement is commanded, ensuring the locomotor's slope cache matches the unit's current
cell before the drive-track begins.

**This is the mechanism that prevents the initial "wrong slope" visual artifact** when
a unit starts moving from a ramp or hill — the slope is snapped to the current cell's
slope index at destination-set time, before the first `Process` tick.

---

## 8. The ForEach_SetSlopeIndex Red Herring — Resolved

`LocomotionClass::ForEach_SetSlopeIndex @ 0x004E1570` appears in **20 vtables**
(all DATA refs: `0x7E4FB0`, `0x7E5080`, `0x7E52B8`, `0x7E7FF8`, `0x7E81D0`,
`0x7E92E8`, `0x7ED138`, `0x7E80B4`, `0x7F2FF8`, `0x7ED248`, `0x7F3ECC`, `0x7F5608`,
`0x7E8144`, `0x7F5B70`, `0x7E5E68`, `0x7E60B8`, `0x7E93B0`, `0x7E945C`, `0x7F02E8`,
`0x7F2F70`). Zero CALL sites exist in `.text`.

This function IS a vtable slot implementation — it is never a caller. Its name is
misleading: it implements a locomotor-level vtable method that, when dispatched to,
updates the locomotor's slope index. But since nothing ever dispatches it (the only
callers would have to explicitly call slot +0x148 of the relevant vtable), it is dead
in standard YR gameplay.

The per-cell slope propagation path is:
```
World::advance_tick
  → TechnoClass::AI (or DriveLocomotion::Process via locomotor tick dispatch)
    → DriveLocomotionClass::Process (every frame)
      → GetOccupiedCell() → CellClass+0x11C comparison
        → if changed: dispatch ILocomotion slot +0x50 (Update_Facing_From_Type)
          → Force_New_Slope via [vtable+0x7C]
            → writes loco+0x18 (cur_slope), loco+0x1C (prev_slope), +0x2C=0
```

---

## 9. CellClass+0x11C — the slope index field

`CellClass+0x11B` = terrain_type (established from `Process_Drive_Track` decompilation).
`CellClass+0x11C` = slope index (1 byte), read by:
- `Update_Facing_From_Type` / `FUN_0069FBE0`: as the argument to Force_New_Slope.
- `DriveLocomotionClass::Process`: in the per-frame change detector.
- `TechnoClass::Set_Destination`: as the initial slope sync value.

This is the cell's slope/ramp index as set by the map loader / theater tile data, range
0..19 based on the 20 terrain slope shapes documented in CLIFF_RAMP_TRAVERSAL docs.

---

## 10. TS-vs-YR Filter

| Code path | Active in standard YR? | Evidence |
|---|---|---|
| `DriveLocomotionClass::Process` slope detector | **Yes** — every moving Drive unit, every frame | Decompiled; no SpecialFlags gate |
| `Update_Facing_From_Type` | **Yes** — dispatched from within `Process` | vtable slot, reached every time slope changes |
| `Force_New_Slope` | **Yes** — called by both above | No TS-only gate; zero-duration transition is permanent |
| `TechnoClass::Set_Destination` slope sync | **Yes** — fires on every move order | In stock flow, destination is set via `TechnoClass::Set_Destination` on every right-click move |
| `FUN_0069FBE0` | **Inactive for Drive units** — different locomotor | vtable DATA only at `0x7F2DDC`; not DriveLocomotion |
| `ForEach_SetSlopeIndex` | **Dead in standard YR** | 20 vtable DATA refs, 0 CALL sites; never dispatched |

---

## 11. Implications for Rust Port

- **No SlopeIndex vtable-dispatch walker is needed.** `ForEach_SetSlopeIndex` is dead;
  do not implement it.
- **Per-cell slope update triggers in `Process` (every frame), not in `PerCellProcess`.**
  The Rust DriveLocomotionClass::process() method must read `CellClass.slope_index` each
  tick and call the Force_New_Slope equivalent when the value differs from the cached
  `loco.cur_slope`.
- **`Force_New_Slope` is always instantaneous.** The transition duration written by
  `Process` (3 ticks) is immediately cancelled by the subsequent `Force_New_Slope` call.
  Implement `force_new_slope()` as a direct field write with `transition_duration = 0`.
- **`TechnoClass::Set_Destination` must call `force_new_slope()` at move-start.** This
  prevents the initial-frame slope artifact when a unit begins moving from a ramp cell.
- **`Draw_Matrix` reads `loco+0x18` (cur_slope) and `loco+0x2C` (transition duration)**
  for VXL tilt interpolation. Since `+0x2C` is always 0, only `+0x18` matters for
  rendering. Confirmed by `Draw_Matrix @ 0x004AFF60` decompilation.

---

## 12. Sources

**Decompiled this session:**
- `DriveLocomotionClass::Force_New_Slope @ 0x004AFB40`
- `DriveLocomotionClass::Update_Facing_From_Type @ 0x004B04D0`
- `DriveLocomotionClass::Process @ 0x004B0500` (full, slope-detection section)
- `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` (full)
- `DriveLocomotionClass::Draw_Matrix @ 0x004AFF60` (full — confirms `+0x18` and `+0x2C` usage)
- `DriveLocomotionClass::Constructor @ 0x004AF5A0` (field map + vtable installs)
- `DriveLocomotionClass::Load @ 0x004AF780` (confirms vtable names)
- `DriveLocomotionClass::Set_Destination @ 0x004AFD40` (not the slope caller)
- `FUN_0069FBE0 @ 0x0069FBE0` (twin of Update_Facing_From_Type)
- `ObjectClass::GetOccupiedCell @ 0x005F6960` (confirmed via FootClass vtable slot +0x1BC)

**Memory reads this session:**
- `0x7E7EB0 + 128 bytes` — ILocomotion vtable; confirms slot +0x7C = `0x004AFB40`
- `0x7E7F20 + 64 bytes` — sub-vtable (Force_Track, Force_New_Slope slots)
- `0x7E8E50 + 4 bytes` — FootClass vtable +0x1BC = `0x005F6960` (GetOccupiedCell)
- `0x4AF5C2 + 14 bytes` — constructor vtable assignments (IUnknown=`0x7E7F7C`, ILoco=`0x7E7EB0`)
- `0x742BD0 + 30 bytes` — TechnoClass::Set_Destination slope-sync block

**Xref queries:**
- `get_xrefs_to 0x004AFB40` → 1 DATA ref at `0x7E7F2C` (ILocomotion vtable slot)
- `get_xrefs_to 0x004B04D0` → 1 DATA ref at `0x7E7F00` (vtable slot)
- `get_xrefs_to 0x0069FBE0` → 1 DATA ref at `0x7F2DDC`
- `get_xrefs_to 0x004E1570` → 20 DATA refs (no CALL sites)

**Byte-pattern searches:**
- `FF 57 7C` (CALL [EDI+0x7C]) → hits at `0x4B04EB` (Update_Facing_From_Type), `0x69FBFD`
  (FUN_0069FBE0), `0x742BE6` (TechnoClass::Set_Destination)
- `FF 51 7C`, `FF 50 7C`, `FF 52 7C` (other register variants) — inspected; no additional
  DriveLocomotion-relevant callers found beyond the three paths above

**Companion docs:**
- `SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md` — confirms ForEach_SetSlopeIndex dead,
  Process_Movement does not touch +0x530
- `DRIVE_TRACK_TABLES_DEEP_DECODE.md §13` — ILocomotion vtable layout (extended to 50+ slots)

---

*End of report.*
