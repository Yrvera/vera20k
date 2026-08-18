# NavCom Lifecycle — Ghidra Research Report

**Date:** 2026-04-23
**Primary addresses:**
- `0x00741970` — `TechnoClass::Set_Destination` (public entry point — **IS** invoked via vtable +0x480 for concrete leaf classes such as `UnitClass`; see §1/§3 corrected 2026-07-12)
- `0x004D94B0` — `FootClass::Set_Destination_Internal` (final writer)
- `0x004D82B0` — `FootClass::OnArrival` (queue pop + idle transition)
- `0x004DF0D0` — `FootClass::Stop_Moving` (naive zeroing)
- `0x004D9960` — `FootClass::PointerExpired` (cleanup on target death)
- `0x004D8F40` — `FootClass::Set_NavCom_With_Suspend` (aircraft-only)
- `0x004B0500` — `DriveLocomotionClass::Process` (the arrival signaller)

**Confidence:** HIGH on all direct-evidence findings; MEDIUM on some struct fields whose names came from the prior `FOOTCLASS_ENTER_QUEUE_AND_NAVCOM_SYSTEM.md` doc (marked inline).
**Active in YR:** **Yes** for the main lifecycle (Set_Destination → arrival → clear). Conditional / aircraft-only for the suspend path.

**This report extends and corrects `FOOTCLASS_ENTER_QUEUE_AND_NAVCOM_SYSTEM.md`.** Corrections are called out explicitly in §6.

**2026-05-27 correction note:** The NavCom reswarm supersedes several open questions and speculative labels in this older report. Use `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md`, `NAVCOM_ONARRIVAL_TAIL_HOOKS_GHIDRA_REPORT.md`, `NAVCOM_POINTEREXPIRED_RETENTION_BRANCHES_GHIDRA_REPORT.md`, `TECHNOCLASS_SET_DESTINATION_PREPROCESSING_FLAGS_GHIDRA_REPORT.md`, and `FOOTCLASS_SET_DESTINATION_GUARD_RECONCILIATION_GHIDRA_REPORT.md` for the corrected details. In brief: no standard YR player, TeamClass/AI, or trigger waypoint path was found to push Foot NavQueue; `+0x687` is a deferred vtable `+0x174` hook that resolves to Scatter for stock Unit/Infantry; `PointerExpired` has verified sensor and Occupier retention branches; `TypeClass+0xD6A` is `BalloonHover`; `TypeClass+0xD2C` is derived from `MovementZone=Subterannean`; and the alleged `Type+0xD28/+0xD29/+0xD2A` stop guard actually reads Unit/Foot instance bytes `+0x6E0/+0x6E1/+0x6E2`.

---

## 1. Overview

`NavCom` (+0x5A4 on every `FootClass`) is an `AbstractClass*` pointing at "where this unit is currently headed". Setting it kicks the locomotor into motion; clearing it is the arrival signal. Everything else — the queue, the suspended copy, the aux pointer, the retry timers — is scaffolding around that one primary write.

Three actors drive the lifecycle:

1. **`TechnoClass::Set_Destination`** (`0x00741970`) — the ~500-line preprocessing entry (locomotor swap, type-specific dispatch, approach-cell finding). Eventually calls `FootClass::Set_Destination_Internal`. **CORRECTED 2026-07-12 (was WRONG — STRUCT_FAMILY_CASCADE):** this doc previously claimed vtable+0x480 bypasses this function "for every ground unit, infantry, vehicle," citing a read of `0x007E9114` (`0x007E8C94`+0x480) = `0x004D94B0` (`Set_Destination_Internal`). That read is real, but `0x007E8C94` is **`FootClass`'s own vtable**, confirmed via `get_xrefs_to 0x007E8C94` → writes only from `FootClass::Constructor` (`0x004D3400`) and `FootClass::Destructor`. It is never the live vtable of a fully-constructed object: `UnitClass::Constructor`, `InfantryClass::Constructor`, and `AircraftClass::Constructor` each call `FootClass::Constructor` first, then overwrite `*this` with their own leaf vtable. For `UnitClass` — confirmed via `get_xrefs_to 0x007F5C70` (writes from `UnitClass::Constructor`/`Destructor`/`Load`) then `read_memory 0x007F60F0 4` (base `0x007F5C70`+0x480) → `70 19 74 00` = `0x00741970` — a real, live `UnitClass` instance's vtable+0x480 calls **`TechnoClass::Set_Destination` itself**, not `Set_Destination_Internal`. So for `UnitClass` (harvesters, tanks, and other ground vehicles), the §3 preprocessing runs on every ordinary vtable+0x480 call — it is not bypassed. This also matches this doc's own §5.1/§7.2 wording ("route through full Set_Destination preprocessing"), which assumed the now-corrected direction all along. `InfantryClass` and `AircraftClass` leaf vtable+0x480 slots are now **RESOLVED (corrected 2026-07-19, re-verified this session):** `0x0051AA40` is `InfantryClass`'s own vtable+0x480 slot — confirmed via `get_xrefs_to 0x0051AA40` → sole DATA xref `0x007EB4D8`, and `get_xrefs_to 0x007EB058` (`0x007EB4D8`−0x480) → writes only from `InfantryClass::Constructor`/`Destructor`/`Load`. `0x0041AA80` is `AircraftClass`'s own vtable+0x480 slot — confirmed via `get_xrefs_to 0x0041AA80` → sole DATA xref `0x007E2724`, and `get_xrefs_to 0x007E22A4` (`0x007E2724`−0x480) → writes only from `AircraftClass::Constructor`/`Destructor`/`Load`. Canonical Ghidra already carries these renames from a prior re-swarm pass (`FOOT_VTABLE_0X480_LEAF_SLOTS_GHIDRA_REPORT.md`): `get_function_callers 0x004D94B0` returns exactly three callers — `AircraftClass__Set_Destination @ 0041aa80`, `InfantryClass__Set_Destination @ 0051aa40`, `TechnoClass__Set_Destination @ 00741970` — confirmed this session via `decompile_function` on both leaf addresses (each contains direct `FootClass__Set_Destination_Internal` calls: 3 in `AircraftClass::Set_Destination`, 1 in `InfantryClass::Set_Destination`). The old label `UnitClass::EnterBuildingOrDock` on `0x0041AA80` was WRONG in every respect — it is not UnitClass's slot, and the function is not an "enter building/dock" handler. Full family: `TechnoClass`+0x480 = no-op stub (`0x00709A30`) → `FootClass` = `Set_Destination_Internal` committer (`0x004D94B0`) → `Unit`/`Infantry`/`Aircraft` = type-specific preprocessing overrides. See §3.8 and §7.1 for the corrected table rows, and §11 item 7 for the resolution record.
2. **`FootClass::Set_Destination_Internal`** — the single function that actually writes `NavCom` and tells the locomotor "go here" via `ILocomotion::Head_To_Coord`.
3. **`DriveLocomotionClass::Process`** (and analogs) — each tick, checks if the unit has reached NavCom's cell; on arrival calls `Set_Destination(NULL, 1)` (empty queue) or `Stop_Moving + OnArrival` (non-empty queue).

Mission_Move's role is peripheral: it only **monitors** the arrival state (NavCom==0 && loco stopped && no queued mission) and triggers `OnArrival` when the locomotor hasn't already. See `FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md`.

---

## 2. Struct Recap (NavCom fields only)

All offsets on `FootClass*`. These were verified in the prior doc and re-confirmed during this investigation:

| Offset | Type | Field | Purpose |
|--------|------|-------|---------|
| `+0x588` | DVec{24} | `NavQueue` | Waypoint queue. Buffer at `+0x58C`, Count at `+0x590`, active count (Count2) at `+0x598`, capacity 10. |
| `+0x5A0` | `AbstractClass*` | `NavCom_Aux` | Scratch/temp NavCom pointer. Cleared at the top of `Set_Destination_Internal` and by `Stop_Moving`. No writers set it to a non-null value in the decompilations examined; effectively dead in YR. |
| `+0x5A4` | `AbstractClass*` | **`NavCom`** | The primary destination pointer. Non-null means "heading to X"; null means "arrived or cancelled". |
| `+0x5A8` | `AbstractClass*` | `SuspendedNavCom` | Saved NavCom when aircraft suspends navigation for a weapon run. Only written by `Set_NavCom_With_Suspend`; only cleared by `PointerExpired`. |

Also relevant (written by `Set_Destination_Internal`, owned by FootClass retry system):

| Offset | Type | Field | Purpose |
|--------|------|-------|---------|
| `+0x640` | int | `PathRetryFrame` | `CurrentFrame` snapshot for the walker path-retry cadence |
| `+0x644` | int | — | Reset to 0 on new destination |
| `+0x648` | int | `WalkPathState` | Reset to 0 on new destination |
| `+0x668`/`+0x66C` | int | `BlockRetryFrame` | `CurrentFrame` snapshot |
| `+0x670` | int | `BlockRetryTimer` | Set to `RulesClass+0x1768 = BlockagePathDelay` (default 60 frames) on every new destination |
| `+0x6B7` | bool | `PathFailedFlag` | Cleared to 0 on every new destination |

---

## 3. `TechnoClass::Set_Destination` — the ~500-line preprocessing entry (IS the vtable+0x480 slot for concrete leaf classes such as `UnitClass` — corrected 2026-07-12, see §1)

**Signature:** `void __thiscall Set_Destination(this, AbstractClass* target, bool initiator)`

This is ~500 lines of dense, branch-heavy preprocessing. Every externally-visible "go to X" path lands here (player right-click, AI move orders, radio-triggered moves, harvester pathfind, etc.). The function's job is to figure out *what* target to set and *how* to set it before delegating to `Set_Destination_Internal`. Major sections:

### 3.1 Early-cancel: "clicking stop on an attack-move"

```
if (TypeClass+0xD6A && target == NULL && NavCom != NULL && ArchiveTarget != NULL) {
    if (NavCom == ArchiveTarget ||
        (NavCom->RTTI == 0xB /* CellClass */ &&
         NavCom->OwnerHouse == ArchiveTarget)) {
        // Trying to cancel but NavCom already matches archive — clear mission instead
        if (vtable+0x184() != 1) vtable+0x1F0();  // force-idle
        return;
    }
    // path-timeout fallback via FUN_004D03D0
}
```

`TypeClass+0xD6A` is now verified as `BalloonHover=`. This branch is a BalloonHover null-destination intercept; it is not an AutoAttackMove-family or chrono-miner flag. See `TECHNOCLASS_SET_DESTINATION_PREPROCESSING_FLAGS_GHIDRA_REPORT.md`.

### 3.2 Null-target path

```
if (target == NULL && !IsForcedMove) {
    FootClass::Stop_Moving();   // zero NavCom + NavCom_Aux
    return;
}
```

The later stop guard in this function is not driven by `TypeClass+0xD28/+0xD29/+0xD2A`. Those TypeClass fields are `Crusher`, `OmniCrusher`, and `OmniCrushResistant`; the stop guard reads Unit/Foot instance bytes `+0x6E0/+0x6E1/+0x6E2` plus owner `+0x2B0`. If the unit cannot accept the null-destination order, the function can route to `Stop_Moving` and exit. **This is why a player click on empty space acts as a stop**: `Set_Destination(NULL, 1)` → `Stop_Moving()` → `NavCom = 0` → Mission_Move sees NavCom=0 next tick → `OnArrival` → transitions to idle.

### 3.3 Same-target early-return

```
if (target == NavCom && IsCommenced == false) return;
```

If the caller is trying to re-assign the same destination and the current mission hasn't commenced yet (`byte +0x94 == 0`), do nothing. Prevents redundant re-pathfinding when the same order arrives twice.

### 3.4 Aircraft helipad routing

If `vtable+0x184() == 0x10` (`AircraftClass`) and the destination is a helipad-capable building, look up the building in the cell and potentially re-route to its landing pad (`MapClass::Get_CellClass → Look_up_building_in_cell → BuildingClass::ClearAnimSlot`). This is the "send aircraft to helipad, not the cell it's clicked on" heuristic.

### 3.5 Enter-building / garrison preprocessing

Mission 7 (Enter) + target is a building/infantry triggers the "enter" interpretation. Several sub-cases:
- `target->TypeClass+0x16B3` (CanBeOccupied / garrisonable): `Transmit_Radio(0x0E /* QUEUE_ENTER */)` to queue entry; on ROGER, swap target to the building's current NavCom.
- `target->TypeClass+0x16AB` (InfiltrateCivilian — spy): if target is civilian + spyable, set ghost cell and route; if target is garrisonable but already full, fall back to standard move.
- `target->TypeClass+0x16A9` (Engineer target): depends on building type and unit's health ratio vs `RulesClass+0x16F8` (`ConditionYellow` or similar). Engineer can capture above threshold; below threshold it falls back to move.
- `target->TypeClass+0x16AE` (`Passengers>0`, transport): add to `EnterQueue` via direct buffer write if this isn't already in it.

### 3.6 Piggyback locomotor swap

For units with `TypeClass+0xD2C` (derived `MovementZone == Subterannean`, not `Teleporter=`), the handler checks the current locomotor's CLSID and can run subterranean/bridge/passability destination rewrite logic. Teleporter-specific locomotor swap uses `TypeClass+0xCD4`.

- If **not** `CLSID_DriveLocomotion`: destroy the current locomotor, instantiate a new `CLSID_DriveLocomotion` via `COM::CoCreateInstance_Locomotor`, piggyback it on top of the existing via `IPiggyback`.
- If current is DriveLocomotion and moving to a `CLSID_TeleportLocomotion` target: check zone/cell match and swap back to Teleport for the warp phase.

This is the chrono-miner style two-locomotor dance: Drive does ground movement, Teleport takes over for the actual warp jump. The NavCom is set once; the two locomotors cooperate.

For `HoverLocomotion` targets (e.g., Rocketeer landing near a pier), a similar swap happens via `CLSID_HoverLocomotion` matching.

### 3.7 Bridge-aware ghost-cell handling

If the target cell is flagged as bridged (`cell+0x140 & 0x100`), check whether `this` and `target` are on the *same* bridge via `MapClass::FindBridgeRecord`. If they're on different bridges, or on no bridge at all when one is expected, walk the bridge offset math with `Sqrt_Approx` and `Math::ftol`. Sets `cStack_7d` which gates the final `Set_Destination_Internal` call.

### 3.8 Final call

At the end of every successful path:

```
FootClass::Set_Destination_Internal(this, param_2);
```

Note the decompiler renders this as `FootClass__Set_Destination_Internal(unaff_retaddr, param_2)` — the `unaff_retaddr` is Ghidra's confusion about the `ECX` register after the long function; it's actually `this`.

