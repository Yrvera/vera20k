---
title: ILocomotion COM Protocol — Unified Specification
date: 2026-04-24
scope: gamemd.exe — consolidates vtable layout, concrete implementations, and FootClass call sites
confidence: HIGH (every vtable address and slot target verified by live Ghidra read today)
active_in_yr: Yes — every mobile unit in YR routes through this interface every tick
supersedes_sections:
  - DRIVE_LOCOMOTION_CLASS.md §"ILocomotion Vtable — Complete 40-Slot Map" (spec is Drive-specific there; this doc is interface-wide)
  - DRIVE_LOCOMOTION_CLASS.md §"IPiggyback Interface — Complete Vtable" (corrects Walk row: Walk **does** implement IPiggyback)
  - FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md line 23/63/312/395 (corrects `vtable+0x10` label from "Is_Moving_Now" to "Is_Moving")
---

# ILocomotion COM Protocol — Unified Specification

## 0. Why this doc exists

`FootClass+0x674` holds an `ILocomotion*`. Every tick, `FootClass::AI` dispatches through
that pointer — which means the locomotion interface sits in the hot path for every mobile
unit in the game. The vtable layout, the slot contracts, and the FootClass call sites
were previously scattered across:

- `DRIVE_LOCOMOTION_CLASS.md` (Drive's impl + slot map — most complete but Drive-centric)
- `FOOTCLASS_COMPLETE_GHIDRA_REPORT.md`, `FOOTCLASS_PATHFINDING_AND_MOVEMENT.md`,
  `FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md`, `FOOTCLASS_AI_GHIDRA_REPORT.md`
  (call-site references using `vtable+0xNN` notation with no unified table)
- Ten sibling `*_LOCOMOTION_*.md` docs (per-class impls with no shared index)

This doc is the single source of truth for the **interface**: vtable addresses for every
concrete locomotor, the canonical 40-slot contract (with reference impl per slot), and the
FootClass state-machine hooks that drive those slots. It does **not** re-document per-class
internals — those stay in their dedicated reports; this doc cross-references them.

---

## 1. COM interface hierarchy

Every locomotor object has a fixed head:

```
+0x00  IUnknown    vtable  (class-specific)
+0x04  ILocomotion vtable  (class-specific)  ← the pointer stored at FootClass+0x674
+0x08  linked FootClass*   (from Link_To_Object, duplicated at +0x04 and +0x08)
+0x0C  linked FootClass*   (same pointer, duplicated)
+0x18  IPiggyback  vtable  (class-specific, ONLY when the class implements IPiggyback)
```

The "linked techno" back-pointer stored at both `+0x08` and `+0x0C` from the IUnknown
base (equivalently `+0x04` and `+0x08` from the ILocomotion vtable pointer's owning
object) is written by `Link_To_Object` — slot 3 of ILocomotion. All other fields past
`+0x18` are class-specific.
(corrected 2026-05-28: was "+0x04 and +0x08 from IUnknown base / +0x00 and +0x04 from ILocomotion owning object"; binary shows `LocomotionClass__Link_To_Object` at `0x0055A710` writes `*(param_1+4)=param_2` and `*(param_1+8)=param_2` where param_1 is the ILocomotion sub-object ptr at object+0x04, landing at object+0x08 and +0x0C — consistent with the layout table in this same section — ROOT_CAUSE: PARAM1_TYPE_MISREAD)

- **ILocomotion** inherits from **IUnknown** → slots 0/1/2 of the ILocomotion vtable
  are `QueryInterface` / `AddRef` / `Release`, followed by 37 ILocomotion-specific
  methods (slots 3..39, total 40 slots, `0xA0` bytes).
- **IPiggyback** also inherits from IUnknown → 8 slots total (0/1/2 = IUnknown,
  3..7 = IPiggyback-specific).
- When a class has both ILocomotion and IPiggyback, its IUnknown vtable at `+0x00`
  handles raw COM `QueryInterface` for either interface (returns the right sub-vtable
  pointer). The other vtables' QI slots are **adjustor thunks** (`sub [esp+4], 4; jmp`)
  that adjust `this` back to the object base before calling the real QI.

---

## 2. Concrete locomotors — vtable addresses

All 11 concrete locomotor classes in `gamemd.exe`, verified by disassembling each
constructor and reading the literal `MOV dword ptr [ESI+N], <addr>` for each vtable
slot. **The "Base" row is the abstract `LocomotionClass` vtable written by
`LocomotionClass::Constructor` (0x0055a6c0) before the derived constructor overwrites
it** — objects never remain with this vtable at runtime but it appears briefly during
construction/destruction sequences.

| Class                 | Object size | Constructor  | IUnknown vtable | **ILocomotion vtable** | IPiggyback vtable | CLSID |
|-----------------------|-------------|--------------|-----------------|------------------------|-------------------|-------|
| *(Base LocomotionClass)*| —         | `0x0055A6C0` | —               | **`0x007EADF4`**       | —                 | — (abstract) |
| **DriveLocomotionClass**   | 0x6C  | `0x004AF540` | `0x007E7F7C`    | **`0x007E7EB0`**       | `0x007E7E8C`      | `{4A582741-9839-11d1-B709-00A024DDAFD1}` |
| **DropPodLocomotionClass** | ~0x30 | `0x004B5AB0` | `0x007E8344`    | **`0x007E8278`**       | `0x007E8254`      | `{4A582745-9839-11d1-B709-00A024DDAFD1}` |
| **FlyLocomotionClass**     | 0x60  | `0x004CC9A0` | `0x007E8AC0`    | **`0x007E89F4`**       | — (none)          | `{4A582746-9839-11d1-B709-00A024DDAFD1}` |
| **HoverLocomotionClass**   | ~0x74 | `0x00513C20` | `0x007EADC8`    | **`0x007EACFC`**       | — (none)          | `{4A582742-9839-11d1-B709-00A024DDAFD1}` |
| **JumpjetLocomotionClass** | ~0x98 | `0x0054AC40` | `0x007ECE34`    | **`0x007ECD68`**       | `0x007ECD44`      | `{92612C46-F71F-11d1-AC9F-006008055BB5}` |
| **MechLocomotionClass**    | ~0x34 | `0x005AFEF0` | `0x007EDC38`    | **`0x007EDB6C`**       | — (none)          | `{55D141B8-DB94-11d1-AC98-006008055BB5}` |
| **RocketLocomotionClass**  | ~0x5C | `0x00661EC0` | `0x007F0BE8`    | **`0x007F0B1C`**       | — (none)          | `{B7B49766-E576-11d3-9BD9-00104B972FE8}` |
| **ShipLocomotionClass**    | 0x6C  | `0x0069EC50` | `0x007F2E58`    | **`0x007F2D8C`**       | `0x007F2D68`      | `{2BEA74E1-7CCA-11d3-BE14-00104B62A16C}` |
| **TeleportLocomotionClass**| ~0x4C | `0x00718000` | `0x007F50CC`    | **`0x007F5000`**       | `0x007F4FDC`      | `{4A582747-9839-11d1-B709-00A024DDAFD1}` |
| **TunnelLocomotionClass**  | ~0x3C | `0x00728A00` | `0x007F5AF0`    | **`0x007F5A24`**       | — (none)          | `{4A582743-9839-11d1-B709-00A024DDAFD1}` |
| **WalkLocomotionClass**    | ~0x3C | `0x0075AA90` | `0x007F6AC4`    | **`0x007F69F8`**       | `0x007F69D4`      | `{4A582744-9839-11d1-B709-00A024DDAFD1}` |

### 2.1 IPiggyback support matrix

Six of the eleven locomotors implement IPiggyback (writes a vtable at object `+0x18`);
five don't. **Walk, Jumpjet, and DropPod** implementing IPiggyback is easy to miss —
their deploy/entry mechanics don't obviously suggest it, but the vtable is there.

| Implements IPiggyback | Does NOT implement IPiggyback |
|-----------------------|-------------------------------|
| DriveLocomotionClass  | FlyLocomotionClass            |
| ShipLocomotionClass   | HoverLocomotionClass          |
| TeleportLocomotionClass | MechLocomotionClass         |
| JumpjetLocomotionClass| RocketLocomotionClass         |
| DropPodLocomotionClass| TunnelLocomotionClass         |
| WalkLocomotionClass   |                               |

> **Correction to `DRIVE_LOCOMOTION_CLASS.md`.** The piggyback table in that doc
> lists Walk as "No". Verified otherwise: `WalkLocomotionClass__Constructor` at
> `0x0075AA90` writes `MOV dword ptr [ESI+0x18], 0x7F69D4`, and the vtable at
> `0x7F69D4` is a full 8-slot IPiggyback vtable with Begin/End/Is_Ok_To_End thunks
> into Walk-specific code (`0x0075C850`..`0x0075CBA0`).

### 2.2 Link_To_Object override matrix

Only three locomotors override slot 3 (`Link_To_Object`) with their own implementation;
the other eight (plus Base) use `LocomotionClass__Link_To_Object` at `0x0055A710`.
Verified by xrefs to `0x0055A710` (9 DATA references = 8 derived + 1 base).

- **Overrides slot 3:** Fly, Hover, Jumpjet (these classes have their own link semantics,
  typically because they store the techno at a different offset or need additional init)
- **Uses base slot 3:** Drive, DropPod, Mech, Rocket, Ship, Teleport, Tunnel, Walk

---

## 3. Canonical ILocomotion vtable — 40-slot contract

**DriveLocomotionClass is the reference implementation** (`0x007E7EB0`). Every other
locomotor has the same 40-slot layout; "Base" columns indicate the shared
`LocomotionClass` method used when the derived class doesn't override that slot.

All Drive addresses below were re-verified today by reading `0x007E7EB0..0x007E7F50`
live and decoding the little-endian function pointers.

| Slot | Offset | Drive addr    | Method                         | Base fallback (`LocomotionClass` addr) | Purpose |
|------|--------|---------------|--------------------------------|----------------------------------------|---------|
| 0    | +0x00  | 0x004B4D90    | `QueryInterface`               | `0x0055A9B0` (via adjustor thunk)      | COM QI — returns `ILocomotion*` for IID_ILocomotion, `IPiggyback*` for IID_IPiggyback (when supported) |
| 1    | +0x04  | 0x004B4DA0    | `AddRef`                       | `0x0055A950`                           | COM AddRef — `InterlockedIncrement` on ref count |
| 2    | +0x08  | 0x004B4DB0    | `Release`                      | `0x0055A970`                           | COM Release — `InterlockedDecrement`, calls dtor when count hits 0 |
| 3    | +0x0C  | base          | **`Link_To_Object`**           | **`0x0055A710`**                       | Store `FootClass*` at object `+0x04` and `+0x08`; called once at attach |
| 4    | +0x10  | 0x004AFB80    | **`Is_Moving`**                | `0x0055ACD0`                           | Returns `true` iff destination coord ≠ NullCoord. Cheap per-frame poll. **NOT the same as `Is_Moving_Now`.** |
| 5    | +0x14  | 0x004AFC90    | **`Destination`**              | `0x0055AC70`                           | Returns the stored destination `Coord3D` (out by pointer or register triple) |
| 6    | +0x18  | 0x004AFCC0    | **`Head_To_Coord`** *(getter)* | `0x0055ACA0`                           | Returns the current `head_to` waypoint — the **next** cell, not the final destination. Falls back to techno position if no waypoint. |
| 7    | +0x1C  | base          | **`Can_Enter_Cell`**           | `0x0055ABF0` (stub, returns 0)         | Tested before every cell entry; Drive's real check runs through `FootClass::LocomotorPassabilityCheck` (slot 107, +0x1AC on the techno vtable). |
| 8    | +0x20  | base          | `Is_To_Have_Shadow`            | `0x0055ABE0` (returns 1)               | Render hook — suppress shadow for airborne Rocket/Jumpjet/Fly mid-flight |
| 9    | +0x24  | 0x004AFF60    | **`Draw_Matrix`**              | —                                      | Build 3×4 VXL transform — facing interp + slope tilt |
| 10   | +0x28  | 0x004B0410    | `Shadow_Matrix`                | `0x0055A7D0` (Build_Shadow_Matrix)     | Shadow variant of Draw_Matrix |
| 11   | +0x2C  | base          | `Shadow_Point`                 | `0x0055ABD0` (returns {0,0})           | Shadow 2D offset |
| 12   | +0x30  | base          | `Draw_Point`                   | `0x0055A8C0`                           | Extra draw offset `{0, z_adjust}` |
| 13   | +0x34  | base          | `Visual_Character`             | `0x0055ABC0` (returns 0)               | Render LOD enum |
| 14   | +0x38  | 0x004B4870    | `Z_Adjust`                     | `0x0055AB??`                           | Z lift in leptons (Drive returns 0) |
| 15   | +0x3C  | 0x004B4880    | `Z_Gradient`                   | `0x0055ABB0`                           | Z gradient enum (Drive = 2 = Deg45) |
| **16**|**+0x40**|**0x004B0500**|**`Process`**                 | `0x0055AC60` (stub)                    | **MAIN per-tick entry — called from `FootClass::AI` every frame.** Runs the movement state machine, consumes the path queue, drives `Process_Movement` → `Process_Drive_Track` → position update. |
| 17   | +0x44  | 0x004AFD40    | **`Move_To`** *(setter)*       | `0x0055AC??`                           | Set destination. **This is what `FootClass::Set_Destination_Internal` calls.** In Drive, guarded by 4 techno-vtable checks (IsCrashing, IsInRearmTimer, IsWarpingOut, IsBeingWarped) — all must be false. Stores coord at object `+0x30..+0x38`, adds bridge Z if target cell is a bridge. |
| 18   | +0x48  | 0x004AFE00    | **`Stop_Moving`**              | `0x0055A??`                            | Clamp speed to ≤ 0.3, clear waypoint, propagate convoy chain stop. |
| 19   | +0x4C  | 0x004B0EF0    | `Do_Turn`                      | —                                      | Forwards to RateTimer facing interp |
| 20   | +0x50  | 0x004B04D0    | `Unlimbo`                      | —                                      | Init on spawn — reads ROT, inits facing |
| 21   | +0x54  | base          | `Tilt_Pitch_AI`                | `0x0055AB90` (no-op)                   | Optional per-tick body pitch work |
| 22   | +0x58  | base          | `Power_On`                     | `0x0055A8F0`                           | Sets powered flag, self-calls refresh |
| 23   | +0x5C  | base          | `Power_Off`                    | `0x0055A910`                           | Clears powered flag |
| 24   | +0x60  | base          | `Is_Powered`                   | `0x0055A930`                           | Reads powered flag |
| 25   | +0x64  | base          | `Is_Ion_Sensitive`             | `0x0055A940` (returns false)           | Ion-storm vulnerability flag |
| 26   | +0x68  | base          | `Push`                         | `0x0055AB70` (returns false)           | Push-from-cell request |
| 27   | +0x6C  | base          | `Shove`                        | `0x0055AB80` (returns false)           | Stronger push request |
| 28   | +0x70  | 0x004B0C40    | `Force_Track`                  | —                                      | Force-set onto a specific DriveTrack index |
| 29   | +0x74  | 0x004B4820    | `In_Which_Layer`               | —                                      | Returns render Layer enum (Drive = Ground=2; Fly = Air; etc.) |
| 30   | +0x78  | base          | `Force_Immediate_Destination`  | `0x0055AC00` (no-op stub)              | Snap to dest immediately (unused in Drive) |
| 31   | +0x7C  | 0x004AFB40    | `Force_New_Slope`              | —                                      | Set slope index, init turn timer |
| **32**|**+0x80**|**0x004AFC20**|**`Is_Moving_Now`**           | —                                      | **True iff actively turning OR has speed AND has a waypoint.** Distinct from `Is_Moving` (+0x10) — `Is_Moving_Now` returns `false` for a unit that is *about* to move but hasn't started. Used by cloak-break detection in `FootClass::AI`. |
| 33   | +0x84  | base          | `Apparent_Speed`               | `0x0055AD10`                           | Delegates to techno `GetCurrentSpeed` (vtable+0x538) |
| 34   | +0x88  | base          | `Drawing_Code`                 | `0x0055ACF0` (returns 0)               | Render-path switch hint |
| 35   | +0x8C  | base          | `Can_Fire`                     | `0x0055AD00` (returns 0)               | Locomotor-imposed fire restriction |
| 36   | +0x90  | 0x004B4C60    | `Get_Status`                   | —                                      | Returns 0 in Drive; used by aircraft for takeoff/landing state |
| 37   | +0x94  | 0x004B4C70    | `Acquire_Hunter_Seeker_Target` | —                                      | Hunter Seeker AI hook — no-op in Drive |
| 38   | +0x98  | 0x004B4C80    | `Is_Surfacing`                 | —                                      | Submarine/Tunnel state — returns false in Drive |
| 39   | +0x9C  | 0x004B48D0    | `Mark_All_Occupation_Bits`     | —                                      | `Apply_Track_Delta` on `head_to` coord |

---

## 4. Per-locomotor override density (summary)

Rough count of how many slots each locomotor overrides (doesn't inherit from base).
Useful for reading order when implementing per-locomotor: start with the class that
overrides the most, since it has the most to decode. Drive and Ship are the richest.

| Class    | Slots overridden (approx) | Notes |
|----------|---------------------------|-------|
| Drive    | ~22 / 40                  | Richest implementation — DriveTrack state machine, slope tilt, path consumption. Reference impl for this doc. |
| Ship     | ~22 / 40                  | ~95% identical to Drive. See `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md`. |
| Walk     | ~18 / 40                  | Infantry — angle-based stepping, no DriveTrack. See `FOOTCLASS_PATHFINDING_AND_MOVEMENT.md`. |
| Jumpjet  | ~18 / 40                  | Vertical flight + hover + landing; references Rules+0x40C. |
| Fly      | ~17 / 40                  | Moves in FACING direction, not toward destination. See `FLY_LOCOMOTION_CLASS_GHIDRA_REPORT.md`. |
| Hover    | ~15 / 40                  | Robot Tank etc. See `HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md`. |
| Teleport | ~14 / 40                  | Chrono warp state machine. See `TELEPORT_LOCOMOTION_DEEP_DIVE.md`. |
| Rocket   | ~12 / 40                  | Ballistic with boost/coast/descent. See `ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`. |
| Mech     | ~8 / 40                   | 3 units, unused in stock YR. |
| DropPod  | ~6 / 40                   | 0 units, unused in stock YR. |
| Tunnel   | ~4 / 40                   | 0 units, unused in stock YR. |

For exact slot-by-slot override lists per class, decode each class's vtable starting at
the addresses in §2 and compare against the base-fallback column in §3.

---

## 5. FootClass → ILocomotion call sites

Every place `FootClass` (or its subclasses) dispatches through the ILocomotion vtable
at `FootClass+0x674`. Addresses are the FootClass call site; slot is the ILocomotion
vtable offset being called.

| FootClass method                              | Call-site addr | ILocomotion slot             | Context / state-machine role |
|-----------------------------------------------|----------------|------------------------------|------------------------------|
| `FootClass::AI`                               | `0x004DA530` (Process dispatch at `0x004DA877`) | **slot 16 Process (+0x40)**  | **Per-tick drive.** Called unconditionally every tick when the locomotor pointer is non-null. Also queries IPiggyback here for chrono-swap end-of-warp detection. (corrected 2026-07-12: was `0x004D16??` — an unverified placeholder that doesn't even match the real prefix; `search_functions_enhanced` shows `FootClass__AI @ 0x004DA530`, and `disassemble_function 0x004DA530` shows the Process call `MOV EAX,[ESI+0x674]; MOV ECX,[EAX]; CALL [ECX+0x40]` at instruction `0x004DA877` — GHIDRA_ADDRESS_SHIFT) |
| `FootClass::AI` (cloak check)                 | `0x004DA530`   | slot 32 `Is_Moving_Now` (+0x80) | Used to decide whether stealth can reapply — moving units can't cloak. (corrected 2026-07-12: was `0x004D16??`; same function as above, `FootClass__AI @ 0x004DA530` via `search_functions_enhanced` — the function dispatches `vtable+0x80` at four separate sites (`0x004DA692`, `0x004DA8BB`, `0x004DA96D`, `0x004DA924`); which one gates cloak-reapply specifically is UNVERIFIED this session — GHIDRA_ADDRESS_SHIFT for the address, mechanism site not re-verified) |
| `FootClass::Set_Destination_Internal` (`0x004D94B0`) | clear branch | slot 18 `Stop_Moving` (+0x48) | When `param_2 == 0` (destination being cleared) and not in a deploy state, stop locomotor. Asserts locomotor non-null with `0x80004003`. |
| `FootClass::Set_Destination_Internal` (`0x004D94B0`) | set branch   | slot 17 `Move_To` (+0x44)     | When `param_2 != 0` and `SuppressHeadToCoord` (byte `+0x6AC`) is clear: fetches target coord via `TargetClass::GetDockCoord` (target vtable+0x4C), then calls `Move_To` on the locomotor. If `+0x6AC` is set, clears the flag and skips `Move_To` (one-shot suppression used by chrono-warp IPiggyback restoration). |
| `FootClass::Set_Destination_Internal`         | preamble       | `LocomotionClass__QueryInterface_IPiggyback` at `0x0045AEA0` (helper, not a vtable call) | Checks whether the current locomotor supports IPiggyback (used to route warp-compatible ops). |
| `FootClass::Mission_Move`                     | `0x004D4200` (dispatch at `0x004D422A`) | slot 4 `Is_Moving` (+0x10)    | Poll each tick: when NavCom (`+0x5A4`) is clear, check `Is_Moving` — if false, unit has arrived, transition to OnArrival. Verified: `FootClass__Mission_Move @ 0x004D4200` reads the locomotor at `+0x674` and executes `MOV ECX,[EAX]; CALL [ECX+0x10]` at `0x004D422A` — a direct slot-4 dispatch. (corrected 2026-07-12: was `0x004B6610` — that address is NOT inside Mission_Move at all; it's a separate function, the base `LocomotionClass`'s default filler for slot 32 (`Is_Moving_Now`, `+0x80`), installed in the Base/DropPod/Teleport vtables (verified via `get_xrefs_to 0x004B6610`: 3 DATA refs at `0x007EAE74`/`0x007E82F8`/`0x007F5080`, zero CALL xrefs). It genuinely dispatches `+0x10` internally (`decompile_function 0x004B6610`), so Ghidra's "Is_Moving_Now_Thunk" label is accurate for *its own* vtable role — but Mission_Move never calls it; the doc conflated the mislabeled-thunk discussion with the real call site — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| `FootClass::Mission_Move` (NavCom set branch) | per doc        | slot 17 `Move_To` (+0x44)     | When mission transitions to State 1 of the Move handler, pulls dock coord from NavCom and issues `Move_To`. |
| `FootClass::GetDestinationCoords` (slot 19, `0x004DBDF0`) | — | slot 5 `Destination` (+0x14) | Returns the locomotor's stored destination. Tube-index gate happens first. |
| `FootClass::LocomotorPassabilityCheck` (slot 107, `0x004D9C10`) | — | slot 7 `Can_Enter_Cell` (+0x1C) | Pre-movement passability test. In Drive, base slot 7 is a stub; the real check is in `Can_Enter_Cell_General` at `0x00481A00`, dispatched through techno vtable +0x1AC. |
| `FootClass::AI` (`0x004DA530`)    | —              | IPiggyback slot 5 `Is_Ok_To_End` (+0x14) then slot 4 `End_Piggyback` (+0x10) | Post-Process check — if the active locomotor is piggybacked and reports it's done, QI for IPiggyback, call `Is_Ok_To_End`; if true, Release the ILocomotion ptr, clear `+0x674`, then call `End_Piggyback`. This is where chrono warps restore Drive. (corrected 2026-05-28: was attributed to `FootClass::Locomotion_AI` (`0x00520F40`); binary decompile of `FootClass::AI` at `0x004DA530` shows the QI→`+0x14`→`+0x10` call sequence; `Locomotion_AI` at `0x00520F40` does NOT call IPiggyback `Is_Ok_To_End` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| `UnitClass::PerCellProcess` (`0x0073A456`)    | —              | IPiggyback QI probe           | Transport enter sequence checks piggyback state before proceeding: `MOV EAX,[EBP+0x674]; PUSH IID_IPiggyback(0x818858); CALL [ECX]` (QueryInterface, slot 0). (corrected 2026-07-12: was attributed to "UnitClass::Mission_Enter" — no such function exists in the binary (only `FootClass__Mission_Enter @ 0x004D9290` and `AircraftClass__Mission_Enter @ 0x00419C80`, verified via `search_functions_enhanced` name_pattern "Mission_Enter"); `get_function_by_address 0x0073A456` shows this address is inside `UnitClass__PerCellProcess` (body `0x00739EC0`-`0x0073B0AE`). The QI-probe mechanism claim itself is correct, only the function attribution was wrong — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| `UnitClass::Mission_Harvest` (`0x0073E7B7`)   | —              | IPiggyback QI probe           | Chrono miner harvest/teleport sequencing. |
| `TechnoClass::Set_Destination` (`0x00741970`) | `0x00742815`   | IPiggyback QI probe           | Destination-setting dispatches through piggyback — e.g. setting a destination on a teleported unit pushes the dest into the stored Drive, not the active Teleport. |
| `FootClass::RadioReceive` / `RADIO_QUERY_DEST` | per doc       | slot 4 `Is_Moving` (+0x10)    | When queried for destination over the radio contact, returns `-10` if NavCom is set and `Is_Moving` is true. |

### 5.1 The path a `Set_Destination` takes

```
Player/AI issues order
    │
    ▼
TechnoClass::Set_Destination (0x00741970)
    │  query IPiggyback (0x00742815) — routes to stored locomotor if piggybacked
    ▼
FootClass::Set_Destination_Internal (0x004D94B0)
    │  guards: +0x6AD deploy flag, +0x82 is_crashing, +0x2E4, +0x2AC (ChronoWarp deploy target)
    │  store NavCom target at +0x5A4 (= param_1[0x169])
    │  IF param_2 == 0 (clear):   locomotor->Stop_Moving() (slot 18, +0x48)
    │  ELSE:
    │     IF +0x6AC (SuppressHeadToCoord) set:
    │        clear +0x6AC (consume one-shot), skip locomotor call
    │     ELSE:
    │        coord = target->GetDockCoord(this)       (target vtable +0x4C)
    │        locomotor->Move_To(coord)                (slot 17, +0x44)
    │
    ▼  reset timers: +0x19A (PathRetry), +0x190 (PathDelay)
(next tick)
    ▼
FootClass::AI → locomotor->Process (slot 16, +0x40)
    │
    ▼  Drive/Ship: Process_Movement → A* → path queue → Process_Drive_Track → step
    ▼  Walk:      angle step toward head_to, advance path queue
    ▼  Teleport:  state machine (arm → phase → teleport → validate)
    ▼  Fly:       facing-direction travel toward flight level
    ▼  ...
```
(corrected 2026-07-12: the two guard offsets were "+0xB9, +0xAB" — raw `int*` array indices from the decompile copied without converting to byte offsets, a PARAM1_TYPE_MISREAD; `param_1` is typed `int *` in `decompile_function 0x004D94B0`, so `param_1[0xb9]` = byte `+0x2E4` and `param_1[0xab]` = byte `+0x2AC`, consistent with the `+0x6AD`/`+0x82` byte-offset convention used by the rest of this diagram — ROOT_CAUSE: PARAM1_TYPE_MISREAD)

### 5.2 The path a per-tick `Process` takes

```
FootClass::AI (per-tick, once per mobile unit)
    │
    ├── IF locomotor has speed or waypoint:
    │      +0x538 MovementTickCounter++
    │
    ├── locomotor->Process()                     (slot 16, +0x40)
    │      → drives the class-specific state machine
    │      → may mutate linked techno position (+0x9C), facing (+0x5C), cloak (+0x74)
    │
    ├── QI active locomotor for IPiggyback (slot 0 of ILocomotion vtable)
    │      IF QI succeeds (piggybacked):
    │         IF piggyback->Is_Ok_To_End() (IPiggyback slot 5, +0x14):
    │            Release(ILocomotion ptr)   (ILocomotion slot 2, +0x08)
    │            this+0x674 = 0             (clear locomotor slot)
    │            piggyback->End_Piggyback() (IPiggyback slot 4, +0x10)
    │            → stashed original locomotor is returned by End_Piggyback;
    │              caller must store it back to +0x674 via the return value
    │   (corrected 2026-05-28: prior text implied End_Piggyback was called with &this+0x674
    │    as out-arg and directly restored it; binary at `FootClass::AI` 0x004DA530 shows the
    │    sequence above — Release → zero +0x674 → End_Piggyback. Return-value wiring to
    │    +0x674 is compiler-inlined and not visible in the decompile fragment.
    │    ROOT_CAUSE: RTTI_LABEL_DRIFT)
    │
    ├── cloak_decay -= 1 IF NOT locomotor->Is_Moving_Now()  (slot 32, +0x80)
    │
    └── Mission dispatch (may call Set_Destination_Internal → slot 17/18 again)
```

---

## 6. IPiggyback interface — 8-slot vtable

IPiggyback enables one locomotor to temporarily shadow another. The canonical use is
Chrono Legionnaire warping: a `TeleportLocomotionClass` is installed into
`FootClass+0x674`, with the unit's original `DriveLocomotionClass` stashed inside the
TeleportLocomotion via `Begin_Piggyback`. When the warp finishes, `Is_Ok_To_End`
returns true, `End_Piggyback` extracts the Drive pointer, and it's restored to `+0x674`.

| Slot | Offset | Drive addr (`0x007E7E8C`) | Method              | Purpose |
|------|--------|---------------------------|---------------------|---------|
| 0    | +0x00  | 0x004B4DC0                | QueryInterface      | Thunk → adds IPiggyback GUID to the sub-vtable's QI |
| 1    | +0x04  | 0x004B4DD0                | AddRef              | Thunk → `LocomotionClass__AddRef` |
| 2    | +0x08  | 0x004B4DE0                | Release             | Thunk → `LocomotionClass__Release` |
| 3    | +0x0C  | 0x004AF8E0                | **Begin_Piggyback** | Stash passed locomotor at this+0x68 (Drive) / +0x48 (Teleport) / +0x2C (DropPod), AddRef it. Returns `E_POINTER` if arg null, `E_FAIL` if already piggybacked. |
| 4    | +0x10  | 0x004AF930                | **End_Piggyback**   | Hand back stashed locomotor, clear stash slot. Returns `S_FALSE` if nothing stashed. Caller owns a ref. |
| 5    | +0x14  | 0x004AF970                | **Is_Ok_To_End**    | Drive: `Is_Moving()==false AND *(int*)(object_base+0x68)!=0 AND flag+0x4D!=0 AND deploy_state(FootClass+0x6AD)==0`. Condition 2 checks the **stash pointer** (`object_base+0x68`, written by `Begin_Piggyback`) — not speed. A unit decelerated to zero mid-warp still has a non-null stash and will correctly restore its original locomotor. `speed!=0` was WRONG here; that condition would cause `Is_Ok_To_End` to return false for any chrono-warped unit that decelerated to zero, breaking locomotor restoration. (verified via `decompile_function 0x004AF970`, 2026-05-20: `*(int*)(in_stack_00000004+0x50)!=0` where IPiggyback `this=object_base+0x18` → reads `object_base+0x68`) |
| 6    | +0x18  | 0x004AF610                | Piggybacker_CLSID   | Return the CLSID of the top-most locomotor (via IPersist QI). |
| 7    | +0x1C  | 0x004B4CD0                | Is_Piggybacking     | `this+0x68 != 0` (Drive) — is there a stash? |

Where the stash pointer lives varies by class: Drive/Ship use `+0x68`, Teleport uses
`+0x48`, DropPod uses `+0x2C`. See each class's field layout in its respective
LOCOMOTION doc.

---

## 7. The base `LocomotionClass` vtable (`0x007EADF4`)

This is a real vtable emitted for the abstract base class. It's written by
`LocomotionClass::Constructor` (`0x0055A6C0`) at the start of construction, then
overwritten by the derived constructor. It also appears transiently during virtual
destruction. Derived classes that don't override a slot share the entries at the
addresses listed in the "Base fallback" column of §3.

Verified xrefs confirming this identification:
- From `0x0055A6DB` in `LocomotionClass__Constructor` [DATA]
- From `0x0055A6F6` in `LocomotionClass__Destructor` [DATA]
- From 4 other destructor/teardown sites (Hover `0x005170D4`, Rocket `0x0066342E`,
  Teleport `0x00719CC2`, Tunnel `0x0072A170`)

Implication: **you will not find an object in a running game whose ILocomotion vtable
is `0x007EADF4`**. If you ever see this at runtime it's a teardown-race bug.

---

## 8. Corrections surfaced by this spec

### 8.1 `vtable+0x10` is `Is_Moving`, not `Is_Moving_Now`

Several FOOTCLASS docs call `vtable+0x10` "Is_Moving_Now". That's **wrong**.
`vtable+0x10` is slot 4 = `Is_Moving` (destination ≠ NullCoord). `Is_Moving_Now` is
slot 32 = `vtable+0x80`. Verified by reading all 160 bytes of Drive's ILocomotion
vtable (`read_memory 0x007E7EB0`, length 160): slot 4 (`+0x10`) = `0x004AFB80`,
slot 32 (`+0x80`) = `0x004AFC20` — two distinct functions.

The distinction matters:
- `Is_Moving` — "do I have a destination set?" (cheap; true throughout a move, including
  the frame the order is issued, before motion begins)
- `Is_Moving_Now` — "am I actually in motion right now?" (checks turning + speed +
  waypoint; false during the setup frame, true during active locomotion, false again
  during the stop decelerate)

`FootClass::Mission_Move` (`0x004D4200`) polls `Is_Moving` directly — verified via
`decompile_function`/`disassemble_function 0x004D4200`: it reads the locomotor at
`+0x674` and issues `CALL dword ptr [ECX+0x10]` at instruction `0x004D422A`. The cloak
decay path in `FootClass::AI` (`0x004DA530`) polls `Is_Moving_Now` (`+0x80`, correct for
"is the unit actually moving right now").
(corrected 2026-07-12: this section previously attributed the "Is_Moving_Now" mislabel to
"the Ghidra auto-label on the thunk at `0x004B6610`" and implied that address was inside
`FootClass::Mission_Move`. Verified wrong: `search_functions_enhanced` shows
`FootClass__Mission_Move` is actually at `0x004D4200`, not `0x004B6610`, and
`get_xrefs_to 0x004B6610` returns zero CALL xrefs — Mission_Move never calls that address.
`0x004B6610` is a *different* function: the base `LocomotionClass`'s default slot-32
(`Is_Moving_Now`) filler, installed in the Base/DropPod/Teleport vtables (3 DATA xrefs at
`0x007EAE74`/`0x007E82F8`/`0x007F5080`). It genuinely dispatches `+0x10` internally
(`decompile_function 0x004B6610`), so Ghidra's "Is_Moving_Now_Thunk" label is actually
*accurate* for that function's own vtable role — it just isn't the FootClass::Mission_Move
call site the original text implied. Whether `FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md`
lines 23/63/312/395 repeat this same address conflation is UNCHECKED — that doc is out of
this doc's ownership scope. ROOT_CAUSE: RTTI_LABEL_DRIFT)

### 8.2 Walk implements IPiggyback

`DRIVE_LOCOMOTION_CLASS.md` §"Which Locomotor Classes Implement IPiggyback" lists Walk
as "No". The Walk constructor at `0x0075AA90` writes a full IPiggyback vtable pointer
at object `+0x18` (address `0x007F69D4`), and the 8-slot vtable there dispatches into
Walk-specific Begin/End/Is_Ok_To_End code. Walk **is** a piggyback-capable locomotor.

This matters for Chrono Legionnaire + infantry scenarios — warping a GI briefly swaps
`FootClass+0x674` from WalkLocomotion to TeleportLocomotion, with Walk stashed via
Begin_Piggyback.

### 8.3 Base `Link_To_Object` has 9 xrefs, matching 8 derived + 1 base

Earlier investigations counted locomotor vtables by hand. Cross-referencing
`LocomotionClass__Link_To_Object` at `0x0055A710` yields 9 DATA refs — 8 derived
classes using the base method (Drive, DropPod, Mech, Rocket, Ship, Teleport, Tunnel,
Walk) plus the base class's own vtable at `0x007EADF4`. Fly/Hover/Jumpjet override
with class-specific link methods (located at each class's own address range).

---

## 9. Cross-references

- `DRIVE_LOCOMOTION_CLASS.md` — richest per-class impl; Process, state machine,
  field layout, constants, DriveTrack stepping. **Reference implementation.**
- `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` — Ship differences (6 concrete divergences)
- `DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md` — Process tick internals
- `DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md` — helper functions
- `FLY_LOCOMOTION_CLASS_GHIDRA_REPORT.md` — aircraft specifics (facing-direction flight)
- `HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md` — hover units
- `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md` — Rocketeer / takeoff/landing state
- `ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md` — ballistic rockets
- `TELEPORT_LOCOMOTION_DEEP_DIVE.md` + `TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md`
  — chrono warp state machine + piggyback restoration
- `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` — Chrono Miner harvest-teleport specifics
- `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md` — miner specifics
- `LOCOMOTION_MATH_AND_CONSTANTS.md` — shared math constants across locomotors
- `FOOTCLASS_COMPLETE_GHIDRA_REPORT.md` — FootClass overview, AI, Set_Destination
- `FOOTCLASS_PATHFINDING_AND_MOVEMENT.md` — path consumption state machine
- `FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md` — Mission_Move poll logic
  (see §8.1 correction re `Is_Moving` vs `Is_Moving_Now`)
- `FOOTCLASS_STRUCT_LAYOUT.md` — FootClass fields touched by locomotors

---

## 10. How to read a locomotor's vtable from Ghidra in 60 seconds

1. **Find the class constructor** — `search_functions` for `<Class>LocomotionClass__Constructor`.
2. **`disassemble_function`** on the constructor address. Look for the three
   `MOV dword ptr [ESI+N], 0x00XXXXXX` lines:
   - `[ESI]` = IUnknown vtable
   - `[ESI+0x4]` = **ILocomotion vtable** (this is the one you want)
   - `[ESI+0x18]` = IPiggyback vtable (if present)
3. **`read_memory`** 160 bytes (0xA0) starting at the ILocomotion vtable address.
4. **Decode little-endian 4-byte pointers** — each is one slot. Slot N is at offset 4*N.
   Compare to §3 to identify which are overridden and which inherit from the base at
   `0x007EADF4`.

All 11 locomotor vtable addresses are tabulated in §2; you should only need to do this
from scratch if a mod adds a new locomotor class.

## Tier 3 application record (2026-08-17, Claude Code session)

Applied hot-path slot prototypes for the Drive and Walk families (the ordinary-ground-
skirmish pair; Ship ≈ Drive). Slot targets re-read live from both vtables this session
and matched this spec and the LANE1 table exactly. ABI proven from RET immediates on the
Drive bodies (Is_Moving/Process/Stop_Moving/Is_Moving_Now: RET 4 = iface only;
Move_To: RET 0x10 = iface + x,y,z — confirmed by decompile storing +0x30/34/38 with
NullCoord check and bridge-Z adjust gated on cell Flags & 0x100) and bound to Walk by
same-slot substitutability.

Applied __stdcall prototypes (10):
- Drive 0x004AFB80 Is_Moving: bool(void* iface); 0x004B0500 Process: bool(void* iface) — RESOLVED by the 2026-08-17 critic pass:
  body deliberately materializes AL (XOR AL,AL paths; one tail-call into slot +0x10),
  and EAX is unconsumed at all five live +0x40 dispatch sites (FootClass__AI 004da877,
  UnitClass__AI 007362ea, InfantryClass__AI 0051bbc0, AircraftClass__AI 00414cbb,
  InfantryClass__Scatter 0051d478). Applied bool per the AL materialization; the two
  other [reg+0x40] calls near 0x00773803/0x00773b2d are a different interface
  (persist-stream), not ILocomotion;
  0x004AFD40 Move_To: void(void* iface, int x,y,z); 0x004AFE00 Stop_Moving: void(void*);
  0x004AFC20 Is_Moving_Now: bool(void* iface) — CORRECTED from a wrong pre-existing
  __thiscall typing (COM slot dispatch passes iface on the stack, not ECX).
- Walk 0x0075AB30 / 0x0075AC80 / 0x0075ACB0 / 0x0075ADA0 / 0x0075AB40: same shapes.

Residuals / flags:
- WRONG LABEL (unapplied, needs authorization): 0x0075ACB0 is Walk's slot-17 Move_To
  but is labeled WalkLocomotionClass__Head_To_Coord — Head_To_Coord is slot 6, whose
  Walk target reads 0x0075AC00 (verified live). Same error class as the corrected
  FOOTCLASS_MISSION_MOVE +0x10 mislabel this spec already documents.
- Receivers remain void* iface — interface-this points at object+0x04, so proper
  typing needs per-family interface-view structs (object layout shifted by 4); on-
  contact work, not bulk.
- Remaining 9 families' hot-path slots and all other 35 slots: type on contact.
