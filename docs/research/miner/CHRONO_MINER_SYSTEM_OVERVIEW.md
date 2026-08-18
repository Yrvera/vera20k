# Chrono Miner System Overview

How the chrono miner teleport system works end-to-end in gamemd.exe.
This document ties together findings from the detailed Ghidra reports into
a single implementation reference. For raw decompiled code and byte offsets,
see the source reports listed at the bottom.

---

## 1. Class Hierarchy and Responsibilities

```
AbstractClass
  └─ ObjectClass
       └─ TechnoClass        ← chrono state fields, Set_Destination (teleport-vs-drive),
       │                        draw with warp flags, visual translucency
       │
       ├─ FootClass           ← owns ILocomotion*, runs IPiggyback swap in AI(),
       │                        Assign_Destination dispatches to active locomotor
       │
       └─ UnitClass           ← Mission_Harvest (5-state ore cycle),
                                Mission_Guard_Harvester, Deploy
```

Every chrono miner is a **UnitClass** instance. It inherits locomotor management from
FootClass and chrono state fields from TechnoClass.

---

## 2. The Locomotor Layer

The chrono miner's **primary** locomotor is always `TeleportLocomotionClass`. Inside it,
a `DriveLocomotionClass` is stored via the **IPiggyback** COM interface. The game swaps
which one is "active" depending on whether the miner should warp or drive.

```
FootClass instance (the chrono miner)
  │
  └─ Locomotor* (+0x674)          points to the ACTIVE locomotor
       │
       ╔═══════════════════════════════════════════════════╗
       ║  TeleportLocomotionClass  (primary, 0x4C bytes)   ║
       ║    +0x04: ILocomotion vtable (0x7F5000)           ║
       ║    +0x18: IPiggyback vtable  (0x7F4FDC)           ║
       ║    +0x1C: HeadToCoord (destination XYZ)           ║
       ║    +0x28: DestCoord (validated destination)       ║
       ║    +0x34: IsMoving (byte, triggers warp)          ║
       ║    +0x38: WarpPhase (0-7 state machine)           ║
       ║    +0x3C: Timer.StartFrame                        ║
       ║    +0x44: Timer.Duration                          ║
       ║    +0x48: PiggybackedLoco* ───────────────┐       ║
       ╚═══════════════════════════════════════════╪═══════╝
                                                   │
                                                   ▼
                                     ┌──────────────────────────┐
                                     │  DriveLocomotionClass    │
                                     │  (piggybacked inside)    │
                                     │  Handles ground driving, │
                                     │  pathfinding, tracks     │
                                     └──────────────────────────┘
```

### IPiggyback Interface (TeleportLocomotionClass, vtable 0x7F4FDC)

| Method | Address | What it does |
|--------|---------|-------------|
| Begin_Piggyback | 0x719E90 | Store a locomotor at +0x48. Fails if already piggybacking. |
| End_Piggyback | 0x719EE0 | Return the stored locomotor, clear ChronoSource fields on TechnoClass. |
| Is_Ok_To_End | 0x719F30 | True only when: not moving, has piggybacked loco, field_35==0, ChronoInTransit==0, WarpPhase==0, field_6AD==0. |
| Is_Piggybacking | 0x71A100 | Returns (+0x48 != NULL). |

### Locomotor Swap Lifecycle

```
                    ┌──────────────────────────────────┐
                    │       FootClass::AI (0x4DA530)   │
                    │       runs EVERY tick             │
                    │                                  │
                    │  1. QueryInterface for IPiggyback │
                    │  2. Call Is_Ok_To_End()           │
                    │  3. If true:                      │
                    │     End_Piggyback() → get old loco│
                    │     Swap active locomotor back    │
                    └──────────────────────────────────┘

    DRIVING state                              WARPING state
    Active = Drive                             Active = Teleport
    Teleport stored under Drive                Drive stored under Teleport

    Drive finishes → Is_Ok_To_End true         Warp completes → Is_Ok_To_End true
    Swap back → Active = Teleport              Swap back → Active = Drive
    (idle, ready for next warp)                (idle, ready to drive to dock)
```

---

## 3. The Teleport-vs-Drive Decision

**(corrected 2026-07-19 — see `CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md` and
`CHRONO_MINER_WARP_TRIGGER_GHIDRA_REPORT.md`, both fresh this session; supersedes the
flowchart and "STRUCTURAL CAVEAT" previously here.)** The prior framing — "empty cell →
FindFirstUnit/FindFirstBuilding NULL → stays Teleport → warps" — is INCOMPLETE/MISLEADING
and does NOT explain the classic ore-to-refinery warp. Two separate gates are involved, and
the classic long-range harvest-return call structurally fails the first one.

### 3.1 Gate 1 — `TechnoClass::Set_Destination`'s Teleporter block (0x741970, block at 0x7423CD)

