# Chrono Miner Locomotion — System Synthesis

**System tag:** `chronominer-locomotion`
**Decoded:** 2026-05-24 via `/decode-system`
**Inputs:** 25 PROOFED per-symbol decodes; 92 parity rows in [_parity.md](_parity.md)

---

## Summary

The chrono miner's locomotion is the **TeleportLocomotionClass** at `0x00718000` — a generic COM-style locomotor for any unit with `Teleporter=yes` in its [UnitType] section. It implements three COM interfaces (IUnknown / ILocomotion / IPiggyback) and runs an **8-state warp machine** keyed on `TeleportLocomotionClass+0x34`. The chrono miner consumes it via the IPiggyback path: its base locomotor is `Drive`, and when the mission code initiates a warp, the engine pushes a TeleportLocomotionClass on top via `Begin_Piggyback`. When the warp completes, `End_Piggyback` restores Drive.

What the player observes — the 60-tick depart-cell shimmer, the WarpOut/WarpIn sound pair, the destination-cell flash, the post-arrival translucency, and the chrono miner's distinctive "instant warp" (no shimmer, no delay) — is produced by this 8-state machine plus a small set of `[General]` Rules keys (`ChronoDelay`, `ChronoTrigger`, `ChronoDistanceFactor`, `ChronoMinimumDelay`, `ChronoRangeMinimum`) and a per-`UnitType` Teleporter flag (`TechnoType+0xe0e`) that gates the chrono-miner-specific instant-warp branch. A separate `ChronoHarvTooFarDistance` key (Rules+0xD7C, stock 50) caps the miner's max teleport range — checked by the miner mission state machine, not by the locomotor itself.

Per the [feedback_chrono_teleport_direction] and [feedback_chrono_miner_no_arrival_shimmer] memories: the chrono miner warps **inbound only** (ore→refinery) and the WarpOut shimmer fires at the **depart** cell only. These are emergent properties of the miner's mission state machine selecting `TeleportLocomotionClass` only for the return-to-refinery leg — the locomotor itself is direction-agnostic.

---

## Symbol scope

| Kind | Count | Symbols |
|---|---|---|
| Functions (core) | 11 | Constructor, Process, StateMachineTick, InitiateWarp, Update_Position, PostWarpValidation, Phase0_SetWarpingOut, ClearPendingWarpPhase, TimerCheck, accessor-bundle (Is_Moving/Destination/HeadToCoord/Stop_Moving/Mark_All_Occupation_Bits), com-stub-bundle (QI/Begin_Piggyback/End_Piggyback/Is_Ok_To_End/ILocomotion_QI_Thunk) |
| Functions (Phase-0 added) | 5 | WarpAttachClass__Detach, HouseClass__HasPowerSurplus, CellClass__HasBridgeOverlay, FUN_0070f770, TechnoClass__Passive_Target_Acquire |
| Struct | 1 | TeleportLocomotionClass (0x4c bytes; 3 vtables; 2 dest caches; state byte; timer fields) |
| Globals | 2 | `g_NullCoord_Teleport_X/Y/Z` (0x00b0ebf8/fc/00 — all zeros), `g_BridgeZOffset_Teleport` (0x00b0ec2c — 0 in stock YR) |
| Strings (INI keys) | 6 | Teleporter (UnitType bool), ChronoDelay, ChronoTrigger, ChronoDistanceFactor, ChronoMinimumDelay, ChronoRangeMinimum, ChronoHarvTooFarDistance, WarpIn, WarpAway, Warpable |
| Enum | 1 | TeleportLocomotionClass state-machine values 0..7 |

**Total: 26 symbols.** Initial scope of 17 doubled by scope-explorer Phase 0 to 26 (within the 34 ceiling). All decodes PROOFED at confidence ≥ 90 (PROOFED-YELLOW absent — every proof passed clean).

