---
name: BuildingClass Mission_Guard and Mission_Construction
description: Short-but-important BuildingClass mission handlers not previously deep-dived. Mission_Guard is a trivial thunk; Mission_Construction is the build-up-animation state driver when mission enum 18 is assigned.
type: reference
---

# BuildingClass Mission_Guard + Mission_Construction — Ghidra Research Report

**Binary:** `gamemd.exe`
**Date:** 2026-04-24
**Confidence:** HIGH for both handler bodies and dispatch; MEDIUM/UNCHECKED for the ordinary factory-placement writer that assigns mission enum 18.
**Active in YR:** Yes (both handlers). MCV deploy reaches Mission_Construction through an explicit mission-18 assignment. The exact ordinary factory-placement assignment site remains unresolved; do not infer that `Unlimbo` or `HouseClass::Place_Production` performs it.

## Dispatch setup (shared)

BuildingClass mission handlers are dispatched by `MissionClass::Mission_Dispatch`
(`0x005B3060`). It reads the current mission enum from `this[0x2B]` (byte at
instance offset `0xAC`) and calls the vtable slot listed in the switch table:

| Mission enum | Vtable offset | BuildingClass slot | Handler address |
|---:|:---:|:---:|:---:|
| **8 (Guard)**, 17 (Harmless reuses) | `+0x214` | 133 | **`0x0044B760`** |
| **18 (Construction)** | `+0x244` | 145 | **`0x00449A50`** |

Both vtable pointers read directly from `BuildingClass` vtable at `0x007E3EBC`:

- `0x007E3EBC + 0x214 = 0x007E40D0 → 0x0044B760` ✓
- `0x007E3EBC + 0x244 = 0x007E4100 → 0x00449A50` ✓

---

## Part A — Mission_Guard (`0x0044B760`)

### 1. Thunk chain resolution

```
0x0044B760  thunk_FUN_005b2e50     JMP 0x005b2e50
0x005b2e50  FUN_005b2e50           MOV EAX, 0x1c2 ; RET
```

That is the whole function. There is **no real body**. It is bit-for-bit
identical to `MissionClass::Mission_Default` (`0x005B2E10`), which also does
`MOV EAX, 0x1c2 ; RET`. The only reason it exists as a separate function is
that the overriding-C++-class needed a distinct address for the vtable slot.

### 2. Behavior

Returns **`0x1C2` (= 450)**. This value is stored into
`MissionClass.Timer` (`this[0x34]` / `+0xD0`) by `Mission_Dispatch` and is the
number of game frames until the handler is called again — the canonical
"default mission sleep interval" used across every generic mission stub in
gamemd.exe. At 15 fps this is ~30 s.

### 3. Entry / exit

Mission_Guard has no state and never transitions itself. The mission stays
`GUARD` until some other code path calls `Queue_Mission(...)` on the building.
Common setters that move a building **into** GUARD (verified across sibling
docs):

- `Mission_Selling` state 0 on an already-sold upgrade → `Queue_Mission(GUARD)`
- `Mission_RepairAndProduce` Hospital/Armory/Helipad completion → `Queue_Mission(GUARD)`
- `Mission_Missile` post-fire state 4 → `Queue_Mission(GUARD)`
- End-of-construction (Mission_Construction state 1 completion path — below)

### 4. Side effects

**None.** No animation touched, no timer advanced, no target scan. This is
confirmed by the disassembly — the entire function is two instructions. The
slot exists only to satisfy the vtable.

### 5. Edge cases — Sentry Gun / Tesla Coil / Prism Tower

**These DO NOT run through Mission_Guard.** Combat-capable defenses (Tesla
Coil, Prism Tower, Sentry Gun, Grand Cannon, Patriot, Flak Cannon) are
assigned `MISSION_ATTACK` (enum 1) by the target-acquisition AI and run
through `BuildingClass::Mission_Attack` (`0x0044ACF0`), which is the actual
per-tick scan + fire machine. See
`BUILDINGCLASS_MISSION_ATTACK_AND_RESIDUALS.md` for the 11-entry jump table
and the `IsChargeMode` 3-state machine (Tesla/Prism charge cycle).

