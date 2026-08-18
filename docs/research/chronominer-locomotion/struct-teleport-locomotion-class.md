# TeleportLocomotionClass — struct layout decode

**Kind:** struct
**Total size:** 0x4c bytes (76), verified via `operator new(0x4c)` in constructor call site
**Proposed Ghidra struct field renames:** see table at bottom

---

## Summary

`TeleportLocomotionClass` implements three COM vtable interfaces: IUnknown (`+0x00`),
ILocomotion (`+0x04`), and IPiggyback (`+0x18`). It holds two destination-coordinate
caches (both initialised to the g_NullCoord sentinel), a 1-byte state counter at `+0x34`
for the 7-state warp machine, a warp-count field at `+0x38`, and a 2-field frame-based
timer at `+0x3C/+0x44`.

Verified via `decompile_function 0x00718000` (constructor), `decompile_function 0x00719BF0`
(TimerCheck — timer fields), `decompile_function 0x00719E30` through `0x00719F30`
(COM stubs — IPiggyback fields), and `decompile_function 0x007192F0` (StateMachineTick).

---

## Active in YR

**Yes.** Constructor called from COM factory every time a unit with `Teleporter=yes`
activates its warp locomotor. All fields are written by YR-live code paths. No gating flag.

---

## Layout table

`param_1` in Constructor (`0x00718000`) is `undefined4*`; index N × 4 = byte offset.
All offsets below are **full-object** byte offsets (not sub-object offsets).

| Byte offset | Constructor expression | Init value | Field name | Confidence | Notes |
|---|---|---|---|---|---|
| `+0x00` | `*param_1 = &...IUnknown_vtable` | `0x007f50cc` | IUnknown_vtable_ptr | HIGH | COM IUnknown |
| `+0x04` | `param_1[1] = &...ILocomotion_vtable` | `0x007f5010` | ILocomotion_vtable_ptr | HIGH | COM ILocomotion |
| `+0x08` | (base ctor) | — | techno_owner_ptr | HIGH | TechnoClass* — read as `param_1[2]` everywhere |
| `+0x0C..+0x14` | `LocomotionClass__Constructor()` | — | base_fields[3] | YELLOW | LocomotionClass base; `+0x08` confirmed as owner ptr |
| `+0x18` | `param_1[6] = &...IPiggyback_vtable` | `0x007f4fe0` | IPiggyback_vtable_ptr | HIGH | COM IPiggyback |
| `+0x1C` | `param_1[7] = g_NullCoord_Teleport_X` | sentinel 0 | dest_cache_0_x | HIGH | HeadToCoord writes; Destination reads |
| `+0x20` | `param_1[8] = g_NullCoord_Teleport_Y` | sentinel 0 | dest_cache_0_y | HIGH | HeadToCoord/Destination |
| `+0x24` | `param_1[9] = g_NullCoord_Teleport_Z` | sentinel 0 | dest_cache_0_z | HIGH | HeadToCoord/Destination |
| `+0x28` | `param_1[10] = g_NullCoord_Teleport_X` | sentinel 0 | dest_cache_1_x | HIGH | Process output X; UpdatePosition reads |
| `+0x2C` | `param_1[0xb] = g_NullCoord_Teleport_Y` | sentinel 0 | dest_cache_1_y | HIGH | Process output Y |
| `+0x30` | `param_1[0xc] = g_NullCoord_Teleport_Z` | sentinel 0 | dest_cache_1_z / piggyback_slot | MEDIUM | **Dual-use**: live dest Z AND Begin_Piggyback stores Drive locomotor ptr here (full-obj +0x30); End_Piggyback reads from IPiggyback-this+0x30 = full-obj+0x48 — see YELLOW |
| `+0x34` | `*(undefined1*)(param_1+0xd) = 0` | 0 | state | HIGH | Byte field; param_1+0xd in undefined4* ptr arith = byte offset 0x34; state machine 0..7 |
| `+0x35` | `*(undefined1*)((int)param_1+0x35) = 0` | 0 | flag_35 | LOW | Direct byte write; purpose YELLOW |
| `+0x36` | `*(undefined1*)((int)param_1+0x36) = 0` | 0 | flag_36 | LOW | Direct byte write; Stop_Moving clears sub-obj +0x32 = full-obj +0x36; purpose YELLOW |
| `+0x37` | — | — | (padding) | — | Not written by constructor |
| `+0x38` | `param_1[0xe] = 0` | 0 | warp_count | HIGH | Incremented on timer expiry if > 0 (TimerCheck @ 0x00719BF0) |
| `+0x3C` | `param_1[0xf] = g_CurrentFrameCounter` | current frame | timer_start_frame | HIGH | Timer reference frame; -1 = timer not armed; compared to g_CurrentFrameCounter in TimerCheck |
| `+0x40` | (not written) | — | field_40 | YELLOW | Not initialised by constructor; possible padding or base-class use |
| `+0x44` | `param_1[0x11] = 0` | 0 | timer_duration_frames | HIGH | Delay in frames; 0 = instant; TimerCheck expiry: elapsed >= duration |
| `+0x48` | `param_1[0x12] = 0` | 0 | field_48 | YELLOW | Cleared by constructor; possibly actual Begin_Piggyback storage slot (see YELLOW) |