**TS-legacy excluded by filter (9 symbols):** WarpAttachClass__UpdateAttack, WarpAttachClass__SpawnWarpAnims, TemporalClass__AI (ChronoSphere temporal-weapon path, separate system), plus 6 general infrastructure helpers (`MapClass__Get_CellClass`, `AnimClass__Constructor`, `VocClass__PlayAt`, `CrateClass__PickupDispatch`, `FUN_006b0ae0`, `STR_ChronoReinfDelay`).

---

## Control flow — how a teleport happens

```
                ┌───────────────────────────────┐
   Player /     │   Miner mission state machine │      (out of scope —
   mission AI   │   in src/sim/miner/, decides  │       lives in /miner/)
                │   when to call HeadToCoord    │
                └───────────────┬───────────────┘
                                │ HeadToCoord(dest)
                ┌───────────────▼───────────────┐
                │   ILocomotion::HeadToCoord    │   (vtable slot +0x0C / +0x44)
                │     0x00718100                │
                │   gate flags + arm cache +0x1C│
                └───────────────┬───────────────┘
                                │  per-tick
                ┌───────────────▼───────────────┐
                │   ILocomotion::Process        │   (per-tick dispatch)
                │     0x00718b70                │
                │   3 paths:                    │
                │     A. no-dest (sentinel)     │
                │     B. drive (mission ≠ 0xf)  │
                │     C. warp (mission == 0xf,  │
                │        radio-state 1/2/6 gate)│
                └───────────────┬───────────────┘
                                │  when warp commits
                ┌───────────────▼───────────────┐
                │  ILocomotion::StateMachineTick│   (8-state warp machine)
                │     0x007192f0                │
                │  state byte at +0x34          │
                └───────────────┬───────────────┘
                                │  state transitions below
              (states 0..7 — see "State machine" section)
                                │  on completion
                ┌───────────────▼───────────────┐
                │  IPiggyback::Is_Ok_To_End     │   true when state==0 and
                │     0x00719f30                │   ChronoInTransit==0
                └───────────────┬───────────────┘
                                │  yes
                ┌───────────────▼───────────────┐
                │  IPiggyback::End_Piggyback    │   restores Drive locomotor,
                │     0x00719ee0                │   clears kill-credit ptrs
                └───────────────────────────────┘
```