The existing `MISSION_GUARD_AREAGUARD_GHIDRA_REPORT.md` covers the
MissionClass-level generic Guard/AreaGuard distinction (auto-target-acquire
vs passive). For **buildings specifically**, Guard is passive-only — there is
no auto-acquire path here; acquire happens in `TechnoClass::AI` (the per-tick
target scan) which queues Mission_Attack when a target is spotted.

### 6. Tiberian Sun legacy note

The MissionClass base Mission_Guard body is also a `return 0x1c2` stub in
gamemd.exe. All real "guard" logic in YR has migrated into either
Mission_Attack (combat buildings) or `TechnoClass::AI` (target acquisition).
No dormant TS behavior identified in this slot.

---

## Part B — Mission_Construction (`0x00449A50`)

354 bytes, 97 instructions, 11 basic blocks, cyclomatic complexity 6.
This is a **2-state machine gated on `this[0x2F]`** (instance offset `0xBC`,
the shared MissionClass sub-state field).

### 7. Decompilation (condensed)

```c
int __fastcall BuildingClass::Mission_Construction(BuildingClass *this) {
    if (this->sub_state == 0) {            // this[0x2F] at +0xBC
        BuildingClass__GrandOpening(this, 0);    // start BuildUp anim (slot 0)
        this->vtable->slot_9D(this, 0xB);        // Receive_Radio(11 = BUILD_BEGIN)
        if (this->Type->BuildUpSound != -1 ||
            RulesClass::Instance->BuildUpSound != -1) {
            VocClass__PlayAt(&this->coord);      // [Type+0xE6C] BuildUpSound
        }
        this->field_0x80 = 1;              // redraw flag
        this->sub_state = 1;
        return 1;                          // re-enter next tick
    }
    else if (this->sub_state == 1) {
        this->field_0x80 = 1;              // redraw every tick during BuildUp
        AnimClass__UpdateLoopingSound();   // keep BuildUp audio alive
        if (this->ConstructionComplete) {  // this+0x6DD — set by anim finishing
            this->vtable->slot_9D(this, 0x0C);   // Receive_Radio(12 = BUILD_END)
            this->vtable->slot_9D(this, 0x03);   // Receive_Radio(3 = OVER_AND_OUT)
            BuildingClass__GrandOpening(this, 1);    // commit idle anim (slot 1)
            this->vtable->slot_13B();             // vtable+0x4DC — OnConstructionComplete
            this->vtable->AssignMission(5, 0);    // vtable+0x1E8 — Queue_Mission(Mission_Retreat → Guard)
            if (this->Type->IsWalkThrough == 0) {  // Type+0x16BF
                FacingClass__UpdateFacing(&facing);  // snap body facing to PrimaryFacing
            }
            FUN_00465af0(this->Type);            // Type+0x1762-gated one-shot cleanup (frees Type+0xE00)
            SoundEvent__Release(&this->sound);   // stop BuildUp looping sound
            return 1;
        }
    }
    return 1;                              // keep mission alive next tick
}
```

(`slot_9D` at vtable+0x274 = `Receive_Radio`; `vtable+0x1E8` = `AssignMission`;
`vtable+0x4DC` = `OnConstructionComplete`-equivalent hook — confirmed against
sibling `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md`.)

### 8. Entry — how Mission_Construction is initiated

The handler is selected when the current mission enum is **18 (`0x12`)**.
The entry writers are only partially resolved:

1. **MCV deploy** — the verified deploy body assigns/queues mission `0x12`
   at `0x007396D2..0x007396D9` **before** it calls the new construction
   yard's `Unlimbo`. This supersedes the earlier post-Unlimbo wording.