Decides whether the ACTIVE locomotor changes when a new destination is assigned.
Asm-verified (`disassemble_function 0x00741970`,
`CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md` §1): a flag byte defaults to 1
("prefer Drive" — create/keep a Drive-locomotor piggyback) and is overwritten to 0
("prefer/keep Teleport") only when ALL of:
  - the OLD destination (NavCom, `FootClass::GetDestination(this,0)` @ 0x65AD30) has
    RTTI==BuildingClass(6), AND
  - that building's `BuildingTypeClass+0x16B3` (`DockUnload=`) is set, AND
  - the NEW destination is a valid `CellClass` (RTTI==0xB), AND
  - `CellClass::FindFirstUnit` (0x47EBA0) returns NULL on that cell — no `UnitClass`
    occupant (RTTI==1). **Re-verified this session via `decompile_function 0x0047eba0`:
    the function iterates the cell's occupant list at `cell+0xE4`/`+0xE8` and returns the
    first occupant whose `What_Am_I()==1`; it never inspects buildings.** The doc's older
    "FindFirstBuilding" identity for this address remains wrong (RTTI_LABEL_DRIFT, first
    corrected 2026-07-12, reconfirmed 2026-07-19).

If the flag stays 1, Drive becomes/stays active and the unit drives. If it becomes 0,
Teleport stays/becomes the ACTIVE locomotor — but that alone does not fire a warp; see 3.2.

**Mission_Harvest state 2's classic fallback call (the "far from refinery, go home"
branch) ALWAYS presents NavCom==NULL.** Its entire dock-search body is gated by
`if (param_1[0x169] != 0) goto default;` (`param_1[0x169]` = NavCom, +0x5A4), verified via
`decompile_function 0x0073E5E0`
(`CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md` §2). A NULL old destination skips the
RTTI/DockUnload/FindFirstUnit chain entirely (`TEST EDI,EDI; JZ 0x7425e6` at
0x742465-0x74246c) and takes the flag=1 default. **So the classic long-range
harvest-return call structurally CANNOT pass Gate 1 — "empty cell alone → warps" is wrong
for this call.**

> **Correction (2026-07-19, `CHRONO_MINER_WARP_ARMING_CALLER_GHIDRA_REPORT.md`):** the
> field this paragraph names is WRONG, though the conclusion holds. Gate 1's "old
> destination" test is `TEST EDI,EDI` where `EDI = Contact_With_Whom(0) = Contacts[0]`
> (`+0xE4[0]`, the primary **radio contact**) — the arming block calls `0x0065AD30`
> (`PUSH 0; CALL 0x0065ad30` at `0x0074240F`), which reads `+0xE4`, and **never reads
> `+0x5A4`**. `+0x5A4` (`param_1[0x169]`) is a *separate* committed-destination field; it
> gates the Mission_Harvest *fallback dock-search*, not Gate 1 (the two are provably
> distinct — `Receive_Radio` case 0x17 compares them). The far/long-range path fails Gate 1
> because **`Contacts[0]` is NULL** (no HELLO/ROGER link formed yet), not because `+0x5A4`
> is NULL. (The function at `0x0065AD30`, formerly mislabeled `FootClass__GetDestination`,
> is renamed `RadioClass__Contact_With_Whom` in Ghidra.)

### 3.2 Gate 2 — the warp itself only fires from Teleport's own HeadToCoord, inside StateMachineTick

Even when Gate 1 does yield flag==0, the warp does not fire synchronously inside
`Set_Destination`. `FootClass::Set_Destination_Internal` (0x4D94B0) dispatches
`Head_To_Coord` (ILocomotion vtable +0x44) to whichever locomotor is CURRENTLY active at
that moment only — verified via `decompile_function 0x004D94B0`
(`CHRONO_MINER_WARP_TRIGGER_GHIDRA_REPORT.md` §1.4). If that's Teleport, `HeadToCoord`
(0x718100) sets Teleport's own `Is_Moving` flag (+0x30). The warp then arms on a LATER
tick inside `TeleportLocomotionClass::StateMachineTick` (0x7192F0) — **re-verified this
session via `get_function_by_address 0x007192f0`: entry 0x7192f0, body
0x7192f0-0x719bed** — the real per-tick `ILocomotion::Process` slot for Teleport
(vtable+0x40, matching `DriveLocomotionClass::Process`'s slot on Drive's vtable; report
§1.2). Inside StateMachineTick the warp-initiation sequence (WarpOut anim, ChronoDelay
math, BeingWarped=1, etc. — see §5) fires only when
`ChronoInTransit==0 && WarpPhase==0 && Is_Moving()==true` (report §1.3). Two
Ghidra-labeled functions that look like plausible warp entry points are label errors, not
real callable functions: the address once labeled `TeleportLocomotionClass__InitiateWarp`
(0x719400 — **re-verified this session: `get_function_by_address` reports body
0x719400-0x71978f, fully inside StateMachineTick's own 0x7192f0-0x719bed range, and
`get_xrefs_to 0x00719400` returns zero references**) and `TeleportLocomotionClass__Process`
(0x718B70, only called synchronously from HeadToCoord, absent from the ILocomotion
vtable) — report §1.1/§5.