The dispatch above is the framework view. Inside `Process` Path C, **the function calls `StateMachineTick` via the same ILocomotion vtable** (slot 0x40 in the ILocomotion vtable, resolved to `0x007192f0` per the constructor's vtable plant — verified via `read_memory 0x007f5040`). `Process` and `StateMachineTick` are both per-tick entry points; which one fires depends on the locomotor's current state and the unit's mission.

---

## State machine — the 8-state warp pipeline

State byte lives at `TeleportLocomotionClass+0x34` (verified — index 0xd of an `int*` is byte offset 0x34). Initialized to 0 by Constructor.

| State | Name (this doc) | What happens | What the player sees |
|---|---|---|---|
| **0** | **Idle / WarpOut warm-up** | Three sub-paths gated at entry: (a) WarpingOut+0x271 set → dispatch TimerCheck and return (re-entry guard); (b) WarpState+0x280 non-zero → fast-forward to that state (external warp); (c) ChronoInTransit+0x270 set → sets BeingWarped+0x270=1, arms a 60-tick timer (`Locomotor+0x40 = 0x3c`), advances to state 1. Otherwise falls into the inline InitiateWarp clone — spawns WarpOut anim at depart cell, plays WarpOut sound, sets WarpingOut+0x271=1, detaches WarpAttachClass, retargets bullets, computes delay. | Unit visible at depart cell for up to 60 ticks, shimmering with the WarpOut anim. **Harvester short-circuit** (TechnoType+0xe0e set AND GetTypeID==1): forces `Locomotor+0x40 = 0`, clears WarpingOut+0x271 — chrono miner skips the shimmer entirely. |
| **1** | **Wait for warm-up timer** | Dispatches `param_1[-1]+0x28` (ILocomotion TimerCheck vtable slot, resolves to `0x00719bf0`). No state writes. TimerCheck advances to state 2 when `g_CurrentFrameCounter - Locomotor+0x3c >= Locomotor+0x40`. | Unit still at depart cell, shimmering, until timer expires. |
| **2** | **Teleport** | Spawns WarpIn anim at depart cell (TechnoClass+0x9c, **still source coords**), plays WarpIn sound at destination coords, sets WarpingOut+0x271=1, clears ChronoInTransit+0x27c and +0x270, clears bridge flag +0x8c, then calls `Update_Position(dest_x, dest_y)` — the actual teleport. Advances to state 3 (or 4 if Update_Position returns true). | Unit disappears from depart cell; arrival shimmer not yet. |
| **3** | **Move to dest** | Calls Update_Position again (mostly a no-op now), writes `TechnoClass+0x284 = Rules+0xbec` (ChronoDelay, stock 60) unconditionally each tick. Advances to state 4 when destination reached. | Brief tick — unit appears at destination. |
| **4** | **Final placement** | Calls `vtable+0x1b4` (Mark occupation), `vtable+0x1cc` (Place), `vtable+0x124` (Unlimbo). Always advances. | Unit committed to occupancy grid at destination. |
| **5** | **PostWarpValidation + arm ChronoDelay** | If WarpState+0x280==0: calls `PostWarpValidation(dest)` — **kills the unit if destination is water, impassable + bridge overlay, or otherwise invalid**. Aircraft with `Powered=yes` and no power surplus also die. Otherwise: clears kill-credit ptrs +0x428/+0x42c, arms timer from `TechnoClass+0x284` (ChronoDelay ticks), spawns WarpIn anim at **arrival** coords (TechnoClass+0x9c now equals destination). | Arrival shimmer at destination. Unit translucent / "being warped" for ChronoDelay ticks. |
| **6** | **Wait for ChronoDelay timer** | Same dispatch as state 1 — TimerCheck via vtable. Stays until timer expires. | Unit fully visible at destination but translucent. |
| **7** | **Cleanup + reset** | Clears WarpingOut+0x271, clears `TechnoClass+0x280`, clears `Locomotor+0x30` (cached Z), resets state byte to 0. | Unit fully materialized; `Is_Ok_To_End` returns true, mission can pop the piggyback. |

**Two visible-output gotchas verified in the decode:**
1. The state-2 "WarpIn anim at depart" and state-5 "WarpIn anim at arrival" both spawn `AnimClass(Rules+0x33c, ...)` — but state-2 fires before `Update_Position` so `TechnoClass+0x9c` is still source, while state-5 fires after teleport so `+0x9c` is destination. Two anims at two cells from the same AnimType pointer.
2. The InitiateWarp inline clone (state-0 path) spawns the WarpOut shimmer at source and a second persistent anim at destination with constructor args `(type, coords, 0, 1, 0x600, 0, 0)` — the `0x600` flag is the persistent-anim selector. Same AnimType, different lifetime — the depart shimmer is a flash, the arrival is a persistent indicator. This reconciles `[feedback_chrono_miner_no_arrival_shimmer]`: the **WarpOut SHP flash** plays at depart only; the persistent arrival anim is a different beast.

---

## INI surface

### `[General]` keys consumed by the locomotor

All verified from `decompile_function 0x00719400` (InitiateWarp) and `decompile_function 0x007192f0` (StateMachineTick state 3), with stock values grep'd from `ini/rulesmd.ini` lines 221–294.

| Key | Rules offset | Type | Stock | Role |
|---|---|---|---|---|
| `ChronoDelay` | `+0xBEC` | int (frames) | 60 | Post-arrival translucent hold. Written to `TechnoClass+0x284` in state 3, consumed by the state-5 timer arm. |
| `ChronoTrigger` | `+0xBF8` | bool | yes (1) | Master gate for distance-based delay. When false, all warps use ChronoMinimumDelay regardless of distance. |
| `ChronoDistanceFactor` | `+0xBF4` | int | 48 | Divisor: `delay_ticks = distance_leptons / 48`. |
| `ChronoMinimumDelay` | `+0xBFC` | int (frames) | 16 | Floor: computed delay clamped to `max(delay, ChronoMinimumDelay)`. |
| `ChronoRangeMinimum` | `+0xC00` | int (leptons) | 0 | Distance threshold; if `distance < ChronoRangeMinimum`, delay overridden to ChronoMinimumDelay. Dead in stock YR (value=0). |
| `ChronoHarvTooFarDistance` | `+0xD7C` | int (cells, YELLOW) | 50 | Chrono miner max warp range. Checked by miner mission state machine **before** calling HeadToCoord — out of scope for the locomotor itself. |
| `WarpIn` | (Rules+0x33c-ish, YELLOW) | AnimType | (anim id) | AnimType used for both depart-shimmer and arrival-persistent anim. |
| `WarpAway` | adjacent | AnimType | (anim id) | Departure-anim variant (YELLOW — exact consumer not traced; may be unused in stock YR). |

### `[UnitType]` keys consumed by the locomotor

| Key | TechnoType offset | Type | Stock units with `=yes` | Role |
|---|---|---|---|---|
| `Teleporter` | `+0xE0E` | bool | 4 (CLEG, CMIN, possibly others — confirmed at `ini/rulesmd.ini` lines 4141, 4210, 4707, 7396) | Two effects: (1) gates the choice of TeleportLocomotionClass for this unit (either as base loco or as a piggyback override over Drive); (2) gates the chrono harvester **instant-warp short-circuit** in InitiateWarp/StateMachineTick state-0 — forces `being_warped_ticks=0` when set AND `GetTypeID()==1` (infantry). |
| `Warpable` | (TechnoType+0xD3A, YELLOW) | bool | (varies) | Per-unit ChronoSphere targettability flag — checked by ChronoSphere/temporal-weapon path, not by the locomotor itself. Out of scope. |

### Sound INI keys

`TechnoType+0x574` (WarpIn sound index) and `+0x578` (WarpOut sound index) override the global `Rules+0x218` (WarpIn fallback) and `Rules+0x21c` (WarpOut fallback). Played via `VocClass__PlayAt` at the depart cell (WarpOut) and destination cell (WarpIn) — both endpoints, confirmed in InitiateWarp steps 11 and StateMachineTick state 2/5. This matches the project memory "sounds fire at both endpoints."

---

## Observable behaviors (player-visible)

Ordered by frequency × visibility. Each is the observable contract the Rust port must reproduce.

1. **60-tick depart-cell shimmer** before any non-harvester teleport. Unit visible, with WarpOut anim playing, for 60 ticks (~2s at 30fps) before disappearing.
2. **Instant warp for chrono miner.** When `Teleporter=yes` + infantry-type test passes, the 60-tick warm-up is skipped — unit disappears in 1 tick.
3. **WarpOut SHP flash at depart cell.** Single-shot anim spawned at source coords during state 0 / InitiateWarp.
4. **WarpIn arrival anim at destination cell.** Persistent anim (`0x600` flag) spawned at destination after teleport completes (state 5).
5. **WarpOut sound at depart, WarpIn sound at arrival.** Two distinct sounds, both endpoints. Per-unit overrides via `TechnoType+0x574/+0x578`; global fallback via `Rules+0x218/+0x21c`.
6. **Post-arrival translucency for ChronoDelay ticks.** Unit visible at destination but rendered translucent until `TechnoClass+0x271` (WarpingOut flag) clears in state 7.
7. **Distance-proportional delay.** `delay = distance_leptons / 48`, clamped to a floor of 16 ticks (`ChronoMinimumDelay`).
8. **Death on water / impassable / blocked bridge arrival.** PostWarpValidation (state 5) sets `TechnoClass+0x3cd=1`, calls `Die` vtable, attributes kill credit via source-building/source-house ptrs.
9. **Aircraft survival exemption with power.** Aircraft (`TechnoType+0x67c==3`) bypass water-death — unless `Powered=yes` and the owning house has no power surplus.
10. **Occupant displacement at arrival.** PostWarpValidation walks `CellClass+0xe4` ground-occupant list and calls `vtable+0x16c` (warp displacement) on each warpable occupant.
11. **Bullet retargeting on warp.** State 0 / InitiateWarp scans `g_BulletClass_Array` and calls `BulletClass__UpdateTarget` for each bullet aimed at the warping unit.
12. **WarpAttachClass detach.** State 0 / InitiateWarp calls `WarpAttachClass__Detach` if `TechnoClass+0x694` is non-null — releases ChronoSphere beam linkage before teleport.
13. **Chrono miner mission gate (`ChronoHarvTooFarDistance`).** Miner mission code (out of scope for this system) rejects warp destinations beyond 50 cells — miner drives normally instead.

---

## Edge cases / parity hazards

Detail-level concerns that compound into observable drift if missed.

1. **POINTER-ARITHMETIC trap.** `param_1` in StateMachineTick is `int*`, so `param_1[0xd]` = byte offset `0xd × 4 = 0x34` (state byte). But in PostWarpValidation `param_1` is `int`, so offsets are direct bytes. Conflating these gives wrong field accesses. Verified in struct-decode.

2. **Two destination caches (+0x1C and +0x28).** `HeadToCoord` writes to `+0x1C..+0x24` (dest-cache-0); `Process` writes to `+0x28..+0x30` (dest-cache-1, the validated output). Both initialized to the `g_NullCoord` sentinel. Rust collapses both into a single `(target_rx, target_ry)` — losing the in-validation vs validated distinction.

3. **Sentinel address corrected.** `g_NullCoord_Teleport_X` is at `0x00b0ebf8`, not `0x00b0ebd8` as initially decoded. The labeler caught the discrepancy by cross-referencing xrefs — the first-pass decoder cited zeros at `0x00b0ebd8` and didn't notice the address didn't appear in Constructor's callsites. The corrected address has 13 xrefs across all the right functions. Lesson: cite-the-address ≠ confirm-the-binding — must also check xref pattern matches behavioral description.

4. **State byte vs index 0xd ambiguity.** `param_1[0xd]` (int* index) = byte offset `0x34`. Decompiler writes `(undefined1*)(param_1+0xd) = 0` for the byte write — that's an undef1 cast on the int* pointer, NOT pointer arithmetic. Both paths reach byte 0x34. Easy to mis-port.

5. **Coord frame: dest fields are leptons (NW-cell), source `+0x288/+0x28C/+0x290` are TechnoClass.** The locomotor's own dest cache (+0x1C, +0x28) is in lepton-NW-cell-frame; the TechnoClass mirror at +0x288 is the same. State 2's `Update_Position(TechnoClass+0x288)` reads from TechnoClass, not from the locomotor's cache. Mixing the two reads frames is a coord bug.

6. **Bridge-Z lift.** `g_BridgeZOffset_Teleport` (0x00b0ec2c) is 0 in stock YR — added to dest Z when teleporting onto a bridge cell. Even with value=0 the `TechnoClass+0x8c` (bridge flag) write is load-bearing for renderer Z-ordering; Rust skipping the flag write produces visible Z-order errors on bridge cells.

7. **Mission==0xf gate on warp dispatch.** Process only commits a warp when `TechnoClass.GetMission() == 0xf` (MISSION_ENTER). Other missions use the drive path. Rust currently warps regardless of mission state — player can observe warps fired during ATTACK or GUARD missions, which gamemd would reject.

8. **Radio-state gate (1, 2, 6) inside MISSION_ENTER.** Within the warp dispatch (LAB_00718ce9), Process verifies the radio link target's mission state against {1, 2, 6} and confirms cell match. Only then commits the dock teleport. Without this gate, the chrono miner can teleport into the wrong refinery or into occupied cells.

9. **State-2/State-5 anim coord ambiguity.** The "spawn anim at TechnoClass+0x9c" line appears in both states but means different cells — state-2 reads source coords (anim fires before teleport), state-5 reads destination (after teleport). Mis-reading this gives both anims at the same cell.

10. **Harvester branch identity.** The instant-warp short-circuit checks `GetTypeID() == 1` (infantry type ID = 1, YELLOW per `feedback_research_confidence_axes`) AND `TechnoType+0xE0E != 0` (Teleporter bool). Both required. A non-infantry teleporter (no current stock unit matches) would NOT instant-warp. The chrono miner satisfies both because CMIN is `Harvester=yes` but the actual type-ID test passes for it as an infantry-tier vehicle in gamemd's RTTI hierarchy (YELLOW — exact RTTI placement unverified).

---

## Parity verdict — Rust port vs gamemd

The Rust port at [src/sim/movement/teleport_movement.rs](../../src/sim/movement/teleport_movement.rs) collapses the 8-state machine into a **2-phase** model (`Relocate`, `ChronoDelay`). This is a major internal redesign — and per the CLAUDE.md parity bar, the default verdict for any mechanism difference is **DRIFT** unless byte-equivalent observable output is proven.

The [_parity.md](_parity.md) file records **92 disparity rows**. Highlights, ranked by visibility × frequency:

### CRITICAL (fires every teleport, player-visible immediately)

- **PostWarpValidation entirely absent.** Units survive teleport to water, impassable cells, or blocked bridges. Chrono Legionnaire and chrono miner both unable to die on bad landings. ([parity row "PostWarpValidation / function entirely absent"](_parity.md))
- **60-tick depart warm-up missing.** Non-harvester teleports look instant in Rust; gamemd shows ~2s of WarpOut shimmer at depart cell first.
- **All anim and sound emission missing.** Rust has no WarpOut SHP, no WarpIn arrival anim, no departure/arrival sound (the latter correctly because `sim/` cannot call `audio/`, but the events must still be emitted to the renderer/audio layer).
- **WarpingOut flag (`TechnoClass+0x271`) never set.** Any system reading this flag (damage immunity during warp, render translucency, post-warp targeting wake-up) sees stale state.
- **BulletClass retargeting missing.** Bullets aimed at a warping unit don't get redirected — projectiles track the unit's old position and bend into the void.
- **WarpAttachClass detach missing.** ChronoSphere beam links persist through teleport.

### HIGH (fires frequently, visible drift)

- **8-state → 2-phase collapse.** Every state 0-7 has at least one observable side-effect missing in Rust (24 DRIFT rows on StateMachineTick alone).
- **InitiateWarp delay formula has 3D-vs-2D mismatch + off-by-one clamp + missing elapsed-time adjustment + GetCoords-vs-cell-pos source.** Ticks of delay differ for units on bridges/cliffs, for large-foundation units, and for back-to-back warps.
- **ChronoDelay source.** gamemd re-reads `Rules+0xBEC` from state 3 each tick (live INI value); Rust caches `compute_chrono_delay` result at issue time. Diverges if Rules reloaded mid-warp AND when ChronoTrigger=false (gamemd uses flat Rules.ChronoDelay; Rust still runs distance formula).
- **Mission gate (==0xf) missing.** Rust warps regardless of unit mission state.
- **Radio-link dock gate missing.** Chrono miner can teleport to wrong refinery or occupied dock.
- **Bridge flag (`TechnoClass+0x8c`) never written.** Z-ordering errors on bridge cells.

### MEDIUM (specific scenarios)

- **Stop_Moving entirely absent.** No way to cancel a teleport once issued — affects damage-interrupt and player cancel commands.
- **Find_Nearby_Passable_Cell fallback missing.** Unit warps onto occupied/impassable cells with no relocation.
- **Sub-cell placement always CELL_CENTER.** Infantry stack at cell center instead of distributing to NE/NW/SE/SW sub-cells.
- **Ground-height Z update missing.** Z stays at `position.z` instead of being adjusted to destination terrain height.
- **End_Piggyback kill-credit ptr clear missing.** Source building/house ptrs at `TechnoClass+0x428/+0x42c` remain set after warp — wrong kill attribution on subsequent damage events.

### Internals where Rust's design is fine

Several gamemd internals (the COM vtable identity, the two-cache design, the `Locomotor+0x30` dual-use slot, the `RateTimer__Set` thunk inside `Mark_All_Occupation_Bits`) are clearly internal mechanism — Rust's flatter design is acceptable as long as the observable outputs match. **But all the parity rows above must be observable-output equivalent before any of these internals can be marked INTERNAL-ONLY.** Currently none are — every row in `_parity.md` is a DRIFT pending evidence.

---

## Per-symbol doc index

### Functions (core 11)
- [fn-constructor.md](fn-constructor.md) — `0x00718000` — 3 vtables + sentinel-init both dest caches
- [fn-process.md](fn-process.md) — `0x00718b70` — per-tick dispatch, 3 paths (no-dest / drive / warp)
- [fn-state-machine-tick.md](fn-state-machine-tick.md) — `0x007192f0` — 8-state warp machine
- [fn-initiate-warp.md](fn-initiate-warp.md) — `0x00719400` — delay formula + anim/sound + teleport + harvester short-circuit
- [fn-update-position.md](fn-update-position.md) — `0x00718260` — bridge Z lift + occupant collision + passable-cell fallback
- [fn-post-warp-validation.md](fn-post-warp-validation.md) — `0x007187a0` — water-death + bridge-blocked-death + aircraft-power exemption
- [fn-phase0-set-warping-out.md](fn-phase0-set-warping-out.md) — `0x007197d0` — 60-tick warm-up timer arm
- [fn-clear-pending-warp-phase.md](fn-clear-pending-warp-phase.md) — `0x00719790` — abort-anim + WarpState clear
- [fn-timer-check.md](fn-timer-check.md) — `0x00719bf0` — frame-stamp expiry check + Passive_Target_Acquire wake-up
- [fn-accessors.md](fn-accessors.md) — bundle for Is_Moving (`0x00718080`), Destination (`0x007180a0`), HeadToCoord (`0x00718100`), Stop_Moving (`0x00718230`), Mark_All_Occupation_Bits (`0x007192c0` — RateTimer stub, name misleading)
- [fn-com-stubs.md](fn-com-stubs.md) — bundle for QI (`0x00719e30`), Begin_Piggyback (`0x00719e90`), End_Piggyback (`0x00719ee0`), Is_Ok_To_End (`0x00719f30`), ILocomotion_QI_Thunk (`0x0071a160`)

### Functions (Phase-0 added)
- [fn-warp-attach-detach.md](fn-warp-attach-detach.md) — `0x0062a4a0` — direct InitiateWarp callee, WarpAttachClass cleanup
- [fn-house-has-power-surplus.md](fn-house-has-power-surplus.md) — `0x0050e1b0` — aircraft survival gate in PostWarpValidation
- [fn-cell-has-bridge-overlay.md](fn-cell-has-bridge-overlay.md) — `0x004865d0` — PostWarpValidation bridge fallback
- [fn-0070f770.md](fn-0070f770.md) — `0x0070f770` — TimerCheck targeting-wake helper (identity TBD per decode)
- [fn-techno-passive-target-acquire.md](fn-techno-passive-target-acquire.md) — `0x00709480` — post-warp auto-targeting

### Struct + Globals + Strings + Enum
- [struct-teleport-locomotion-class.md](struct-teleport-locomotion-class.md) — 0x4c-byte COM object
- [global-null-coord-teleport.md](global-null-coord-teleport.md) — `0x00b0ebf8/fc/00` sentinel triple (address-corrected)
- [global-bridge-z-offset-teleport.md](global-bridge-z-offset-teleport.md) — `0x00b0ec2c` (=0 stock)
- [string-teleporter.md](string-teleporter.md) — UnitType.Teleporter at `0x00843e60`
- [string-chrono-rules-keys.md](string-chrono-rules-keys.md) — 5 [General] keys bundle
- [string-warp-anim-keys.md](string-warp-anim-keys.md) — WarpIn / WarpAway
- [string-warpable.md](string-warpable.md) — TechnoType.Warpable
- [string-chrono-harv-too-far.md](string-chrono-harv-too-far.md) — Rules+0xD7C, stock 50, chrono-miner range cap
- [enum-state-machine-states.md](enum-state-machine-states.md) — 8 states

---

## References

All inline citations in per-symbol docs use the form `decompile_function 0x00XXXXXX`, `get_xrefs_to 0xXXXX`, `read_memory 0xXXXX`, `get_struct_layout NAME`, etc. — see individual docs for verification trail.

**Key addresses (top-level entry points only):**
- Constructor: `0x00718000`
- Process: `0x00718b70`
- StateMachineTick: `0x007192f0`
- InitiateWarp: `0x00719400` (inline-called from StateMachineTick state 0)
- PostWarpValidation: `0x007187a0`
- IUnknown vtable base: `0x007f50cc`
- ILocomotion vtable base: `0x007f5010`
- IPiggyback vtable base: `0x007f4fe0`

**Project memory entries that load-bearing for this synthesis:**
- `feedback_chrono_teleport_direction` — chrono miner warps inbound only; outbound is normal drive
- `feedback_chrono_miner_no_arrival_shimmer` — WarpOut SHP fires at depart cell only (reconciled here: the persistent arrival anim is a separate spawn with different constructor flags)
- `feedback_research_confidence_axes` — confidence-axis discipline applied throughout per-symbol docs
- `feedback_direction_bugs` — coord-frame annotations applied to every coord field

**Out of scope (intentionally deferred):**
- The miner mission state machine in [src/sim/miner/](../../src/sim/miner/) and `miner/` docs — that's the layer that decides *when* to call `HeadToCoord`. The locomotor decoded here doesn't care which direction or which mission; it just teleports.
- `ChronoSphere__WarpUnitsAtCell` (`0x0065ec30`) and `BuildingClass__DeployUnit_ChronoWarp` (`0x0070fee0`) — these are the ChronoSphere superweapon path, separate from unit-side TeleportLocomotionClass.
- `WarpAttachClass__UpdateAttack` / `__SpawnWarpAnims` / `TemporalClass__AI` — ChronoSphere temporal-weapon code path, TS-filter excluded.
- AnimClass / VocClass / CrateClass / MapClass internals — general infrastructure used by but not owned by the locomotor.

---

## Next steps (user's call)

1. **Rank the 92 DRIFT rows in `_parity.md` by visibility × frequency** and pick the first to fix. The CRITICAL list above is the obvious starting set.
2. **Feed `_system.md` to `/brainstorm`** for a design spec that decides how to model the 8-state machine in Rust — preserve the current 2-phase external API but expand the internal state space, or expose all 8 states.
3. **Run `/write-plan` directly** if you want the implementation broken into tasks before deciding the design.
4. **`/decode-system chronominer-locomotion --resume`** later if a new sub-area is identified during implementation (e.g., if AnimClass or VocClass integration becomes load-bearing).