**Observation:** `TechnoClass::Set_Destination` has **zero direct callers** shown by Ghidra's xref search — it is reached exclusively through vtable dispatch. **Corrected 2026-07-12 (was WRONG — OFFSET_RETYPED_WRONG + RTTI_LABEL_DRIFT):** the address previously cited here, `0x007F60F0`, is not a vtable base to which `+0x480` should be added — it IS the `+0x480` slot's address already. `read_memory 0x007F60F0 4` → `70 19 74 00` = `0x00741970` confirms the slot's contents directly at offset 0. The true base is `0x007F5C70` (`0x007F60F0 − 0x480`), confirmed via `get_xrefs_to 0x007F5C70` to be **`UnitClass`'s** own vtable (writes from `UnitClass::Constructor`/`Destructor`/`Load`), not a generic "TechnoClass vtable." So: for `UnitClass`, ordinary vtable dispatch (right-click move, AI orders, etc.) reaches `TechnoClass::Set_Destination` directly — this is the normal route, not a special-case "non-FootClass slot." Self-call paths (Helipad re-route, see §7.2) are additional, real re-entry points, but not the primary route. The `FootClass`-only vtable+0x480 slot at `0x007E9114` does contain `FootClass::Set_Destination_Internal` (`0x004D94B0`) — that specific byte read was correct — but per §1, `0x007E8C94` is never a live object's vtable, so this fact describes no reachable runtime path for a real unit. **RESOLVED 2026-07-19 (re-verified this session):** the direct-call bypass into `Set_Destination_Internal` at `0x0041AA80` is real (confirmed via `decompile_function 0x0041AA80`: three direct `FootClass__Set_Destination_Internal(...)` calls), and the address is **`AircraftClass::Set_Destination`**, not `UnitClass::EnterBuildingOrDock` — the old label was wrong on both the owning class and the function's role. Confirmed via `get_xrefs_to 0x0041AA80` → sole DATA xref `0x007E2724`, and `get_xrefs_to 0x007E22A4` (`0x007E2724`−0x480, the vtable base) → writes only from `AircraftClass::Constructor`/`Destructor`/`Load`. Canonical Ghidra already carries a plate comment on `0x0041AA80` from a prior re-swarm (`FOOT_VTABLE_0X480_LEAF_SLOTS_GHIDRA_REPORT.md`) naming it `AircraftClass::Set_Destination` and stating it does aircraft-specific airfield-docking preprocessing before committing via the shared `FootClass` committer. The sibling `InfantryClass` slot at `0x0051AA40` is analogously `InfantryClass::Set_Destination` (confirmed via `get_xrefs_to 0x0051AA40` → sole DATA xref `0x007EB4D8`; one direct `FootClass__Set_Destination_Internal` call in its body). `get_function_callers 0x004D94B0` lists exactly these three: `AircraftClass__Set_Destination @ 0041aa80`, `InfantryClass__Set_Destination @ 0051aa40`, `TechnoClass__Set_Destination @ 00741970` — the complete Unit/Infantry/Aircraft override family. See §1 and §11 item 7.

---

## 4. `FootClass::Set_Destination_Internal` — the actual writer

**Signature:** `void __thiscall Set_Destination_Internal(this, AbstractClass* target)`

This is the only function that *directly assigns* `NavCom = target`. Everything else routes through it (or through `Stop_Moving`, which bypasses it). Full control flow:

```
[STEP 1] NavCom_Aux = NULL   // always, first instruction

[STEP 2] Guard conditions (only if target != NULL):
   if (+0x6AD /* deploy/locomotor-piggyback active guard */)      return;
   if (+0x82  /* secondary guard, byte */) return;  // verified via decompile_function 0x004D94B0
   if (+0x2E4 /* WarpedOutOf */)      return;
   if (+0x2AC /* ChronoTarget-ish */) BuildingClass::DeployUnit_ChronoWarp(true);
   // Note: NavCom is NOT yet written. A non-null target is silently dropped
   // here — the caller's order just fails without indication.

[STEP 3] NavCom = target   // the one instruction that matters

[STEP 4] Special null-target handling:
   if (target == NULL && +0x6AD != 0 && +0x2B0 != 0) {
      // Linked deploy/piggyback cleanup
      (*(this + 0x2B0))->field_0x2AC = 0;
      this->field_0x2B0 = 0;
      +0x6AE = 1;
   }

[STEP 5] If NavCom is still NULL (target was NULL):
   int what = this->What_Am_I();   // vtable+0x2C
   if (what == 2 /* AircraftClass — corrected 2026-07-19: was mislabeled "UnitClass"; RTTI 2 =
                    AircraftClass, RTTI 1 = UnitClass. Verified this session via
                    decompile_function 0x00746e20 (UnitClass::What_Am_I returns 1) and
                    decompile_function 0x00523340 (InfantryClass::What_Am_I returns 0xF); the
                    RTTI-2 constant is produced by raw code at AircraftClass's own vtable+0x2C
                    slot 0x0041C180 (read_memory: `b8 02 00 00 00 c3` = mov eax,2; ret), reached
                    from AircraftClass's own vtable base 0x007E22A4 (get_xrefs_to 0x007E22A4 →
                    writes only from AircraftClass::Constructor/Destructor/Load). See §4.2. */ &&
       (CurrentMission == 1 /* Attack */ || QueuedMission == 1) &&
       TarCom != NULL) {
      // Attacking aircraft with a target — don't tell the locomotor to stop (corrected 2026-07-19,
      // was "attacking vehicle"; see §4.2 for the reclassification and its open caveat).
      // The locomotor continues its current approach arc while the weapon fires.
   } else {
      assert(Locomotor != NULL);
      Locomotor->Clear_Navigation();   // loco vtable+0x48
      NavCom = target;                 // re-assert (target == NULL here)
   }
   goto RETRY_TIMER_RESET;

[STEP 6] NavCom is non-null, normal path:
   if (+0x304 /* PreviousLocomotor / tethered loco */ != NULL) {
      PreviousLocomotor->Release();   // vtable+0xF8
      +0x304 = NULL;
   }

   // Piggyback sanity check on the active locomotor
   LocomotionClass::QueryInterface_IPiggyback(&locoLocal);
   // (+ two asserts for piggyback validity)

   // Check active locomotor's CLSID
   locoLocal->GetClassID(&clsid);
   if (clsid == CLSID_WalkLocomotion) {
      // Walker: reset walker-specific retry state if not recently reset
      int wait = +0x648;    // WalkPathState
      if (+0x640 /* PathRetryFrame */ != -1) {
         int elapsed = CurrentFrame - +0x640;
         if (elapsed < wait) wait -= elapsed;
      }
      if (wait == 0) {
         +0x640 = CurrentFrame;
         +0x644 = 0;
         +0x648 = 1;
      }
   }

   if (+0x6AC /* one-shot skip_head_to_coord_once */ == 0) {
      // FETCH DESTINATION COORDS from the NavCom target
      coord = NavCom->vtable[0x4C](this);    // Get_Dock_Coord / Get_Approach_Coord
      assert(Locomotor != NULL);
      Locomotor->Head_To_Coord(coord.x, coord.y, coord.z);   // loco vtable+0x44
   } else {
      +0x6AC = 0;
      // Skip Head_To_Coord for this call only; NavCom was still written.
   }

[STEP 7] RETRY_TIMER_RESET (always runs, null or not):
   +0x6B7 = false;                              // PathFailedFlag (verified via decompile_function 0x004D94B0 — last LAB_004d96c2 block writes `*(undefined1 *)((int)param_1 + 0x6b7) = 0`)
   +0x668, +0x66C = CurrentFrame, (unused_EBX)  // BlockRetryFrame
   +0x670 = RulesClass+0x1768;                  // BlockRetryTimer = BlockagePathDelay (60)
   +0x640 = CurrentFrame;                       // PathRetryFrame
   +0x644 = 0;
   +0x648 = 0;                                  // WalkPathState
```

### 4.1 The 4 "silent drop" guards

A non-null `Set_Destination_Internal(target)` call can silently return without writing NavCom if any of:

- `+0x6AD` (deploy/locomotor-piggyback active guard) is nonzero
- `+0x82` (secondary guard, single byte) is nonzero — verified via `decompile_function 0x004D94B0`: `if ((*(char *)((int)param_1 + 0x82) != '\0') && (param_2 != 0)) return;`
- `+0x2E4` (WarpedOutOf / ChronoWarping) is nonzero

This matters for parity: a player order issued while a unit is mid-deploy or mid-warp is *silently swallowed*, not queued or retried. The engine does not surface any feedback. A Rust port that queues these orders for retry will behave differently.

### 4.2 The "attacking aircraft, don't stop locomotor" exception (corrected 2026-07-19)

**Correction (2026-07-19, re-verified from the live binary this session):** the `What_Am_I() == 2` check does **not** mean `UnitClass`. RTTI `1` = `UnitClass` (`decompile_function 0x00746e20` → `return 1;`), RTTI `0xF` = `InfantryClass` (`decompile_function 0x00523340` → `return 0xf;`), and RTTI `2` = `AircraftClass` — the constant is produced by the code sitting at `AircraftClass`'s own vtable+0x2C slot, `0x0041C180` (`read_memory 0x0041C180 16` → `b8 02 00 00 00 c3 90...` = `mov eax, 2; ret`, immediately followed by `int3`/NOP padding, i.e. a genuine tiny leaf function Ghidra has not auto-defined), reached from `AircraftClass`'s own vtable base `0x007E22A4` (`get_xrefs_to 0x007E22A4` → writes only from `AircraftClass::Constructor`/`Destructor`/`Load`; cross-checked against the known-good `UnitClass` vtable base `0x007F5C70`, whose `+0x2C` slot correctly resolves to `0x00746e20` = `UnitClass::What_Am_I`).

So this carve-out fires for **`AircraftClass` instances**, not ground vehicles. The `What_Am_I == 2 && Mission == Attack && TarCom != NULL` carve-out prevents a `Set_Destination(NULL)` from aborting an **attacking aircraft's** current approach-arc when it's mid-attack-movement. The locomotor keeps its current `Head_To_Coord` target and the aircraft finishes its turn-toward-target arc, even though NavCom just got cleared. **Open question (not resolved this session):** the original "tank strafing while attacking" framing is retired as WRONG (ground vehicles are RTTI 1, not 2), but this session did not re-derive what the equivalent player-visible aircraft behavior actually looks like (e.g. whether it reproduces the "keep circling/strafing while shooting" feel for gunships/aircraft specifically, or something else) — tracked as new item in §11.

### 4.3 The locomotor vtable contract

Two virtual calls on the locomotor:
- `Head_To_Coord(x, y, z)` at **vtable +0x44** — tells the locomotor "start moving to (x, y, z) in the world"
- `Clear_Navigation()` at **vtable +0x48** — tells the locomotor "stop any pending move"

And one virtual call on the NavCom target:
- `Get_Dock_Coord(follower)` at **target vtable +0x4C** — returns the coord *at which the follower should arrive*. For `CellClass` this is the cell center; for `BuildingClass` this is the docking bay coord; for a `UnitClass` (e.g., passenger boarding a transport) this is the transport's current position.

The concrete `DriveLocomotionClass::Set_Destination @ 0x4AFD40` (= Head_To_Coord impl):
```
if (owner->vtable+0x37C()) return;   // early abort: some state check
if (owner->vtable+0x380()) return;   // early abort
if (owner->vtable+0x1D4()) return;   // early abort
if (owner->vtable+0x1D8()) return;   // early abort

this->DestX = x;  this->DestY = y;  this->DestZ = z;   // +0x30..+0x38
if ((x,y,z) != NullCoord) {
   cell = MapClass::Get_Cell_At(x, y);
   if (cell.Flags & 0x100 /* bridged */) {
      this->DestZ += g_BridgeZOffset_Drive;
   }
}
```

So at the locomotor level, the destination is just three coord fields. Bridge height is added automatically on write.

---

## 5. Arrival flow: who tells Mission_Move it's done

This is the part the prior doc didn't cover. The short version: **the locomotor is the source of truth for arrival**, not the mission handler.

### 5.1 `DriveLocomotionClass::Process` (0x4B0500), the per-frame driver

Called every frame from `FootClass::AI → ILocomotion::Process` (vtable +0x40). The relevant arrival code (paraphrased):

```
// Per-frame cell-bump check
new_groundlevel = MyCell->GroundLevel;  // cell+0x11C
if (new_groundlevel != this->LastGroundLevel) {
   // Cell-level change — start a 3-frame transition timer
   this->LastGroundLevel = this->PrevGroundLevel;
   this->PrevGroundLevel = new_groundlevel;
   this->HeightChangeTimer = 3;
}

if (not mid-track) {
   // CASE 1: NavCom is a CellClass
   if (owner->NavCom != NULL && owner->NavCom->RTTI == 0xB /* CellClass */) {
      CellStruct my_cell = owner->Get_Current_Cell();   // owner vtable+0x1B8
      if (my_cell == NavCom->CellCoord) {
         // Arrived at NavCom's cell
         if (owner->NavQueue.Count2 == 0) {
            // Empty queue — route through full Set_Destination preprocessing
            owner->Set_Destination(NULL, 1);     // vtable+0x480
            // This zeros NavCom via the null-target path of
            // Set_Destination → Stop_Moving → NavCom = 0
         } else {
            // Non-empty queue — direct path
            FootClass::Stop_Moving();            // zero NavCom + NavCom_Aux
            owner->OnArrival(0, 1);              // vtable+0x484 → pop next waypoint
         }
         return;
      }
   }

   // CASE 2: Mission == Move (5) and we're at the destination cell
   if (owner->CurrentMission == 5 /* Guard? or Move? */ && !arrived_flag &&
       this->DestCurrent != NullCoord &&
       owner->Location == this->DestCurrent) {
      // Same empty-queue / non-empty-queue split
      if (owner->NavQueue.Count2 == 0) owner->Set_Destination(NULL, 1);
      else { FootClass::Stop_Moving(); owner->OnArrival(0, 1); }
      return;
   }

   // CASE 3: Re-aim at moving NavCom target (if it moved)
   if (!is_crashing && NavCom != NULL) {
      coord = NavCom->vtable[0x4C](owner);   // Get_Dock_Coord (fresh)
      this->Head_To_Coord(coord);            // re-aim if changed
   }
}

// ... Process_Movement drives the actual sub-cell motion ...

// At end of Process: check if we've become fully idle
if (DestCurrent == NullCoord && DestCurrent_Old == NullCoord &&
    owner->Target == -1 && owner->TypeClass+0x578 /* some idle threshold */ > 0) {
   owner->SetSpeedFraction(0.0);     // vtable+0x544
}
```

**Key insight:** When the locomotor reaches the NavCom cell with no queued waypoints, it calls `Set_Destination(NULL, 1)` on itself — which routes back through `TechnoClass::Set_Destination`, hits the null-target branch, and calls `Stop_Moving`. This is *why* `Stop_Moving` is listed as a callee of so many locomotor functions: the locomotor triggers its own arrival via the public API.

### 5.2 Complete arrival chain for a typical ground-move order