### RESOLVED (2026-07-19) — which caller arms Teleport

Full closure in `CHRONO_MINER_WARP_ARMING_CALLER_GHIDRA_REPORT.md` (4-lane trace swarm +
adversarial verify, CONFIRMED). The question conflated two roles:

- **Contact SOURCE** (puts the refinery into `Contacts[0]`): the miner's own **HELLO
  (radio `0x02`)** in `Mission_Harvest` state 2 (`0x0073E5E0`, `LAB_0073ee51`). On ROGER,
  `Transmit_Radio_Impl` (`0x0065A970`) writes `refinery → miner.Contacts[0]`.
- **ARMING CALL** (flips Drive→Teleport): **`FootClass::Receive_Radio` case `0x12`
  (MOVE_TO_CELL), call site `0x004D91EB`** = `Set_Destination(dockCell, 1)`. Driven by the
  refinery's CAN_DOCK reply: `Mission_Enter` sends CAN_DOCK(`0x0E`) → `BuildingClass::
  Receive_Radio` case `0x0E` (`0x0043C2D0`) computes dock cell = NW anchor +(3,1) and sends
  `0x12` back. At that instant `Contacts[0]`=refinery (a `DockUnload` building), `param_2`=
  the dock CellClass → Gate 1 keeps Teleport. (`Mission_Enter` Site C `0x004D941D` is a
  redundant re-arm; Sites A/D use `vtable+0x484`≠Set_Destination with `param_2=0`.)

**Close-vs-far settled:** the observable warp is a **FULL ore→refinery jump** in the normal
case (refinery within `ChronoHarvTooFarDistance`=50 cells — the miner links from the ore
position and warps in one hop to the dock cell). The "short final-approach hop only"
speculation was WRONG; drive-to-staging-then-final-warp is the **>50-cell fallback** only.
See the arming-caller report §4.

---

## 4. The Harvest Cycle (UnitClass::Mission_Harvest, 0x73E5E0)

A 5-state state machine at UnitClass byte offset 0xBC:

```
  State 0: SCAN
    Diamond spiral search for ore cells
    Both war miner and chrono miner use TiberiumLongScan (48 cells) in state 0.
    (Corrected 2026-05-19: prior text said chrono uses TiberiumShortScan (6) —
    that was WRONG. Verified: 0x73E851 reads RulesClass+0x177C (LongScan) for
    both. TiberiumShortScan (RulesClass+0x1778) is used only in state 1
    continuation scans. See MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN report.)
    The chrono-specific branch in state 0 is locomotor-CLSID cancel (clears an
    in-progress teleport destination before scanning), NOT a different radius.
    Found ore → Set_Destination(ore_cell)
    Ore cell is empty → but drive piggyback is created (need to pathfind to ore)
    Unit DRIVES to ore
                    │
                    ▼
  State 1: HARVEST
    Wait HarvesterLoadRate (2) ticks between bales
    CellClass::Reduce_Tiberium decrements density (0-11 at CellClass+0x11E)
    When cell depleted → short scan for more nearby
    When storage full (20 bales for CMIN) →
                    │
                    ▼
  State 2: RETURN
    Find_Docking_Bay (0x4DF040) → nearest refinery
    Distance check: dist <= ChronoHarvTooFarDistance (50) * 256 leptons?
      YES → RadioClass::Transmit_Radio(RADIO_DOCKING=2, dock) → reserve slot → state 3
      NO  → Compute dock-adjacent cell from BuildingType->DockOffset (+0x1618/+0x161C)
             Set_Destination(adjacent_cell)
             (corrected 2026-07-19: this call's OLD destination (NavCom) is ALWAYS NULL
             here — Mission_Harvest state 2's entire dock-search body is gated on
             NavCom==NULL to even reach this fallback, verified via decompile_function
             0x73E5E0. Per §3.1, a NULL old destination takes Set_Destination's flag=1
             "prefer Drive" default — it does NOT itself keep Teleport active or arm a
             warp, regardless of what FindFirstUnit finds on the adjacent cell. The
             "★ WARP triggers" claim previously here was wrong. What actually carries the
             miner home, and where/whether a warp gets armed later in the sequence, is
             OPEN — see §3 "which caller supplies the DockUnload NavCom".
             CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md §2,
             CHRONO_MINER_WARP_TRIGGER_GHIDRA_REPORT.md §2/§6.)
             → Drive becomes/stays active by default; unit drives (§3 OPEN item covers the
             unresolved short-hop-warp hypothesis for the final approach)
                    │
                    ▼
  State 3: DOCK
    Queue Mission_Enter (mission 7)
    After warp-in: FootClass::AI swaps to DriveLocomotionClass
    Unit DRIVES last few cells to dock pad
    Dock, unload (HarvesterDumpRate = 14.4 frames/bale), exit facing 0x47 (ESE)
                    │
                    ▼
  State 4: LOST
    No ore found anywhere → fall back to Guard mission