Object ends at `+0x4C` (exclusive). Verified via `operator new(0x4c)`.

---

## Timer formula (verified)

Verified via `decompile_function 0x00719BF0` (TimerCheck). `param_1` there is `int`
(direct byte offsets):

```
elapsed = g_CurrentFrameCounter - *(int*)(param_1 + 0x3C);
expired = elapsed >= *(int*)(param_1 + 0x44);
```

- `+0x3C` = timer start frame (from constructor: seeded with `g_CurrentFrameCounter`)
- `+0x44` = timer duration in frames (from constructor: init 0 = instant)
- `+0x38` = warp_count: incremented on each timer expiry if `*(int*)(param_1+0x38) > 0`

---

## Coordinate frame annotations

**dest_cache_0** (`+0x1C/+0x20/+0x24`) — HeadToCoord destination.
Written by `HeadToCoord` from the Process-validated result. Read by `Destination`.
Frame: location-space leptons (NW-cell frame), matching TechnoClass+0x9C/+0xA0/+0xA4.

**dest_cache_1** (`+0x28/+0x2C/+0x30`) — Process-output buffer.
Written by `Process` after validating the warp target cell. Read by `HeadToCoord`.
Sentinel: both caches initialised to `g_NullCoord_Teleport_X/Y/Z` = `(0, 0, 0)`.
Frame: location-space leptons.

Map cell (0,0) is always impassable border, making (0,0,0) a safe sentinel for "no valid
destination." Verified via `read_memory 0x00B0EBF8` (12 bytes, all zeros) in
fn-constructor.md. (Address corrected from earlier 0x00B0EBD8.)

---

## Vtable slot summary

### ILocomotion vtable (`0x007f5010`)

Verified from constructor `param_1[1]` = `&TeleportLocomotionClass__ILocomotion_vtable`.

| Slot | Vtable offset | Address | Name |
|---|---|---|---|
| 0 | +0x00 | `0x00718080` | Is_Moving |
| 1 | +0x04 | `0x007180A0` | Destination |
| 2 | +0x08 | `0x007192F0` | StateMachineTick |
| 3 | +0x0C | `0x00718100` | HeadToCoord |
| 17 | +0x44 | `0x00718230` | Stop_Moving |
| 25 | +0x64 | `0x007192C0` | Mark_All_Occupation_Bits (YELLOW) |

### IUnknown vtable (`0x007f50cc`)

| Slot | Vtable offset | Address | Name |
|---|---|---|---|
| 0 | +0x00 | `0x00719E30` | QueryInterface |
| 1 | +0x04 | — | AddRef (standard COM stub) |
| 2 | +0x08 | — | Release (standard COM stub) |

### IPiggyback vtable (`0x007f4fe0`)

| Slot | Vtable offset | Address | Name |
|---|---|---|---|
| 2 | +0x08 | `0x00719E90` | Begin_Piggyback |
| 3 | +0x0C | `0x00719EE0` | End_Piggyback |
| 4 | +0x10 | `0x00719F30` | Is_Ok_To_End |

---

## TechnoClass-side fields consumed

These are **TechnoClass** fields accessed via `*(int*)(TeleportLocomotionClass+0x08)`.
Documented here to avoid offset-confusion when porting to Rust.

All offsets are TechnoClass direct byte offsets.

| TechnoClass offset | Name | Role |
|---|---|---|
| `+0x8C` | bridge_on_destination | Set when dest cell has bridge overlay (UpdatePosition mode 1) |
| `+0x21C` | owning_house | HouseClass ptr; power-surplus check in PostWarpValidation |
| `+0x271` | warp_anim_gate | WarpingOut flag; cleared to 0 on timer expiry (TimerCheck) |
| `+0x27C` | chrono_in_transit | ChronoInTransit flag; Is_Ok_To_End checks == 0 |
| `+0x280` | warp_state | Pending warp counter; PostWarpValidation guard |
| `+0x284` | chrono_delay_countdown | ChronoDelay ticks from Rules+0xBEC; state 3/5 in StateMachineTick |
| `+0x288` | dest_x | Destination X leptons (NW-cell frame); read in StateMachineTick state 2 |
| `+0x28C` | dest_y | Destination Y leptons; read in state 2 |
| `+0x290` | dest_z | Destination Z leptons; read in state 2 |
| `+0x2B4` | targeting_active | 0 = no active target; TimerCheck re-engages weapons when 0 |
| `+0x2D8` | linked_anim_or_obj | Anim/helper ptr; cleared in PostWarpValidation death path |
| `+0x3CD` | falling_dying_flag | Set to 1 in PostWarpValidation on water/impassable landing |
| `+0x428` | source_building_ptr | Kill-credit arg 1; cleared by End_Piggyback |
| `+0x42C` | source_house_ptr | Kill-credit arg 2; cleared by End_Piggyback |
| `+0x5A4` | radio_link | Radio link ptr; Process checks radio state 1/2/6 before dock commit |
| `+0x694` | warp_attach_ptr | WarpAttachClass ptr; detached before warp in InitiateWarp |
| `+0x9C` | location_x | Location X leptons (NW-cell frame); anim spawn coords |
| `+0xA0` | location_y | Location Y leptons |
| `+0xA4` | location_z | Location Z leptons |