2. **Ordinary factory placement** — current reinspection found no immediate
   `0x12` mission assignment and no direct current/queued-mission-field write
   in `FactoryClass::StartProduction @ 0x004C9C70`,
   `FactoryClass::AI @ 0x004C9B20`,
   `FactoryClass::CompletedProduction @ 0x004CA1A0`,
   `HouseClass::Place_Production @ 0x004FB0E0`, or
   `BuildingClass::Unlimbo @ 0x00440580`. The actual writer/caller for the
   normal player and AI build paths is therefore **UNCHECKED**. In
   particular, the inspected body does not support the former claim that
   `HouseClass::Place_Production` assigns CONSTRUCTION after Unlimbo.
3. **Spawned pre-placed map buildings** — the prior scenario-loader claim
   was not re-audited in this correction. Treat its exact mission assignment
   as UNCHECKED unless cited by a dedicated scenario-load trace.

This entry-site uncertainty does not affect the handler's verified state
machine or factory-created health: `BuildingClass::Init_Managers @
0x00442C40` has already copied `Type.Strength` into `+0x6C/+0x70` before the
factory receives the object, and the inspected placement/construction bodies
do not overwrite those fields.

### 9. Sub-state field (`+0xBC`)

Shared across all BuildingClass mission state machines (Selling, Missile,
Bunker docking etc.). For Mission_Construction it holds only **0 or 1**:

- **0** — first tick only. Fire GrandOpening(0), radio BUILD_BEGIN, play
  BuildUpSound, request redraw, advance to 1.
- **1** — every subsequent tick until `+0x6DD` (ConstructionComplete) is 1.
  Then run completion ritual + Queue_Mission(GUARD) + bail.

There is **no timer at `+0x620` in this handler**. The `+0x620` timer
accumulator is used by `Mission_RepairAndProduce` (Hospital heal, Armory
veterancy, Repair Depot HP tick, ProduceCashTimer). Mission_Construction's
progression is driven entirely by the BuildUp **animation finishing** — the
anim system writes `+0x6DD = 1` when the BuildUp frame sequence hits its last
frame. This is the canonical YR handshake between the 21-slot anim system
and the mission dispatcher.

### 10. Animation — slot 0 (BuildUp) then slot 1 (idle base)

`GrandOpening(0)` binds `Type+0xF04 + 0*0xC = Type+0xF04` (entry 0 =
BuildUpAnim) and seeds the instance anim state at `+0x0F8..+0x110`.
`GrandOpening(1)` on completion binds `Type+0xF10` (entry 1 = idle/steady-
state overlay). The BuildingAnimControl table at `Type+0xF04` uses 0xC-byte
entries; entry format is `{ AnimTypeClass*, flags, ... }`.

### 11. Side effects — full list

Per tick while state 1:
- `+0x80` (redraw dirty flag) set to 1 each tick — keeps the sprite
  re-rasterised as the BuildUp frames advance.
- `AnimClass::UpdateLoopingSound` called — keeps the BuildUp loop alive.

On first entry (state 0 → 1):
- `GrandOpening(0)` — seed anim slot 0.
- `Receive_Radio(0xB = BUILD_BEGIN)` — notifies radio peers (used by MCV,
  factory).
- `VocClass::PlayAt(this.coord)` — plays **`BuildUpSound`** (Type+0xE6C fallback
  to `RulesClass+0x6C8`). Zero if both = -1.
- `+0xBC` → 1.

On completion (state 1 → exit):
- `Receive_Radio(0xC = BUILD_END)` then `Receive_Radio(0x3 = OVER_AND_OUT)`
  — closes the radio link used during building deploy.
- `GrandOpening(1)` — commit the idle base overlay.
- `OnConstructionComplete` (vtable+0x4DC) — this is the big hook that fires
  `[Rules] BuildupTime`-style side-effects: radar events, prerequisite
  unlocks, Factory linkage, ProduceCashTimer start, power recomputation,
  wall auto-connect, spotlight/ambient-light instantiation. Most of that
  logic was actually already seeded in `Unlimbo`, but `OnConstructionComplete`
  re-fires the ones that are gated on "building actually usable now".