```
Frame N:    player right-clicks empty cell
            → Command::Move issued
            → vtable+0x480(cell, 1) = TechnoClass::Set_Destination(cell, 1)
              → preprocessing, no special case
              → FootClass::Set_Destination_Internal(cell)
                → NavCom_Aux = 0
                → NavCom = cell
                → Locomotor->Head_To_Coord(cell.center)
                → BlockRetryTimer = 60

Frames N..N+K-2:
            → DriveLocomotionClass::Process each frame
              → Process_Drive_Track / Process_Movement advance sub-cell lepton
              → Every frame, re-aim at NavCom coord (idempotent if static target)
            → Mission_Dispatch every ~14-16 frames
              → Mission_Move: NavCom != NULL, return timer

Frame N+K-1 (entering last cell):
            → DriveLocomotionClass::Process
              → cell-bump check detects owner cell == NavCom cell
              → NavQueue empty → Set_Destination(NULL, 1)
                → Stop_Moving()
                  → NavCom_Aux = 0, NavCom = 0

Frame N+K (or N+K plus dispatch timer):
            → Mission_Dispatch calls Mission_Move
              → NavCom == 0, Is_Moving_Now returns false (loco has been reset)
              → QueuedMission == -1
              → OnArrival(0, 1)
                → NavQueue empty → fall through
                → SetSpeedFraction(0.0)  [vtable+0x544]
              → return 1
            → Next dispatch: unit is in Move mission with no NavCom, no loco
              → Some mission transition triggers Set_Mission(Guard) — via the
                attack-move intent in Arrival_Target_Handler or via external
                idle handling. Not traced further in this report.

Frame N+K+M (idle wind-down):
            → DriveLocomotionClass::Process
              → all DestCurrent == NullCoord, Target == -1
              → SetSpeedFraction(0.0) idempotent call
```

Two `SetSpeedFraction(0.0)` calls happen: once from `OnArrival`, once from the locomotor's fully-idle tail. Both set the same double at owner+0x578 to 0.0. The TechnoClass::SetSpeedFraction body is trivial: clamp to [0, 1].

### 5.3 Why two parallel paths (Set_Destination(NULL) vs Stop_Moving+OnArrival)?

From the DriveLoco decomp:
- **Empty queue**: go through `Set_Destination(NULL, 1)` because it performs the full teardown (clears path-retry state, resets locomotor via `Clear_Navigation`, issues the ghost-cell update). Heavier, but correct for a "truly arrived" state.
- **Non-empty queue**: bypass that teardown with `Stop_Moving + OnArrival` because `OnArrival` will immediately re-issue `Set_Destination` with the next waypoint. No point tearing down what will be rebuilt.

A Rust port that treats these the same will either over-work (empty-queue running the cheap path) or leave dangling state (non-empty running the expensive path only to be overridden).

---

## 6. Corrections to the prior doc

`FOOTCLASS_ENTER_QUEUE_AND_NAVCOM_SYSTEM.md` has two factual errors and one missing detail that I verified by decompilation:

### 6.1 `Set_NavCom_With_Suspend` body was wrong

The prior doc at line 213-219 shows:
```c
void Set_NavCom_With_Suspend(FootClass* this, mission, p3, p4) {
    this->SuspendedNavCom = this->NavCom;    // OK
    this->Target = this->NavCom;             // WRONG — this assignment does not exist
    Override_Mission(mission, p3, p4);
    Set_Destination(p4, true);
}
```

**Actual body** (from decompilation of 0x4D8F40):
```c
void Set_NavCom_With_Suspend(FootClass* this, int mission, int target_rtti, AbstractClass* destination) {
    this->SuspendedNavCom = this->NavCom;             // +0x5A8 = +0x5A4
    TechnoClass::Override_Mission_And_Target(         // FUN_007013A0
        this, mission, target_rtti, destination);
    this->vtable[0x480](destination, 1);              // Set_Destination(destination, true)
}
```

Where `TechnoClass::Override_Mission_And_Target (0x7013A0)` is:
```c
void Override_Mission_And_Target(this, mission, target_rtti, target_ptr) {
    this->ArchiveTarget = this->Target;               // +0x2B8 = +0x2B4 (save for restore)
    MissionClass::Override_Mission(mission, target_rtti, target_ptr);
    this->Set_ArchiveTarget(target_rtti);             // vtable+0x3C8
}
```

**Correct semantics:** `Set_NavCom_With_Suspend` saves both NavCom (→SuspendedNavCom) *and* Target (→ArchiveTarget), then overrides the mission and sets the new destination. There is no `Target = NavCom` assignment.

### 6.2 `Set_NavCom_With_Suspend` has exactly one caller

The prior doc at line 222-223 said:
> "Used by AircraftClass::Set_NavCom_Override (0x41BB30) to temporarily redirect aircraft during specific mission types (4=attack, 0x1A-0x1F=special)."

Correct on the caller, but the mission gate was described imprecisely. Actual body of `AircraftClass::Set_NavCom_Override`:

```c
void AircraftClass::Set_NavCom_Override(this, new_mission, rtti, target) {
    switch (this->CurrentMission) {
        case 4:     // Retreat
        case 0x1A:  // (aircraft-specific)
        case 0x1B:  // (aircraft-specific)
        case 0x1E:  // (aircraft-specific)
        case 0x1F:  // (aircraft-specific)
            if (this->+0x294 == 0) {  // (corrected 2026-05-28: was != 0; binary shows == 0 enters the restriction block; via decompile_function 0x0041BB30 — OPERATOR_OR_ORDER_DRIFT)
                // Only suspend when going to another flight-mission
                switch (new_mission) {
                    case 4: case 0x1A: case 0x1B: case 0x1E: case 0x1F:
                        break;  // allowed — fall through to Set_NavCom_With_Suspend
                    default:
                        return; // silently refuse
                }
            }
            // If +0x294 != 0 ("busy"), skip the restriction entirely — always allow
            break;
        default:
            break; // non-flight missions always pass through
    }
    FootClass::Set_NavCom_With_Suspend(new_mission, rtti, target);
}
```

**Active in YR: Conditional.** Only active for aircraft in flight-missions transitioning to another flight-mission (e.g., Retreat → some 0x1A-0x1F aircraft weapon mission). Ground units never invoke this path; there's no analogous infantry or vehicle helper.

### 6.3 `PointerExpired` NavCom-clear is conditional, not unconditional

The prior doc at lines 455-462 showed:
```c
if (this->NavCom == expired) {
    // Complex logic: check if unit is in harvest mission targeting dying tiberium,
    // check sensor coverage, etc. before clearing.
    this->NavCom_Aux = NULL;
    this->NavCom = NULL;
}
```

**The "complex logic" is actually two distinct conditions** (decompiled from 0x4D9960):

```c
if (this->NavCom == expired) {
    bool should_clear = true;

    // Condition A: "Visible-cell retention" — if caller said "soft expire"
    //   (param_3 == 0) and the expired object was a tangible ObjectClass
    //   (flags & 1) and its cell is sensor-covered by our owner, then
    //   DO NOT clear NavCom. The unit can still "see" where it was and
    //   continue pathing to the cell.
    if (param_3 == 0 && expired != NULL && (expired->Flags & 1) /* +0x14 */) {
        coord = expired->Get_Coord();
        cell = MapClass::Get_Cell_At(coord);
        if (cell->SensorCountForHouse(this->Owner->+0x30) != 0) {
            should_clear = false;
        }
    }

    // Condition B: "Infantry capturing InfantryClass target in mission 8"
    //   — if we're InfantryClass in Mission_Capture targeting an InfantryClass
    //   that still has flags & 2, is IsActive, has Strength > 0, and isn't
    //   in Mission_Selling, DO NOT clear. Infantry → infantry captures are
    //   retained across the expire.
    int what = this->What_Am_I();
    if (what == 8 /* ??? */ && this->RTTI_via_2C == 0xF /* InfantryClass */ &&
        this->Type+0xEB4 /* Some TS-era flag */) {
        AbstractClass* nav = this->NavCom;
        if (nav != NULL && (nav->Flags & 2) &&
            (char)nav->IsActive /* +0x90 */ && nav->Strength /* +0x6C */ > 0 &&
            nav->What_Am_I() != 0x13 /* Selling */) {
            // passes — fall through with should_clear preserved
        } else {
            // Fails — the rest of the branch flips should_clear semantics
        }
    }

    if (should_clear) {
        this->NavCom_Aux = NULL;
        this->NavCom = NULL;
    }
}
```