---

## Proposed Ghidra struct field renames

No Ghidra struct type exists for `TeleportLocomotionClass` (verified via `get_struct_layout`
returning "Structure not found"). Renames apply when the struct type is created.

| Field offset | Proposed name | Confidence | Rationale |
|---|---|---|---|
| `+0x1C` | dest_cache_0_x | HIGH | HeadToCoord writes this as output X |
| `+0x20` | dest_cache_0_y | HIGH | HeadToCoord writes this as output Y |
| `+0x24` | dest_cache_0_z | HIGH | HeadToCoord writes this as output Z |
| `+0x28` | dest_cache_1_x | HIGH | Process writes validated dest X |
| `+0x2C` | dest_cache_1_y | HIGH | Process writes validated dest Y |
| `+0x30` | dest_cache_1_z | HIGH | Process writes validated dest Z (dual-use — see YELLOW) |
| `+0x34` | state | HIGH | State machine byte 0..7 |
| `+0x35` | flag_35 | LOW | Purpose unknown |
| `+0x36` | flag_36 | LOW | Stop_Moving clears; purpose unknown |
| `+0x38` | warp_count | HIGH | Incremented per completed warp cycle |
| `+0x3C` | timer_start_frame | HIGH | g_CurrentFrameCounter when timer armed; -1 = not armed |
| `+0x44` | timer_duration_frames | HIGH | Frames to wait; 0 = instant expiry |
| `+0x48` | field_48 | LOW | Cleared by constructor; possible alt piggyback slot |

---

## Out-of-scope refs

None — struct decode only. All function cross-references covered by individual decode tasks.

---

## Unverified / YELLOW

- **`+0x30` dual-use / piggyback slot conflict**: Constructor writes `dest_cache_1_z` here.
  Begin_Piggyback (0x00719E90, param_1 is full-object `this`) also writes Drive locomotor ptr
  at `*(int*)(param_1+0x30)` = full-obj +0x30. BUT End_Piggyback receives `this = base+0x18`
  (IPiggyback vtable ptr), so its `*(int*)(param_1+0x30)` = full-obj +0x48. This means
  Begin_Piggyback stores at +0x30 while End_Piggyback reads from +0x48 — apparent mismatch.
  Either Begin_Piggyback also receives IPiggyback-this (then +0x30 = +0x48) or the two use
  different slots. Resolution requires decompiling Begin_Piggyback `this` type directly.
  YELLOW until confirmed.

- **`+0x40` field**: Not written by constructor. Not read in any decompiled function in this
  session. May be padding, a base-class slot, or a field used by StateMachineTick states
  not yet decoded. YELLOW.

- **`+0x44` vs stub's `timer_ticks` at +0x40**: The existing stub (decoder-3 draft) placed
  `timer_ticks` at +0x40 and `field_44` at +0x44. Corrected here: TimerCheck reads
  duration from `param_1+0x44` (int, direct byte offset), not +0x40. Constructor writes
  `param_1[0x11]` = 0 at +0x44, confirms. +0x40 is NOT written by constructor. YELLOW on
  what +0x40 actually stores.

- **Base-class fields `+0x08..+0x17`**: Set by `LocomotionClass__Constructor`. Layout not
  decoded in this session. `+0x08` confirmed as TechnoClass owner ptr from all accessor
  decompiles. The 12 bytes at `+0x0C..+0x17` are LocomotionClass base fields. YELLOW.

- **`+0x3C` seeded with g_CurrentFrameCounter**: Constructor writes `param_1[0xf] = g_CurrentFrameCounter`.
  TimerCheck treats +0x3C as start-frame (elapsed = now - start). The seeding at construction
  means the timer is "pre-armed" at birth with the construction frame — any subsequent
  Duration=0 check will expire immediately. Whether this is intentional or just a safe
  default is not confirmed. YELLOW on intent.