- `Queue_Mission(5 = Mission_Retreat)` — and this is where the pipeline is
  subtle: enum 5 dispatches through BC vtable+0x21C, which for BuildingClass
  is `0x004496B0` ("Mission_Retreat" in vtable doc v1, relabeled as the
  "deployed-state driver" — sets `+0x6DD = 1` on Gattling, runs Refinery/
  Hospital/Armory idle checks, clears WeaponsFactory bib). The next mission
  transition (to true MISSION_GUARD enum 8) happens inside that handler's
  early-termination branches. Practical end state for a stock building
  one tick later: enum 8 Guard, which is the no-op Part A stub.
- `FacingClass::UpdateFacing` — snap primary facing to
  `Type+0xED8 PrimaryFacing` unless `Type+0x16BF IsWalkThrough` (walls) is
  set.
- `FUN_00465af0(Type)` — Type-level one-shot: if `Type+0x1762` is set and
  `Type+0xE00 != 0`, free the resource at `+0xE00` (appears to be a cached
  build-queue string pointer; confirmation deferred).
- `SoundEvent::Release(&this+0x4DC)` — stop the BuildUp sound loop.

Returns 1 on every tick (including the last), meaning `Mission_Dispatch`
re-invokes Mission_Construction at the next frame. The actual switch to
Guard is via `Queue_Mission`, which `Mission_Dispatch` will pick up next
tick (the Queue is consumed at the top of dispatch).

### 12. Edge cases

**Interrupted construction — low power:** Mission_Construction does **not**
power-gate. `HasPower` (`+0x660`) is NOT read. The BuildUp animation plays
regardless of the owner's power state, and `+0x6DD` is set by the anim
finishing, not by power. There is no pause/cancel here.