```

---

## 5. The Warp Sequence (TeleportLocomotionClass::StateMachineTick, 0x7192F0)

Called every tick via ILocomotion vtable slot 16. WarpPhase at locomotor +0x38 controls state.

### Self-Teleport (Chrono Miner / Chrono Legionnaire moving)

Everything happens in **Phase 0, one single tick**:

```
Phase 0: WARP_START
  Condition: IsMoving==1 AND current position != destination

  (corrected 2026-07-12: step order below was wrong in two places — "Detach all anim
  effects" and "Start timer" were listed at the wrong positions. Verified via
  decompile_function 0x7192F0, reading the phase-0 branch in raw code order.
  OPERATOR_OR_ORDER_DRIFT.)

  1. Stop all units targeting this one (FUN_0070D4A0)
  2. Spawn WarpOut anim (Rules+0x33C) at departure point
  3. Calculate distance: (int)sqrt(dx*dx + dy*dy + dz*dz) in leptons
  4. Calculate chrono delay AND start the locomotor timer in the same step
     (StartFrame=CurrentFrame, Duration=delay — the timer write is part of this
     calculation, not a separate later step):
       if ChronoTrigger (Rules+0xBF8):
         delay = distance / ChronoDistanceFactor (Rules+0xBF4, default 48)
       clamp to max(ChronoMinimumDelay (Rules+0xBFC, default 16), delay)
       if distance < ChronoRangeMinimum (Rules+0xC00): force minimum
  5. Set BeingWarped (+0x271) = 1 on TechnoClass
  6. Harvester instant-unwarp: if WhatAmI()==1 (UnitClass) AND TypeClass+0xE0E
     (Harvester=yes flag) → timer Duration reset to 0, BeingWarped reset to 0
     (corrected 2026-07-18: was "Infantry chrono-kill: if infantry AND owner has
     Chronosphere"; binary condition at 0x719588-0x7195A5 is
     `WhatAmI()==1 && *(char*)(*(int*)(techno+0x6c4)+0xe0e)!='\0'` — no owner or
     Chronosphere check exists anywhere in this branch. `UnitClass::What_Am_I`
     returns 1 (re-confirmed this session, matches the RTTI mapping already
     verified 2026-07-12). TechnoTypeClass+0xE0E is the `Harvester=` INI boolean
     — confirmed independently this session via `search_strings "^Harvester$"`
     (hit 0x0083d4cc) → `get_xrefs_to 0x0083d4cc` → `UnitTypeClass::ReadINI`
     write site at 0x007476b9 (`MOV byte ptr [EDI+0xe0e],AL` immediately after
     the ReadBool call for the "Harvester" key). This is the chrono
     miner/harvester's own instant-unwarp fast path (skip the translucency/lock
     period), not an infantry-specific mechanic. Verified via
     decompile_function 0x7192F0 — INFERENCE_HARDENED.)
  7. Detach all anim effects from unit (WarpAttachClass::Detach — only if the
     unit has an active attach list; this happens here, AFTER the infantry
     check, not right after step 1)
  8. Unmark from old cell
  9. Play ChronoOutSound (TypeClass+0x578, fallback Rules+0x21C)
  10. Set bridge flag from destination cell
  11. Mark at new cell — UNIT IS NOW AT DESTINATION
  12. Play ChronoInSound (TypeClass+0x574, fallback Rules+0x218)
  13. Set mission to GUARD_AREA (2)
  14. Handle crate pickup at destination
  15. Spawn WarpOut anim at arrival point
  16. Clear PendingWarpPhase (+0x280)
  17. WarpPhase stays 0 (does NOT increment)

  Subsequent ticks:
    Pre-phase check: BeingWarped==1, WarpPhase==0, PendingWarpPhase==0
    → Calls TimerCheck (0x719BF0) each tick
    → Timer counts down
    → When expired: BeingWarped = 0 (corrected 2026-07-18: "unit fully opaque" removed —
      no verified rendering effect ties to this field, see §6)