The Condition A rule **is the "a crashing aircraft you ordered an attack on" preservation**: you told a unit to attack a target, the target blew up; if the target's cell is still within your sensor range, the unit keeps moving to that cell (as if it were a ground order). If the cell is outside sensor range (no idea where it went), clear NavCom.

This is observable behavior in gamemd.exe and relevant to parity: a Rust port that always clears NavCom on expire will have units stop when their target dies, not finish the approach.

### 6.4 The NavCom **push** side was audited in the 2026-05-27 reswarm

`NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` supersedes the hypotheses below. It found no active standard YR player command, TeamClass/AI movement, or trigger waypoint path that appends to Foot NavQueue; those systems route through vtable `+0x480` / `Set_Destination` instead. The only positive population path found is `FootClass::Load @ 0x004DB3C0`, which reconstructs serialized queue entries from save data. `OnArrival`, `Mission_Enter`, and `PointerExpired` are consumers/cleanup paths only. Treat the older bullets in this section as historical investigation notes, not current conclusions.

The prior doc documents `OnArrival` popping from the NavQueue (at +0x598, Count2). **No function in the decompilations examined writes to that queue's buffer.** Ghidra's function search for `Queue_Navigation`, `Queue_NavCom`, and related names returns nothing. The only write surface found was `FootClass::PointerExpired`'s shift-left cleanup.

Three hypotheses:
1. **Map scripting path** — trigger actions in `.map`/`.mpr` files can push NavCom waypoints. Not traced here.
2. **TeamClass/AI path** — the AI's goal stack may push via a helper; the team-waypoint system lives in TeamClass.
3. **Dead feature** — the queue is a TS-era holdover that no active YR code populates, kept for serialization compatibility. Given `FootClass::Save` and `Load` both read/write the queue count, it must deserialize correctly, but zero-count is the norm.

**Practical consequence for parity:** In standard YR player-vs-AI skirmish, normal movement should not append to Foot NavQueue. A shift-right-click on a second destination should be modeled through the verified command/Set_Destination path, not by preserving Rust's previous `nav_queue.push` behavior.

(For AI team waypoint chaining, the later producer audit already checked the standard TeamClass movement slice and found member destinations are issued through `Set_Destination`, not Foot NavQueue appends.)

---

## 7. Complete Producer/Consumer Enumeration

### 7.1 Who writes NavCom (+0x5A4)

Only two functions *directly* assign `NavCom = X`:

| Function | Assigns | Route in |
|----------|---------|----------|
| `FootClass::Set_Destination_Internal` (0x4D94B0) | `NavCom = target` (step 3 of §4) | Via `TechnoClass::Set_Destination` (0x741970, `UnitClass`'s own vtable+0x480 route), or direct from `AircraftClass::Set_Destination` (0x41AA80) / `InfantryClass::Set_Destination` (0x51AA40) — corrected 2026-07-19, was "direct from `UnitClass::EnterBuildingOrDock`"; see §1/§3.8 |
| `FootClass::Stop_Moving` (0x4DF0D0) | `NavCom = 0`, `NavCom_Aux = 0` | Called from ~15 places (see §7.3) |

Also `PointerExpired` conditionally sets `NavCom = 0` (see §6.3).

And `Set_NavCom_With_Suspend` copies `SuspendedNavCom = NavCom` but doesn't *write* NavCom itself — it calls through `Set_Destination` to do that.

### 7.2 Call sites of `TechnoClass::Set_Destination` (vtable +0x480)

Not statically enumerable via Ghidra — these are vtable calls. Known hits from the decompilations traced in this investigation:

| Caller | Context | Arguments |
|--------|---------|-----------|
| `FootClass::OnArrival` (0x4D82B0) | NavQueue pop | `(NavQueue.Buffer[0], 0)` |
| `FootClass::Set_NavCom_With_Suspend` (0x4D8F40) | Aircraft suspend-nav | `(new_destination, 1)` |
| `DriveLocomotionClass::Process` (0x4B0500) | Cell-match arrival, empty queue | `(NULL, 1)` |
| `AircraftClass::Mission_Move` state 0 (0x4166C0) | Null NavCom path / approach cell | `(NULL, 1)` or `(approach_cell, 1)` |
| `FUN_0051F660` (InfantryClass::Mission_Move) | Sequence 0x1B-0x1E cancel | `(NULL, 1)` |
| `TechnoClass::Set_Destination` (0x741970) | Helipad re-route, bridge adjust | `(modified_target, 1)` (self-call) |
| `UnitClass::Mission_Harvest` (0x73E5E0) | Harvester moves to ore | `(cell, 1)` |
| `UnitClass::PerCellProcess` (0x739EC0) | Per-cell trigger dispatch (refinery credit deposit, garrison/transport entry, bridge exit, deploy, mine damage) — several vtable+0x480 call sites inside, args vary by branch | `(target, 1)` (corrected 2026-07-12: row was labeled "UnitClass::Mission_Enter"; `get_function_by_address 0x00739EC0` returns `UnitClass__PerCellProcess`, and `decompile_function 0x00739EC0` shows a per-cell-trigger dispatcher, not a dedicated enter-mission handler — RTTI_LABEL_DRIFT) |
| `InfantryClass::PerCellProcess` (0x519630) | Per-cell trigger dispatch, including garrison/transport entry | `(target, 1)` (corrected 2026-07-12: row was labeled "InfantryClass::Mission_Enter (0x5196A0)"; `0x5196A0` is not a function entry — it falls inside `InfantryClass::PerCellProcess`, which starts at `0x519630` per `get_function_by_address 0x005196A0` and `0x00519630` — RTTI_LABEL_DRIFT / address pointed mid-function) |
| `BuildingClass::MissionRepairAndProduce` (0x44B780) | Produced unit eject | `(exit_cell, 1)` |
| Player command handlers | right-click move, attack-move, etc. | `(target, 1)` |

Every non-arrival call passes `1` (true) as the initiator flag. Only the empty-queue-arrival and NavQueue-pop paths pass `0`/`1`/`1`/`0` varied.

### 7.3 Call sites of `FootClass::Stop_Moving` (0x4DF0D0)

From `get_function_callers`:

| Caller | Context |
|--------|---------|
| `BuildingClass::MissionRepairAndProduce` (0x44B780) | Service bay halts the serviced unit |
| `DriveLocomotionClass::Process` (0x4B0500) | Arrival with non-empty queue |
| `DriveLocomotionClass::Process_Drive_Track` (0x4B0F20) | Mid-track interruption |
| `DriveLocomotionClass::Process_Movement` (0x4B2630) | Mid-move abort (blocked/invalid) |
| `DriveLocomotionClass::Stop_And_Scatter` (0x4B4890) | Scatter-triggered stop |
| `ShipLocomotionClass::Process` / variants (0x69FC10, 0x6A05F0, 0x6A1C80) | Same as Drive but for ships |
| `FUN_0051A2B0` (unnamed InfantryClass) | Infantry-specific mid-action stop |
| `FUN_006A3EC0` (unnamed ship) | Ship-specific mid-action stop |
| `InfantryClass::Fire_At_Target` (0x5206B0) | Infantry halts to fire |
| `InfantryClass::PerCellProcess` (0x519630) | Per-cell dispatch, including entry commit (corrected 2026-07-12: row was "InfantryClass::Mission_Enter (0x5196A0)"; actual caller per `get_function_callers 0x004DF0D0` is `InfantryClass__PerCellProcess @ 0x519630` — RTTI_LABEL_DRIFT) |
| `UnitClass::PerCellProcess` (0x739EC0) | Per-cell dispatch, including entry commit (corrected 2026-07-12: row was "UnitClass::Mission_Enter"; `get_function_by_address 0x00739EC0` returns `UnitClass__PerCellProcess` — RTTI_LABEL_DRIFT) |
| `UnitClass::Mission_Harvest` (0x73E5E0) | Harvester stops at ore |
| `TechnoClass::Set_Destination` (0x741970) | Null-target branch |

Note: Mission_Move itself does NOT call `Stop_Moving`. It only triggers `OnArrival`, which does not call `Stop_Moving` either. The locomotor's `Process` is the actual invoker during normal arrival.

### 7.4 Who calls `Clear_Navigation` on the locomotor

Only `Set_Destination_Internal` during its null-target branch. The locomotor's own `Stop_Moving` (e.g., `DriveLocomotionClass::Stop_Moving @ 0x4AFE00`) also resets `DestCurrent → NullCoord`, but that's the locomotor's internal coord — it does NOT null the owner's NavCom.

### 7.5 Who calls `Head_To_Coord` on the locomotor

- `Set_Destination_Internal` during its non-null branch (primary)
- `DriveLocomotionClass::Process` during per-frame re-aim (keeps moving target fresh)
- `AircraftClass::Mission_Move` state 1 (explicit aircraft approach)
- Possibly others not traced in this investigation

---

## 8. State Machine Diagram

```
                    ┌─────────────────────────────────────────────────────┐
                    │                                                     │
                    │       NavCom = NULL           NavCom = target       │
                    │       (idle or arrived)       (heading there)       │
                    │                                                     │
                    ▼                                                     ▼
              ┌──────────┐                                        ┌────────────┐
              │  IDLE    │                                        │  TRAVEL    │
              │  NavCom=0│                                        │  NavCom=T  │
              └──────────┘                                        └────────────┘
                    │                                                     │
                    │  TechnoClass::Set_Destination(T, 1)                 │
                    │  → Set_Destination_Internal                         │
                    │    → NavCom = T                                     │
                    │    → Locomotor.Head_To_Coord(T.DockCoord)           │
                    └──────────────────────────────────────────────────── │
                                                                          │
                                        ┌─────────────────────────────────┘
                                        │
                                        │  DriveLocomotionClass::Process
                                        │  detects our_cell == T.cell
                                        │
                                        ├── NavQueue.Count2 == 0:
                                        │   Set_Destination(NULL, 1)
                                        │   → Stop_Moving()
                                        │   → NavCom = 0
                                        │   → (Mission_Move sees it → OnArrival
                                        │      → SetSpeedFraction(0.0))
                                        │   => IDLE
                                        │
                                        └── NavQueue.Count2 > 0:
                                            Stop_Moving()
                                            → NavCom = 0
                                            OnArrival(0, 1)
                                            → NavCom = NavQueue.Buffer[0]  (re-issues via Set_Destination)
                                            → NavQueue shift-left
                                            => TRAVEL (new target)


    Side transitions:
    ─────────────────────────────────────────────────────────────────────

    Target dies:
      ObjectClass::Destructor → PointerExpired(expired, param_3)
        → if NavCom == expired and not sensor-retained and not strict infantry Occupier retention:
            NavCom = 0                               TRAVEL → IDLE
        → SuspendedNavCom cleared unconditionally
        → NavQueue scrubbed via shift-left

    Aircraft mission override (e.g., Retreat → Attack):
      AircraftClass::Set_NavCom_Override(new_mission, target)
        → Set_NavCom_With_Suspend(new_mission, ...)
          → SuspendedNavCom = NavCom (save)
          → ArchiveTarget = Target (save)
          → MissionClass::Override_Mission
          → Set_Destination(new_destination, 1)
            → Set_Destination_Internal(new_destination)
              → NavCom = new_destination      TRAVEL(A) → TRAVEL(B), SuspendedNavCom = A

    Player clicks empty space (or stop button):
      TechnoClass::Set_Destination(NULL, 1)
        → null-target branch
        → Stop_Moving()
          → NavCom = 0                               TRAVEL → IDLE

    Locomotor fully idle:
      DriveLocomotionClass::Process end-of-tick
        → DestCurrent == Null && Target == -1
        → SetSpeedFraction(0.0)                      (idempotent in IDLE)
```

---

## 9. INI Keys Involved

| Key | Section | Default | Effect on NavCom lifecycle |
|-----|---------|---------|----------------------------|
| `BlockagePathDelay` | `[General]` | 60 | Written to `FootClass+0x670` on every `Set_Destination_Internal` call. Frames to wait before retrying path when blocked. Verified at `RulesClass+0x1768`. |
| `PathDelay` | `[General]` | .01 | Walker-specific retry interval (stored at `RulesClass+0x1760` as double, minute-fraction). Used by the Walker CLSID branch in `Set_Destination_Internal`. |
| `CloseEnough` | `[General]` | 2.25 | Arrival tolerance for some approach-target cases. Consumed elsewhere in pathfinding, not in the NavCom setter directly. |
| `Teleporter` / `Locomotor` | unit section | per-unit | Drives the piggyback locomotor swap logic in `TechnoClass::Set_Destination`. |
| `Passengers` | unit/building section | per-type | `TypeClass+0x16AE` byte. Gates the `EnterQueue` append path in §3.5. |

No NavCom-specific INI key exists. The behavior is coded, not data-driven.

---

## 10. Current Rust Implementation Status

From the Mission_Move investigation (cross-referenced):

| System | Rust status |
|--------|-------------|
| `NavCom` pointer | Now represented by `NavigationState::nav_com` in `src/sim/components.rs`; older `MovementTarget`-only status is stale. |
| `NavCom_Aux` | Now represented by `NavigationState::nav_com_aux`; exact guard/drop side effects still need implementation parity. |
| `SuspendedNavCom` | Now represented by `NavigationState::suspended_nav_com`; PointerExpired cleanup/retention is still missing. |
| NavQueue | Now represented by `NavigationState::nav_queue`; standard runtime command appends are not supported by the newer producer audit. |
| `Set_Destination` as a single entry point | Partial: [movement::issue_move_command_with_layered](../../ra2-rust-game/src/sim/movement/movement_commands.rs#L1) is the rough equivalent, but it lacks type-specific dispatch (enter-building, helipad re-route, piggyback swap) |
| Null-target handling (Stop_Moving) | Partial: [movement_tick.rs:937](../../ra2-rust-game/src/sim/movement/movement_tick.rs#L937) clears `movement_target` on path exhaustion |
| Silent-drop guards (`+0x6AD` deploy/piggyback active / `+0x82` / `+0x2E4`) | Partially modeled but not exact; Rust must clear aux before guarded non-null drops and preserve old NavCom/Drive destination when the binary would silently return |
| Locomotor arrival signalling (`Process` calls `Set_Destination(NULL)`) | Not applicable (no separate locomotor object) |
| `SetSpeedFraction(0.0)` brake-to-zero | Not applicable (no gradient speed; movement is step-based) |
| PointerExpired NavCom cleanup | Not applicable yet (no persistent pointer references) |

Structural delta to port the lifecycle authentically:
1. Introduce a `navcom: Option<EntityRef>` field on ground-unit entities (or a `MoveTarget` with RTTI-tagged variants).
2. Add an `advance_navigation` step between locomotor tick and mission tick that:
   - detects cell-match arrival,
   - splits into "empty queue → clear" vs "queued → rotate" paths,
   - calls a mission-level `on_arrival` hook.
3. Add the silent-drop guards on the issue-move path (unit in deploy/limbo/warp states).
4. Hook `PointerExpired`-equivalent cleanup when entities are despawned (any unit with navcom pointing at the dead one gets cleared, subject to sensor-retention rule).

Each of these is a ~dozen-line change if the struct layout is ready; the hard part is the mission state machine that coordinates them.

---

## 11. Open Questions

1. **[RESOLVED 2026-05-27] NavQueue push producers** (§6.4). `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` found no standard runtime player, TeamClass/AI, or trigger waypoint producer; only save-load reconstruction positively populates nonzero entries.
2. **[RESOLVED 2026-05-27] `FootClass+0x6AC`**. `FOOTCLASS_SET_DESTINATION_GUARD_RECONCILIATION_GHIDRA_REPORT.md` verifies this is a one-shot `skip_head_to_coord_once` byte: Set_Destination_Internal writes NavCom, clears `+0x6AC`, and skips locomotor `Head_To_Coord` for that call.
3. **`FootClass+0x294`** — the "busy" flag checked by `AircraftClass::Set_NavCom_Override`. Not the same as InLimbo or IsDeploying. Aircraft-specific.
4. **`TypeClass+0xC94`** — gates the `DriveLocomotionClass::Stop_Moving` follower-stop loop (vehicles pulling trailers?). Possibly `HasTrailer` or `IsTrain`. Not traced.
5. **[RESOLVED 2026-05-27] The `PointerExpired` infantry retention branch**. `NAVCOM_POINTEREXPIRED_RETENTION_BRANCHES_GHIDRA_REPORT.md` verifies this is not a generic capture shortcut: it retains NavCom only under mission `8`, InfantryClass, `InfantryType+0xEB4` (`Occupier`), non-null NavCom, target flag bit `0x02`, target active byte true, positive target health, and target mission not `0x13`.
6. **[RESOLVED 2026-05-27] `TypeClass+0xD6A`, `+0xD28`, `+0xD29`, `+0xD2A`, `+0xD2C`**. `+0xD6A` is `BalloonHover=`, `+0xD2C` is derived `MovementZone == Subterannean`, and `+0xD28/+0xD29/+0xD2A` are the crush fields. The old stop-guard wording was a decompiler indexing mistake; the stop guard reads Unit/Foot instance bytes `+0x6E0/+0x6E1/+0x6E2`.
7. **[RESOLVED 2026-07-19] `InfantryClass`/`AircraftClass` leaf vtable+0x480 slots** (`0x0051AA40` / `0x0041AA80`, previously "needs_reinvestigate"/UNVERIFIABLE in §1 and §3.8). Re-verified this session: `0x0051AA40` is `InfantryClass::Set_Destination` (sole DATA xref `0x007EB4D8` via `get_xrefs_to`; vtable base `0x007EB058` written only by `InfantryClass::Constructor`/`Destructor`/`Load`). `0x0041AA80` is `AircraftClass::Set_Destination` (sole DATA xref `0x007E2724`; vtable base `0x007E22A4` written only by `AircraftClass::Constructor`/`Destructor`/`Load`) — the old `UnitClass::EnterBuildingOrDock` label was wrong. `get_function_callers 0x004D94B0` confirms exactly three callers: `AircraftClass__Set_Destination @ 0041aa80`, `InfantryClass__Set_Destination @ 0051aa40`, `TechnoClass__Set_Destination @ 00741970`. Canonical Ghidra already carries these renames plus plate comments from `FOOT_VTABLE_0X480_LEAF_SLOTS_GHIDRA_REPORT.md` (2026-07-13 re-swarm); this session independently re-derived the same facts from raw xrefs/decompiles rather than trusting the labels. See §1, §3.8, §7.1.
8. **[NEW 2026-07-19, OPEN]** With the RTTI fix in §4.2 (`What_Am_I() == 2` = `AircraftClass`, not `UnitClass`), the STEP 5 "don't stop the locomotor while attacking" carve-out in `Set_Destination_Internal` actually applies to attacking **aircraft**, not ground vehicles. This session only corrected the class attribution; it did not re-derive the resulting player-visible behavior for aircraft (what `CurrentMission == 1`/`QueuedMission == 1` and `TarCom != NULL` mean in an aircraft attack-approach context, or whether ground `UnitClass` has its own separate carve-out elsewhere that was conflated with this one). Needs a dedicated re-investigation pass.

---

## 12. Sources

**Ghidra addresses fully decompiled this investigation:**
- `0x00741970` — `TechnoClass::Set_Destination` (~500 lines)
- `0x004D94B0` — `FootClass::Set_Destination_Internal`
- `0x004D8F40` — `FootClass::Set_NavCom_With_Suspend`
- `0x0070 13A0` — `TechnoClass::Override_Mission_And_Target` (helper)
- `0x004DF0D0` — `FootClass::Stop_Moving`
- `0x004D9960` — `FootClass::PointerExpired`
- `0x004D82B0` — `FootClass::OnArrival` (re-read)
- `0x004B0500` — `DriveLocomotionClass::Process`
- `0x004AFD40` — `DriveLocomotionClass::Set_Destination` (= Head_To_Coord impl)
- `0x004AFE00` — `DriveLocomotionClass::Stop_Moving` (locomotor-level clear)
- `0x0041BB30` — `AircraftClass::Set_NavCom_Override`
- `0x004D3710` — `TechnoClass::SetSpeedFraction` (resolved vtable +0x544 from memory read at `0x007E91D8`)
- `0x0041AA80` — `AircraftClass::Set_Destination` (vtable+0x480 leaf slot; re-verified 2026-07-19 via `get_xrefs_to`/`decompile_function`, see §1/§3.8/§11 item 7)
- `0x0051AA40` — `InfantryClass::Set_Destination` (vtable+0x480 leaf slot; re-verified 2026-07-19, same method)
- `0x00746e20` — `UnitClass::What_Am_I` (RTTI 1; re-verified 2026-07-19 via `decompile_function`, see §4.2)
- `0x00523340` — `InfantryClass::What_Am_I` (RTTI 0xF; re-verified 2026-07-19 via `decompile_function`, see §4.2)
- `0x0041C180` — AircraftClass's vtable+0x2C `What_Am_I` code (RTTI 2; re-verified 2026-07-19 via `read_memory`, raw `mov eax,2; ret`, see §4.2)

**Docs referenced:**
- `FOOTCLASS_ENTER_QUEUE_AND_NAVCOM_SYSTEM.md` — extended and corrected (see §6)
- `FOOTCLASS_VTABLE_COMPLETE.md` — confirmed vtable +0x480 (Set_Destination), +0x484 (OnArrival), +0x4A4..+0x4D0 (TarCom family)
- `TECHNOCLASS_VTABLE_COMPLETE.md` — confirmed +0x3C8 = `Set_ArchiveTarget`
- `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` — confirmed `RulesClass+0x1768 = BlockagePathDelay`
- `DRIVE_LOCOMOTION_CLASS.md` — confirmed `RulesClass+0x1760 = PathDelay`
- `FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md` (this series) — the sibling report on the monitor side

**Global memory cited:**
- FootClass primary vtable at `0x007E8C94`; +0x544 resolves to `0x004D3710` (SetSpeedFraction), verified by raw read at `0x007E91D8` = `10 37 4D 00` (little-endian).
- `g_NullCoord_Drive_X/Y/Z` — the "inactive" coord marker for DriveLocomotion state.
- `g_BridgeZOffset_Drive` — vertical offset added for bridge cells in Head_To_Coord.