**Sell-during-build:** Mission_Construction does not check for Selling. But
the normal flow is: `HouseClass::Sell_Building_At_Cell` (called by
`Unlimbo`'s own pre-place sweep and by the sidebar sell button) calls
`Assign_Mission(SELLING)` synchronously, which overwrites enum 18 with
enum 19. Mission_Dispatch's switch then routes to `Mission_Selling` on the
next tick. **Result:** a half-built building sold during BuildUp skips to
Selling state 0 directly; it refunds the full `Cost × SellBack` (no
health-scaling — see `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` §refund-formula).
The BuildUp anim is abandoned mid-frame; `SoundEvent::Release` is called by
the Selling handler's state-0 init, so no orphan audio.

**MCV-redeploy of a ConYard:** The MCV deploy path creates a new
`BuildingClass`, assigns/queues mission `0x12`, then calls `Unlimbo`. The
ConYard's BuildUp anim plays normally. There is no "skip build-up" flag for
redeployed structures in this verified path. Evidence: mission assignment at
`0x007396D2..0x007396D9` precedes the Unlimbo call in the deploy body.

**Damage during construction:** Buildings **can** take damage while in
Mission_Construction. `ReceiveDamage` (`0x00442230`) has no gate on the
current mission enum or on `+0x6DD`. HP begins at `Type+0xA0 Strength`
because `BuildingClass::Init_Managers @ 0x00442C40` copied it into
`+0x6C/+0x70` before constructor return — **there is no construction-driven
HP scaling** during BuildUp. A building
destroyed during BuildUp fires `OnDestroyed` (`0x00445880`) normally and
leaves a wreckage/crater. The BuildUp animation does not carry a damage-
state variant; if `SetDamagedState` fires during BuildUp (HP drops below
ConditionYellow), the steady-state overlay that `GrandOpening(1)` would
bind to gets the `+0x10` damaged-offset path on the second GrandOpening,
not during the anim itself. Practical parity note: gamemd.exe does not
visually reflect damage during BuildUp — the sprite is just the BuildUp
sequence regardless of HP.

**Pre-placed map buildings:** scenarios that bypass production assign
`MISSION_GUARD` directly and set `+0x6DD = 1`; they never enter enum 18.

---

## Decision: combined report

Final length is just over the 200-line soft cap (this report ≈ 240 lines
including headers), but both parts are tightly coupled to the same dispatch
table and share vtable / sub-state context. Splitting would require
duplicating the dispatch-setup section in both files. Keeping combined.

If future work needs to expand either part (e.g. full `OnConstructionComplete`
hook decomp or full BuildUp-anim frame walker), split into:
- `BUILDINGCLASS_MISSION_GUARD_GHIDRA_REPORT.md`
- `BUILDINGCLASS_MISSION_CONSTRUCTION_GHIDRA_REPORT.md`

## Open questions

1. **`vtable+0x4DC` / OnConstructionComplete** — full decomp deferred. It is
   listed in `BUILDINGCLASS_VTABLE_FULL_300.md` at slot ~307 (if that
   numbering is consistent). Relevant for ProduceCashTimer start (Type+0x6D0
   CDTimer), radar pip events, prerequisite unlock broadcast. Not blocking
   for Guard/Construction parity.
2. **`FUN_00465af0`** — `Type+0x1762` flag identity unknown. The function
   free-clears `Type+0xE00`/`+0xE04`. Best guess: a one-shot cached build
   description string that gets released after first BuildUp. Non-critical.
3. **BuildingAnimControl entry-0 frame count** — the frame count that drives
   `+0x6DD = 1` lives in `AnimTypeClass` referenced by
   `Type+0xF04[0].AnimType`. Actual anim-completion side-effect (writing
   `+0x6DD`) lives in `AnimClass::AI` / `AnimClass::End` — not in this
   handler.
4. **Simultaneous Assign_Mission contention** — if two code paths call
   `Assign_Mission` in the same tick (e.g., damage-driven auto-retreat vs.
   production-complete), which one wins? `Queue_Mission` overwrites, so
   order-of-operations within the sim tick matters. Documented elsewhere in
   `MISSIONCLASS_STATE_MACHINE.md`.

## Sources

### Primary decompilation (Ghidra MCP live)

- `0x0044B760` thunk_FUN_005b2e50 — Mission_Guard
- `0x005B2E50` FUN_005b2e50 — real body (`MOV EAX,0x1C2; RET`)
- `0x00449A50` BuildingClass__Mission_Construction — Mission_Construction (97 instructions)
- `0x005B3060` MissionClass::Mission_Dispatch — switch table for enum → vtable
- `0x00447780` BuildingClass::GrandOpening — anim slot binding
- `0x00442C40` BuildingClass::Init_Managers — copies `Type+0xA0` to object `+0x6C/+0x70`
- `0x007396D2..0x007396D9` MCV-deploy mission-18 assignment before Unlimbo
- `0x00465AF0` Type-level one-shot cleanup

### Vtable reads (direct memory)

- BuildingClass vtable `0x007E3EBC` + `0x214` → `0x0044B760` (confirmed)
- BuildingClass vtable `0x007E3EBC` + `0x244` → `0x00449A50` (confirmed)

### Cross-references to existing docs

- `BUILDINGCLASS_VTABLE_FULL_300.md` — slot 133, slot 145 labels
- `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` §17 — mission handler map
- `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` — refund formula, radio
- `BUILDINGCLASS_MISSION_ATTACK_AND_RESIDUALS.md` — combat buildings do NOT
  use Guard
- `MISSION_GUARD_AREAGUARD_GHIDRA_REPORT.md` — MissionClass-level generic
  Guard behavior
- `MISSIONCLASS_STATE_MACHINE.md` — full mission enum → vtable offset map
- `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` — BUILD_BEGIN (0xB), BUILD_END
  (0xC), OVER_AND_OUT (0x3) semantics
- `BUILDING_ANIMATION_21_SLOT.md` / master §12 — slot 0/1 BuildUp/idle roles