```

### Chronosphere Path (external warp, phases 0-7)

Used when the Chronosphere superweapon warps a unit. Different entry point — the
superweapon handler sets PendingWarpPhase=3 and ChronoDestCoords on TechnoClass,
then the state machine picks up from Phase 2:

```
Phase 0: ChronoInTransit detected → set WarpingOut=1, timer=60 frames, WarpPhase→1
Phase 1: TimerCheck waits 60 frames. WarpPhase→2
         ("unit translucent at departure" removed 2026-07-18: TechnoClass::Draw
         does not read +0x271 anywhere — see §6 correction; no verified visual
         effect is tied to this phase this session)
Phase 2: Spawn anim, read ChronoDestCoords, call Update_Position. WarpPhase→3 or 4
Phase 3: Continue Update_Position. Store ChronoDelay into +0x284. WarpPhase→4
Phase 4: Final position, mark occupancy. WarpPhase→5
Phase 5: PostWarpValidation (0x7187A0): water death, occupied cell damage, bridge check.
         Spawn WarpOut anim. Start chrono lock timer. WarpPhase→6
Phase 6: TimerCheck waits for chrono lock. WarpPhase→7
Phase 7: Clear all flags (BeingWarped, IsMoving, PendingWarpPhase, WarpPhase→0)
```

---

## 6. Visual Rendering During Warp

There is **no gradual fade** for self-teleport. Two separate visual systems exist:

### Teleport Translucency — WRONG, no verified draw-flag link to BeingWarped

**(corrected 2026-07-18, was: "When `BeingWarped` (+0x271) is true, `TechnoClass::Draw`
(0x706640) adds draw flag `0x2004` — binary 50% translucency. The unit appears at the
destination semi-transparent for the chrono delay duration, then snaps to fully opaque
when the timer expires.")** Full decompile of `TechnoClass::Draw` (0x706640) this session
contains **no reference to offset +0x271 anywhere in its body**. Draw flag `0x2004` is
produced by two independent, unrelated code paths, neither of which reads BeingWarped:
(1) `TechnoClass_GetVisualState` (0x00703860, reached via `FootClass::GetVisualState`
0x004DA4E0 → vtable+0x68), whose state 1-5 values are computed from TypeClass+0xC9A /
self+0x41A / self+0x88 / self+0x89 — this is the **cloak/IronCurtain visibility gradient**,
confirmed to have zero relation to chrono warp; (2) the `vtable+0x160`-gated call to
`TechnoClass__ScaleByTemporalVisualPhase` + `TechnoClass__ScaleByWarpInVisualPhase`, which
per the sibling `CHRONO_WARP_VISUAL_RENDERING.md` 2026-07-12 correction operates on the
temporal-erasure/gap-generator fields (+0x1B4/+0x1BC/+0x1C0), not BeingWarped. This
directly refutes the CONFIRMED status this claim was given in the 2026-07-12 fix-swarm
pass on THIS doc, and re-confirms an older, previously-overridden 2026-05-06 audit
finding on `CHRONO_WARP_VISUAL_RENDERING.md` ("BeingWarped is NOT read by
TechnoClass::Draw"). `TechnoClass::IsBeingWarped` (0x70C5C0, trivial `return this+0x271`)
has **no static callers** (`get_function_callers` returns none) — consistent with
vtable-only dispatch, but its actual reader (if any) was not located this session.
UNVERIFIED-pending-reinvestigate: whether gamemd renders ANY visual translucency during
the self-teleport BeingWarped/lock window at all, and if so, which function reads +0x271
for it. ROOT_CAUSE: INFERENCE_HARDENED (asserted CONFIRMED without tracing the vtable+0x68
indirection to its actual target). Verified via decompile_function 0x706640 + 0x004DA4E0 +
0x00703860 + 0x70C5C0 + get_function_callers 0x70C5C0 + read_memory 0x7F5034 (TeleportLoco
vtable+0x34 → 0x0055ABC0 `LocomotionClass::Visual_Character`, trivial return-0, confirming
the visual-state fallthrough to TechnoClass_GetVisualState for the chrono miner's active
locomotor).

### Temporal Weapon Fade (NOT teleport — do not confuse)

The smooth fade people associate with "chrono" is the Chrono Legionnaire's **erasing beam**
effect on its TARGET, driven by `TechnoClass::UpdateTemporalVisual` (0x70E5A0) — a
10-phase (0-9, plus terminal phase 10) state machine at TechnoClass offsets
+0x198 (StartFrame)/+0x1A0 (Duration)/+0x1A4 (Phase), re-confirmed byte-exact this
session via decompile_function 0x70E5A0. "Smooth mathematical curves" is MISLEADING
(corrected 2026-07-18): the function itself is pure phase/duration bookkeeping with
**zero arithmetic/curve computation** — no formula for the visual fade lives here; this
matches the sibling `CHRONO_WARP_VISUAL_RENDERING.md` 2026-07-12 correction for the same
function ("formulas replaced with UNVERIFIED markers... zero arithmetic"). Wherever the
actual alpha/scale curve is computed (if anywhere) was not located this session. This is
set by `TemporalClass::InitiateWarp` (0x71AF20) on the VICTIM, not on the Legionnaire
itself. Completely unrelated to locomotor teleport. ROOT_CAUSE: INFERENCE_HARDENED.

### Warp Animations (see ANIM_CLASS_GHIDRA_REPORT.md for full details)

All warp visual effects come from AnimType overlays, not unit rendering changes.
Spawned via `AnimClass::Constructor(type, coords, delay=0, loopCount=1, flags=0x600, zAdj=0, reverse=0)`.
Flag `0x600` = `0x200` (center sprite on coords) | `0x400` (unused). Anims self-destruct when done.

| Rules Offset | INI Key | AnimType | Properties | Spawned When |
|-------------|---------|----------|------------|-------------|
| +0x340 | WarpAway | WARPAWAY | Flat, Translucent, Rate=300 (3 ticks/frame), ground layer | Parsed; not used by verified TeleportLocomotion rows |
| +0x338 | WarpIn | WARPIN | Flat, Translucent, Rate=120 (7 ticks/frame), YSort=-64 | Parsed; not used by verified TeleportLocomotion rows |
| +0x33C | WarpOut | WARPOUT | Flat, Translucent, Rate=120 (7 ticks/frame), YSort=-64 | Departure/arrival for verified TeleportLocomotion rows |
| +0x344 | ChronoSparkle1 | CHRONOSK | Flat, Rate=150 (6 ticks/frame), LoopCount=1, ZAdj=-124 | Parsed; not used by verified TeleportLocomotion rows |

Per-type sound overrides: TechnoTypeClass+0x574 (ChronoInSound), +0x578 (ChronoOutSound).
Global fallbacks: Rules+0x218 (ChronoInSound), Rules+0x21C (ChronoOutSound). Default: `ChronoMinerTeleport`.

**Rate conversion**: `internal_ticks = 900 / INI_Rate`. At 15fps, Rate=300 → 3 ticks/frame (fast), Rate=120 → 7 ticks/frame (slower).

**Semicolon format**: INI values like `WarpAway=WARPAWAY;RING1` specify primary + secondary anims. RING1 is an expanding/fading ring rendered with a special Z-buffered quad path in AnimClass::DrawIt.
|-------------|---------|-------------|
| +0x33C | WarpOut | At both departure AND arrival (same anim!) |
| +0x218 | ChronoInSound | Arrival (global fallback) |
| +0x21C | ChronoOutSound | Departure (global fallback) |

Per-type sound overrides: TechnoTypeClass+0x574 (ChronoInSound), +0x578 (ChronoOutSound).

---

## 7. TechnoClass Chrono Fields (Verified)

All byte offsets on TechnoClass, definitively resolved from binary:

| Offset | Type | Name | Purpose |
|--------|------|------|---------|
| +0x08C | byte | IsOnBridge | Set from destination cell flags during warp |
| +0x218 | ptr | GhostCell | CellClass* for building deploy ghost (NOT warp-related) |
| +0x270 | byte | IsWarpingOut | Set by Temporal weapon on target; set in Chronosphere Phase 0 |
| +0x271 | byte | IsBeingWarped | Set during teleport, cleared on timer expiry. NOT a confirmed draw-flag trigger (corrected 2026-07-18 — see §6; `TechnoClass::Draw` 0x706640 does not read this offset) |
| +0x27C | byte | ChronoInTransit | Set externally by Chronosphere superweapon (flag, not countdown) |
| +0x280 | int | PendingWarpPhase | Set to 3 by Chronosphere, 0 by state machine. NOT a CoordStruct. |
| +0x284 | int | ChronoLockDuration | **Chronosphere superweapon path only** — initially ChronoReinfDelay, overwritten with ChronoDelay in Phase 3 (0x719983). **Self-teleport never touches this field**; its post-warp timer lives at `TeleportLocomotionClass+0x44` (locomotor timer.Duration). See `PHASE0_CHRONO_DELAY_FORMULA_MATH_GHIDRA_REPORT.md`. |
| +0x288 | int | ChronoDestCoord.X | Warp destination (set by Chronosphere or state machine) |
| +0x28C | int | ChronoDestCoord.Y | |
| +0x290 | int | ChronoDestCoord.Z | |
| +0x428 | ptr | ChronoSourceBuilding | Building that initiated Chronosphere warp (NULL for self-teleport) |
| +0x42C | ptr | ChronoSourceHouse | House owning the Chronosphere (NULL for self-teleport) |

### TechnoTypeClass Flags

| Offset | INI Key | Type | Purpose |
|--------|---------|------|---------|
| +0x574 | ChronoInSound | int | Per-type warp-in sound (-1 = use global) |
| +0x578 | ChronoOutSound | int | Per-type warp-out sound (-1 = use global) |
| +0xCD4 | Teleporter | bool | Enables teleport movement (chrono miner, chrono legionnaire) |
| +0xCCE | Chronoshiftable | bool | Can be moved by Chronosphere superweapon |

### RulesClass Constants ([General] section)

| Offset | INI Key | Type | Default | Purpose |
|--------|---------|------|---------|---------|
| +0xBEC | ChronoDelay | int | 60 (corrected 2026-07-12, was "—"; source ini/rulesmd.ini line 221) | Post-warp chrono lock duration |
| +0xBF0 | ChronoReinfDelay | int | 180 (corrected 2026-07-12, was "—"; source ini/rulesmd.ini line 222) | Delay for Chronosphere warp |
| +0xBF4 | ChronoDistanceFactor | int | 48 | Divisor: delay = distance / factor |
| +0xBF8 | ChronoTrigger | bool | true | Enable distance-based delay calculation |
| +0xBFC | ChronoMinimumDelay | int | 16 | Floor for warp timer duration |
| +0xC00 | ChronoRangeMinimum | int | 0 (corrected 2026-07-12, was "—"; source ini/rulesmd.ini line 227) | Below this distance, force minimum delay |
| +0xD7C | ChronoHarvTooFarDistance | int | 50 | Max cells for CMIN warp-return (compared as leptons * 256); gated on TechnoTypeClass+0xCD4 Teleporter flag — verified via decompile_function 0x73E5E0 (war miner uses a separate Rules+0xD78 threshold when Teleporter==false) |

---

## 8. One Complete Harvest-Return — sequence below is CORRECTED, one step OPEN

**(corrected 2026-07-19)** The 10-step sequence formerly here assumed the classic
Mission_Harvest state-2 fallback `Set_Destination` call (dock-adjacent cell, far-refinery
branch) itself keeps Teleport active and fires an instant warp. Fresh this session,
`CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md` §2 and
`CHRONO_MINER_WARP_TRIGGER_GHIDRA_REPORT.md` §1.5 both prove that call's OLD destination
(NavCom) is ALWAYS NULL when it runs, which routes to `Set_Destination`'s flag=1 "prefer
Drive" default (§3.1) — so this specific call structurally cannot be the warp trigger.
Corrected sequence:

1. **Mission_Harvest state 1** fills storage to 20 bales
2. **State 2**: `Find_Docking_Bay` (0x4DF040) locates nearest refinery
3. **State 2**: computes **dock-adjacent cell** from `BuildingType->DockOffset`
4. Calls **TechnoClass::Set_Destination** (0x741970) with that cell; NavCom is NULL at
   this call (verified via decompile_function 0x73E5E0) → flag=1 default → Drive
   becomes/stays active, NOT Teleport
5. **FootClass::Set_Destination_Internal** (0x4D94B0) dispatches `Head_To_Coord` to the
   ACTIVE locomotor only (verified via decompile_function 0x4D94B0) — here that's Drive,
   so Drive's `Head_To_Coord` (0x4AFD40) runs, not Teleport's
6. Unit drives toward the dock-adjacent cell under normal A* pathfinding
7. **OPEN — UNVERIFIED-pending-reinvestigate:** somewhere in the accepted-dock sequence
   (`FootClass::Mission_Enter` 0x4D9290's CAN_DOCK negotiation, `FootClass::Receive_Radio`
   0x4D8FB0 case 0x12 MOVE_TO_CELL, or `TechnoClass::Set_Destination`'s separate "Dock=
   list re-target" block) some OTHER `Set_Destination` call must present NavCom as a
   `DockUnload=yes` building to satisfy Gate 1 and actually arm Teleport — see §3 "OPEN".
   The evidence gathered so far makes a SHORT final-approach hop (staging cell → accepted
   dock pad) structurally more plausible than a single long-range warp, but this is NOT
   proven; the caller that supplies the DockUnload-building NavCom was not identified.
8. **IF** Teleport is armed at that point: `TeleportLocomotionClass::StateMachineTick`
   (0x7192F0) Phase 0 fires on a later tick — WarpOut anims, chrono delay, moves unit
   instantly, `BeingWarped = 1` (see §5)
9. `BeingWarped = 1` for chrono delay frames, no confirmed visual translucency tied to it
   (corrected 2026-07-18, see §6); timer expires → `BeingWarped = 0`
10. **FootClass::AI** detects `Is_Ok_To_End = true` → swaps to **DriveLocomotionClass** →
    miner **drives** last cells into refinery dock

---

## 9. Key Function Reference

| Address | Function | Role |
|---------|----------|------|
| 0x7192F0 | TeleportLocomotionClass::StateMachineTick | Main warp state machine (every tick); confirmed this session as the real ILocomotion::Process vtable+0x40 slot for Teleport, matching DriveLocomotionClass::Process's slot on Drive's vtable — verified via get_function_by_address 0x007192f0, see §3.2 |
| 0x718100 | TeleportLocomotionClass::HeadToCoord | Sets IsMoving=1 on Teleport; warp arms on a LATER StateMachineTick, not synchronously — see §3.2 |
| 0x719400 | (label drift, not a real function) | Body 0x719400-0x71978f fully inside StateMachineTick's own range; zero xrefs (re-verified this session via get_function_by_address + get_xrefs_to). Do NOT cite as "InitiateWarp" or any independently-callable warp entry point — CHRONO_MINER_WARP_TRIGGER_GHIDRA_REPORT.md §1.1/§5 |
| 0x718B70 | TeleportLocomotionClass::Process | Validates destination, sets DestCoord — synchronous helper called only from HeadToCoord (0x7181ac); NOT the per-tick ILocomotion::Process (absent from the ILocomotion vtable) — CHRONO_MINER_WARP_TRIGGER_GHIDRA_REPORT.md §1.2/§5 |
| 0x718260 | TeleportLocomotionClass::Update_Position | Moves unit, handles bridge Z |
| 0x7187A0 | TeleportLocomotionClass::PostWarpValidation | Water death, damage, bridge check |
| 0x719BF0 | TeleportLocomotionClass::TimerCheck | Timer expiry for phases 1/6 |
| 0x719E90 | TeleportLocomotionClass::Begin_Piggyback | Store locomotor for swap |
| 0x719EE0 | TeleportLocomotionClass::End_Piggyback | Return stored locomotor |
| 0x719F30 | TeleportLocomotionClass::Is_Ok_To_End | 6 conditions for swap readiness |
| 0x741970 | TechnoClass::Set_Destination | Gate 1 of 2 (§3.1) — defaults to Drive, only yields flag=0 with a DockUnload-building old NavCom + empty unit-free destination cell; does NOT by itself fire a warp (Gate 2 is §3.2) |
| 0x4D94B0 | FootClass::Set_Destination_Internal | Dispatches Head_To_Coord to active loco (corrected 2026-07-12, was "Assign_Destination" — verified via decompile_function 0x4D94B0, RTTI_LABEL_DRIFT) |
| 0x4DA530 | FootClass::AI | Per-tick IPiggyback swap check |
| 0x73E5E0 | UnitClass::Mission_Harvest | 5-state harvest cycle |
| 0x4DF040 | FootClass::Find_Docking_Bay | Locate nearest refinery |
| 0x47EBA0 | CellClass::FindFirstUnit | Filters cell occupants for RTTI==1 (UnitClass); NOT a building check (corrected 2026-07-12, was "CellClass::FindFirstBuilding" — verified via decompile_function 0x47EBA0 + 0x746e20 [UnitClass::What_Am_I==1] + 0x459ec0 [BuildingClass::WhatAmI==6], RTTI_LABEL_DRIFT) |
| 0x65AAA0 | RadioClass::Transmit_Radio | Dock reservation protocol |
| 0x706640 | TechnoClass::Draw | Flag 0x2004 driven by cloak/IronCurtain visual-state (0x00703860), NOT +0x271 (corrected 2026-07-18 — see §6, INFERENCE_HARDENED) |
| 0x70C5C0 | TechnoClass::IsBeingWarped | Returns +0x271 byte; no static callers found this session (get_function_callers) |
| 0x70E5A0 | TechnoClass::UpdateTemporalVisual | 10-phase fade (temporal weapon, NOT teleport) |

---

## 10. Source Reports

Detailed decompiled code and assembly for all findings above:

- `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` — state machine, IPiggyback, Set_Destination
- `CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md` — byte-verified Set_Destination Teleporter-block predicate (Gate 1, §3.1 above)
- `CHRONO_MINER_WARP_TRIGGER_GHIDRA_REPORT.md` — StateMachineTick warp-arming mechanism (Gate 2, §3.2 above), open DockUnload-NavCom-caller question
- `CHRONO_WARP_VISUAL_RENDERING.md` — draw flags, temporal vs teleport visual
- `TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md` — field offset verification with evidence
- `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md` — Mission_Harvest states, ore scanning
- `HARVESTER_DOCK_UNLOAD.md` — docking, unloading, credits, exit facing
- `SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md` — slave miner, ore growth, ore values
- `ANIM_CLASS_GHIDRA_REPORT.md` — AnimClass/AnimTypeClass struct, constructor, AI, draw, lifecycle
- `LOCOMOTION_MATH_AND_CONSTANTS.md` — original chrono sections 7-8, INI constants
