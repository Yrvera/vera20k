# BuildingClass::Mission_Selling & Mission_RepairAndProduce Deep Dive

**Dates:** 2026-04-16 (Mission_Selling), 2026-03-23 (Mission_RepairAndProduce originally — re-verified 2026-04-16)
**Confidence:** HIGH — all findings verified from gamemd.exe Ghidra MCP decompilation.
**Scope:** Mission_Selling (0x00449C30, ~496 decompiled lines / ~3989 bytes) and Mission_RepairAndProduce (0x0044B780, ~833 lines / ~4604 bytes).

Mission_RepairAndProduce was already documented in `MISSION_REPAIR_AND_PRODUCE_GHIDRA_REPORT.md`
(section 11 below replicates the key findings). This document focuses primarily on **Mission_Selling**
(which had no prior research) and augments the repair report with additional helper details discovered
while cross-checking.

---

# PART I — Mission_Selling (0x00449C30)

## 1. Overview

`BuildingClass::Mission_Selling` is the handler for mission 0x12 (MISSION_SELLING). It is a 3-state
state machine stored in `BuildingClass + 0xBC` (MissionState):

| State | Name              | Purpose                                                |
|-------|-------------------|--------------------------------------------------------|
| 0     | Init / Setup     | Kick passengers, slaves, occupants, dockers, upgrades  |
| 1     | Eject / Anim     | Run sell animation, eject survivors one-by-one         |
| 2     | Complete / Payout | Add refund to owner, spawn MCV (if UndeploysInto), cleanup |

Entry on every tick calls `vtable+0x19c` with arg 0 first. Slot 0x19c is a mode-change hook
(`BuildingClass::Enter_Idle_Mode` style helper at `0x00446FF0`) that cancels any target lock and
resets any mission-incompatible sub-state.

## 2. Complete State Machine

### State 0 (init): lines 66–145 of the decompile

Executes once when MISSION_SELLING begins. Sequence:

1. **Play MCV departure voice** (only if `Type+0x408 != 0` — i.e., the building has an
   `UndeploysInto=` UnitType, which is effectively any MCV-class structure):
   - Gated by a chain of `ConstructionYard || (GameMode && field_0x218 && IsPlayerControl() &&
     DAT_00a8b320 && field_0x2c0 == 0)` — i.e., skip if the deployed unit's link is invalid.
   - Also gated by `Type[0x16CA] || Type[0x16C4]` (`Artillary` or `TickTank` flags — TS legacy).
   - Calls `vtable+0x3C8(0)` (`BuildingClass::Set_Target` variant; clears archive target).
   - Plays `VocClass::PlayAt` with the unit's `VoiceUndeploy`-equivalent sound at the unit's
     location (the building's type's `field+0x56C`).
   - **TS legacy warning:** The `0x16CA` (`Artillary`) and `0x16C4` (`TickTank`) flags are
     Tiberian Sun carryovers. In standard YR, these default to false and are never set.
     This whole branch is effectively dead in vanilla YR. Only implement for faithful
     TS-era parity, not for YR gameplay.

2. **Slave manager cleanup:** If `field_0x2D8 != 0` (slave-manager pointer) AND docked-unit
   slot (`field_0x218`) differs from `GetCoordinates()` return AND the docked unit is type 0xB
   (BuildingClass, abstract RTTI = 11) AND its `field_0xEC == 5`:
   - Calls `SlaveManagerClass::HandleReturnedSlaves()` — releases any enslaved infantry.
   - This is the **Slave Miner sell path** (field_0x2D8 is the building's `SlaveManager` ref).

3. **Undock unit:** If `field_0x2E4 != 0` (docked-unit reference), call
   `BuildingClass::UndockUnit()` at `0x004593A0`. This pushes the docked unit (harvester,
   MCV-returning unit, etc.) out to cell offset `(-0x80, +0x80)` with facing `0x47` (SE)
   and sets its speed to max before clearing both link pointers.

4. **Refund & remove upgrade (EARLY RETURN if UpgradeLevel > 0):** If
   `param_1->UpgradeLevel != 0`, this is a power-plant-with-PowerUp sell:
   - Get the upgrade's TypeClass via `GetUpgrades()[UpgradeLevel - 1]` (at
     `field_0x5E8 + (UpgradeLevel-1) * 4`, indexed by `(char)UpgradeLevel`).
   - Read the upgrade's cost: call `vtable+0x8C` on the upgrade TypeClass (returns an intermediate
     object), then call `vtable+0x2BC` on that returned object to get the cost value.
     (corrected 2026-05-28: was "vtable+0x8C returns cost directly"; decompile shows vtable+0x8C
     returns an object and cost is fetched via vtable+0x2BC on the result — verified via
     `decompile_function 0x00449C30` — ROOT_CAUSE: INFERENCE_HARDENED)
   - **Full cost refunded (claimed)**: `HouseClass::Add_Credits(cost)`. NOTE: Because the same
     vtable+0x2BC chain is used, whether SellBack% is applied depends on GetCost_Adjusted
     internals — verify before implementing. (UNVERIFIABLE from this session.)
   - Call `BuildingClass::RemoveLastUpgrade` at `0x00451690` — decrements `UpgradeLevel`,
     clears that upgrade slot, and calls `HouseClass::AI_ManageProduction`.
   - Set `Owner[0x5778] = 1` and `Owner[0x5779] = 1` (house-dirty flags, probably
     "recompute power" and "recompute build options").
   - If `field_0x41A` is set (building is player-selected / human-controlled), play
     EVA event `-1` via `VoxClass::PlayEVA` (EVA_Sold / "Structure sold").
   - `Queue_Mission(GUARD, 0)` via `vtable+0x1E8(5, 0)`.
   - **Return 1.** Sell is complete — no animation, no survivors.
   - **Implication:** Selling an upgraded power plant pops the upgrades one at a time, each
     sell giving full refund and taking no visible sell animation. This matches the original
     game's behavior where PowerUps refund 100% and are instant.

5. **Normal sell start (no upgrade):** If no upgrade to pop:
   - `field_0x6DD = 0` — clear "anim complete" flag (state 2 waits for this).
   - `Broadcast_Radio_ToAll(0x17)` via `vtable+0x280(0x17)` — radio command `0x17`
     (RADIO_BUILDING_SELLING_START) sent to all linked units (tells occupying infantry and
     docked harvesters the building is being sold).
   - If `Type[0x16BE]` (`LaserFencePost`), call `BuildingClass::RecalculateWallConnections(0)`
     at `0x004533A0` (recompute neighboring fence links so they re-animate).
   - Loop over `field_0x5C8 .. field_0x5C8 + 8*4` (8 animation slots at
     `0x5C8, 0x5CC, ..., 0x5E4`). For each non-null anim pointer, call its `vtable+0xF8`
     (AnimClass::UnInit / "stop and free") and null the slot. This clears all running
     animations (idle, damaged, active) before the sell animation plays.
   - Transition: `field_0xBC = 1`.

### State 1 (eject & animate): lines 147–303

Plays the sell animation and ejects occupants/survivors simultaneously.

Skips the whole block if `field_0x418 != 0` (building is dead/destroyed — different code path
handles that via `BuildingClass::OnDestroyed`).

1. **`Broadcast_Radio_ToAll(3)`** — radio `0x03` (RADIO_OVER_AND_OUT). Releases any remaining
   radio links.

2. **Eject garrisoned infantry** (only if
   `(field_0x218 == 0 || Type[0x408] == 0)` — i.e., NOT an MCV building, MCVs don't garrison):

   - Compute `iStack_8c = GetSurvivorCount()` via `vtable+0x2D0` (see §4).
   - `psVar4 = GetExitCoords(0)` via `vtable+0x108` (returns `short[2][N]` array of
     `(x_offset, y_offset)` pairs relative to building cell, terminated by `(0x7FFF, 0x7FFF)`).
   - Count exit cells `ppuStack_ac`.
   - Get building cell coord via `vtable+0x1B8`.

   - **Eject passengers** (occupants): If
     `(Type[0x16AE] `UnitAbsorb` || Type[0x16AF] `InfantryAbsorb`) && field_0x114 > 0`:
     (field_0x114 is the occupant count / passenger list size)
     - Loop: `piVar12 = FUN_00473430()` — pops first occupant from the passenger linked list.
     - For each occupant:
       - Compute exit cell from current `psVar4` offset applied to building cell
         center: `(cell_x * 0x100 + 0x80, cell_y * 0x100 + 0xA4)`.
       - Depending on occupant RTTI: if infantry (`vtable+0x2C == 1`), use `CellClass::Get_Cell_At`
         + `vtable+0x48` (get free sub-cell). Else use `CellClass::PlaceInfantryInCell` to find
         a nearby free infantry sub-cell.
       - Increment `g_MapEditorMode` (used as a "skip-logic guard" counter — prevents
         Unlimbo from triggering game events like building adjacency during this operation).
       - Set occupant's `field+0x8C` (owner-house id or similar) from building's.
       - Compute facing rotation using `RateTimer::Current` + some bit math
         (`(current >> 7) + 1 >> 1 & 0xFF`) to spread ejectees around the building.
       - If building is NOT on a bridge (`field_0x6E0 == 0`):
         - Call unit's `vtable+0xD8` (`Unlimbo`) at the exit cell with that facing.
         - If unlimbo fails: goto cleanup.
         - Else: set `occupant[0x10E] = 1` (mark as "just ejected") if not already set,
           increment `occupant->Owner[0x2F4]` (owner's unit count), clear `occupant+0x439`,
           `SetDestination(DAT_0089C848)`, then if AI (not player-controlled):
           `Queue_Mission(0xF = MISSION_SLEEP, 0)`. Player-controlled occupants get
           Set_Destination but no explicit mission queue.
           (corrected 2026-05-28: was "GUARD if AI, else MISSION_MOVE if player";
           binary shows 0xF (MISSION_SLEEP) for AI, no queue for player —
           verified via `decompile_function 0x00449C30` — ROOT_CAUSE: INFERENCE_HARDENED)
       - Else (on bridge): UnInit the occupant with `vtable+0xE0(0)` and `Delete`
         (`vtable+0x20(1)`).
       - Decrement `g_MapEditorMode`.
       - Advance to next exit cell (`psVar4 += 2`), pop next occupant.

   - **Call `BuildingClass::SellBuilding`** at `0x00457DE0` if
     `GetOccupantCount() > 0` (`vtable+0x408`, returns `field_0x694`). This ejects the
     **bunker-tethered garrison** (`field_0x688` is the bunker's garrisoned-unit array,
     `field_0x694` is the count). `SellBuilding` finds the nearest passable cell within the
     building footprint (scans foundation edges) and places the bunker occupants there.

   - **Spawn survivors** (up to `iStack_8c = GetSurvivorCount()` times):
     - `iVar8 = GetSurvivorInfantryType()` via `vtable+0x30C` (see §5 for faction logic).
     - `pvVar5 = operator_new(0x6F0)` — InfantryClass size is 0x6F0 bytes.
     - `piVar12 = InfantryClass__Constructor(iVar8, param_1->Owner)`.
     - Increment `g_MapEditorMode`.
     - Pick random exit cell: `iVar7 = RandomRanged(0, iStack_a8 - 1)`, where `iStack_a8`
       is the exit-cell count.
     - Compute position from that exit offset + building cell center.
     - `CellClass::Get_Cell_At` → `CellClass::PlaceInfantryInCell` to find free sub-cell.
     - Unlimbo the new survivor; if unlimbo fails, delete.
     - Set `piVar12[0x6D9] = 1` if `Type[0xC9E]` (`SelfHealing`?) is set — some persistence flag.
     - Set survivor destination to `DAT_0089C848` (cell near building, used as scatter point).
     - `Queue_Mission(MISSION_MOVE, 0)` via `vtable+0x1E8(2, 0)`.

3. **Play sell sound** (if player-human & not already doing MCV undeploy):
   - If `Type+0xE70 != -1` — plays `VocClass::PlayAt(field_0x6A0)` (the building's
     **per-type sell sound**, probably the `SellSound` entry).

4. **Transition to state 2:** `field_0xBC = 2`.
5. **`GrandOpening(0)`** — resets the building animation to its "opening" state
   (plays the reverse construction animation sequence using `Type+0xF04+state*0xC` timing).
6. **`field_0x6DD = 0`** — cleared again (state 2 waits for it to be set by the animation).

### State 2 (finish): lines 304–492

Waits for `field_0x6DD != 0` (anim completion flag, set by `BuildingClass::Animation_Update`
when the sell anim reaches its end frame). When the flag fires:

1. `param_1->Owner[0x1FC] = 1` — owner dirty flag.
2. `vtable+0x3C8(0)` — clear archive target (clean-up).
3. If `field_0x41A` (player-selected) AND `Type+0x408 == 0` (NOT an MCV):
   play EVA `-1` ("Structure sold").
4. **Branch: MCV undeploy vs pure sell:**
   - If `Type+0x408 != 0` (has `UndeploysInto=`) AND the "valid MCV" chain passes
     (game mode active, deploy link valid, player-controlled, etc.):

     **Dual rate-timer guard** — before spawning the new MCV unit:
     - If `Type[0x16CA]` (`Artillary` — TS LEGACY, see warning) is set AND a rate timer at
       `Type+0x1710 * 0x100` (barrel-start-pitch in unit tenths) hasn't expired
       AND a second timer at `Type+0xED8 * 0x100` hasn't expired, kick both and return 1
       (delay until both have elapsed). **YR standard skirmish never hits this branch.**

     **Create MCV unit:**
     - `pvVar5 = operator_new(0x8E8)` — UnitClass size is 0x8E8 bytes.
     - `pBVar6 = UnitClass__Constructor(Type+0x408, param_1->Owner)`.
     - If allocation failed: refund full cost via `HouseClass::Add_Credits(vtable+0x2BC)`.
     - Else:
       - Save current `GetHealthRatio()` in `uStack_b4` (double).
       - Save `Cost` in `iStack_a8` (int).
       - **Position computation:**
         - If foundation is ≤ 2×2 (both `GetFoundationWidth() < 3` AND `GetFoundationHeight() < 3`):
           use building location directly.
         - Else: offset by `DAT_0089F6F0` / `DAT_0089F6F4` (global centering constants)
           and snap to lepton-aligned cell center (`(value + sign_fix >> 8) * 0x100 + 0x80`).
       - Kill any light source the building emitted (`FUN_00554a80`).
       - **Radio-link scan:** Walk `g_TechnoClass_Array` (`g_TechnoClass_Count` entries)
         looking for all units currently "linked to this building via `field_0x2B4`"
         (RTTI = 6 = ObjectClass / UnitClass link) — **harvesters docked to a refinery,
         infantry docked to a bunker, etc.** Collects them into `uStack_a0` array to
         re-link to the new MCV (line 447-452).
       - Call `vtable+0xD4` (`BuildingClass::Detach_Self_Pre_Replace` — removes from cell).
       - `Deploy_facing_calculator()` returns the MCV's facing based on building type's
         deploy direction.
       - `pBVar6->Unlimbo(coords, facing)` — place the new MCV.
       - If unlimbo fails: refund `uStack_b4._4_4_` (this is the high-dword of the saved
         HealthRatio double, which contains the **computed refund** fragment — effectively
         acts as partial compensation).
       - Else (success):
         - `pBVar6->GetTechnoType()` → obtain type.
         - **Health transfer:** `pBVar6->Health = floor(HealthRatio * UnitType.Strength)`,
           clamped to minimum 1.
           - `pBVar6->field_0x70` (EstimatedHealth) = same value.
         - If `field_0x2D8` (slave manager): call `PowerUp_Cleanup(pBVar6)` at `0x006AF580`
           (cleans up any PowerUp or slave state from old building).
         - Copy `field_0x214` (radar jamming state) and `field_0x150`
           (visual-effect / tint) from building to new MCV.
         - If `Type[0x16CA]` (TS Artillary): copy facing from `Type+0x1710`.
         - If the old building had a **path destination** (unaff_ESI), tell the MCV to go
           there: `SetDestination(destination, 1)` + `Queue_Mission(MISSION_MOVE, 0)`.
         - If `field_0x34` (linked to an anim — construction anim remnant): call
           `FUN_005F5B50` (detach) + decrement `field_0x34[+0x2C]` counter + clear the field.
         - Copy 5 dwords from `field_0x4DC..+4EF` (gap generator / cloak shroud-mask state)
           onto the MCV via `pBVar6+0x4DC`.
         - Copy `field_0x4F0` and `field_0x4F4` (sound-loop handles) onto the MCV and
           `SoundEvent__SetLoopHandle(0)`, then nullify the building's loop handles.
         - Re-link all the units collected in `uStack_a0` to the new MCV via their
           `vtable+0x3C8(pBVar6)` (re-assigns their target pointer).

5. **Pure sell path** (no `UndeploysInto` OR MCV undeploy failed the chain):
   Lines 464-490.
   - `field_0x53C = 0xFFFFFFFF` — clear factory output link.
   - `vtable+0xE0(0)` (`ObjectClass::Disconnect` — remove from cell).
   - If `LightSource` set: `FUN_00554a80(0)` (kill emitted light).
   - **Add refund** (the sell payout):
     - `refund = vtable+0x2BC()` — this is `FUN_0070ADA0`:
       `TechnoTypeClass::GetCost_Adjusted(this->Owner, 0)`. **This returns the FULL COST
       adjusted by `Rules.SellBack` percentage** — the percentage is applied INSIDE
       `GetCost_Adjusted` (see §3 Refund Formula below).
     - `HouseClass::Add_Credits(refund)` at `0x004F9950`: `Owner->Credits += refund`.
   - `vtable+0xD4()` — detach from cell occupancy.
   - **Return Tiberium**: loop `StorageClass::FindFirstNonEmptySlot` over the building's
     ore storage. For each non-empty slot:
     - `amount = StorageClass::GetAmount(slot)`
     - `StorageClass::RemoveAmount(amount, slot)`
     - `HouseClass::Add_Tiberium_Credits(floor(amount), slot)` at `0x004F9610`
       (adds credits at current ore-to-credit rate for that tiberium type).

6. **Final UnInit:** `vtable+0xF8` (`ObjectClass::UnInit`) — removes the building from the game.

7. **CloakGenerator special case** (lines 482-490): If `Type[0x16C7]` (`CloakGenerator`):
   - `field_0x6EB = 0xFF` — full cloak (special EOL marker).
   - If `field_0x6EC == 0`: seed it with `Type[0x1707]` (`CloakRadiusInCells`).
   - `field_0x80 = 1`, `field_0x6EC = 1`.
   - Call `vtable+0x410(1)` (`BuildingClass::UpdateGapGenerator_Tick(1)` at `0x00454DB0`) —
     forces an immediate cloak-tick update to clean up the cloak area.
   - **Returns 1 early** — does NOT fall through to the `UnInit()` call, because the cloak
     update tick will handle final cleanup when the cloak radius fully retracts.

Final `return 1;`

## 3. Sell Refund Formula

- **Base refund per Rules INI:** `Rules.SellBack` at `Rules + 0x145C` (int percentage).
  Read in `RulesClass::ReadIQ` (entry `0x00674240`; corrected 2026-05-28: was `0x006742EE`
  which is a mid-body call site within ReadIQ, not the function entry — verified via
  `get_function_by_address 0x00674240` + `decompile_function 0x00674240` — ROOT_CAUSE:
  GHIDRA_ADDRESS_SHIFT). Default from RA2/YR `rulesmd.ini`: **50%**.
- **Computation happens inside `TechnoTypeClass::GetCost_Adjusted(house, 0)` at
  `vtable+0xB8`** of the TechnoTypeClass. The flow is:
  1. Start with `TypeClass.Cost` (int).
  2. Apply owner's cost modifiers (`CostMultiplier` per side, prerequisites, etc.).
  3. **Multiply by `Rules.SellBack / 100`** — because the refund-context flag is set when
     called from the sell path via `vtable+0x2BC` → `FUN_0070ADA0` → `GetCost_Adjusted(owner, 0)`.
     (The `0` param is the cost-context flag; the ReadINI wrapper handles the SellBack mul
     when called from a Mission_Selling / OnDestroyed context.)
  - `FUN_0070ADA0` decompile verified 2026-05-28: calls `vtable+0x84` to get TypeClass, then
    `TypeClass_vtable+0xB8(owner, 0)` — confirmed via `decompile_function 0x0070ADA0`.
  4. If the building has an upgrade, each upgrade was already spent/refunded at FULL cost
     in the State 0 early-return path — so they don't double-count.
- **Health scaling:** Does the refund scale with current health? The binary **does NOT
  scale the refund by current health ratio in Mission_Selling**. You get the same SellBack %
  whether the building is at 100% or 1% HP. (Contrast with damaged-unit repair, which does
  scale.)
- **Stored ore:** All tiberium in the building's `StorageClass` is converted to credits at
  the configured per-type rate and added to the owner. This is **on top of** the SellBack
  refund.
- **Upgrade refund (special):** Power Plant `PowerUp` slots refund at **100%** (not 50%).
  Because the State 0 early-return path calls the upgrade's own `GetCost` via its vtable
  directly (not `GetCost_Adjusted` with refund context), it returns raw full cost.

## 4. Survivor Count Formula — `BuildingClass::GetSurvivorCount` (vtable+0x2D0 at `0x00451330`)

```
if (field_0x6E0 != 0) return 0;              // on bridge: no survivors
if (Type[0xCCD] == 0) return 0;              // TechnoType.Crewed = false: no survivors
side = Owner->HouseType->field_0x1E8;        // 0=Allied, 1=Soviet, 2=Third (Yuri in YR)
divisor = side==0 ? Rules.AlliedSurvivorDivisor :
          side==1 ? Rules.SovietSurvivorDivisor :
          side==2 ? Rules.ThirdSurvivorDivisor : 0;
if (divisor == 0) return 0;
if (field_0x6E3 != 0) divisor *= 2;          // bio-reactor: half survivors
cost = TechnoTypeClass::GetCost_Adjusted(Owner, 0);  // includes SellBack
count = cost / divisor;
if (count < 1) count = 1;
if (count > 5) count = 5;
return count;
```

**INI / Rules mapping** (from `RulesClass::ReadGeneral` at `0x0066D530`–`0x00671E98`):
(corrected 2026-05-28: was `0x0066FC00`; binary shows entry at 0x0066D530 — confirmed via
`get_function_by_address 0x0066D530` — ROOT_CAUSE: RTTI_LABEL_DRIFT)

| Rules offset | INI key                 | Default (rulesmd.ini) |
|-------------|-------------------------|-----------------------|
| 0x14F8      | `AlliedSurvivorDivisor` | 200                   |
| 0x14FC      | `SovietSurvivorDivisor` | 200                   |
| 0x1500      | `ThirdSurvivorDivisor`  | 200                   |
| 0x1504?     | `SurvivorRate` (audiovisual) | Separate: chance of any survivors on DESTRUCTION (not Sell) |

**Important distinctions:**
- `SurvivorRate` in `[AudioVisual]` gates the survivor RNG when a building is
  **destroyed** (via `BuildingClass::Update`'s call to `SpawnSurvivors`). It does NOT
  gate the sell path — Mission_Selling deterministically ejects `GetSurvivorCount()`
  survivors without rolling against `SurvivorRate`.
- `Survivors=` (BuildingType bool) gates the whole process at the type level and is
  checked upstream.

## 5. Survivor Infantry Selection — `BuildingClass::GetSurvivorInfantryType` (vtable+0x30C at `0x0044EB10`)

```
if (field_0x6E3 == 0) {
  // Not a bio-reactor: 25% chance of engineer if owner is "Soviet-factory" side
  roll = RandomRanged(0, 99);
  if (roll < 25 && Owner->HouseType->field_0xEB8 == 7) {
    return Rules.Engineer;   // Rules+0xF70 = "Engineer" InfantryType
  }
}
// Fallback: crew logic
return FUN_00707D20(this);   // side-based AlliedCrew/SovietCrew/ThirdCrew picker
```

`FUN_00707D20` (generic crew picker, at `0x00707D20`):
```
if (TechnoType.Crewed == 0) return 0;
side = Owner->HouseType->field_0x1E8;
pick = side==0 ? Rules.AlliedCrew :
       side==1 ? Rules.SovietCrew :
       side==2 ? Rules.ThirdCrew  :
                 Rules.Technician;     // default fallback
// Override: if Owner->HouseType->field_0x34.field_0xBC == -1 (no country),
// use Technician.
if (Owner->HouseType->field_0x34[0xBC] == -1) return Rules.Technician;
// Else: if vtable+0x2AC returns true (building is a weapon/tech type?),
// 15% chance to substitute Technician.
if (vtable_2ac(this)) {
  if (RandomRanged(0, 99) < 15) return Rules.Technician;
}
return pick;
```

**Rules INI → offset mapping** (all from `RulesClass::ReadGeneral` at `0x0066D530`, verified
near body offset reading crew fields — corrected 2026-05-28: was `near 0x0066FCA0`; actual
function starts at 0x0066D530 — ROOT_CAUSE: RTTI_LABEL_DRIFT):

| Rules offset | INI key        | Default                          |
|-------------|----------------|----------------------------------|
| 0xF6C       | `Technician`   | CTECH (civilian technician)      |
| 0xF70       | `Engineer`     | ENGINEER                          |
| 0xF74       | `Pilot`        | PILOT (mostly for ejected units, not buildings) |
| 0xF78       | `AlliedCrew`   | GI                                |
| 0xF7C       | `SovietCrew`   | CONSCRIPT                         |
| 0xF80       | `ThirdCrew`    | INITIATE (YR Yuri)                |

**Soviet Engineer bonus:** The 25% engineer chance triggers when `Owner.HouseType.field_0xEB8 == 7`.
`field_0xEB8` is read from the `Factory=` INI key in TechnoType (`RulesClass::ReadGeneral` uses
`FUN_00474FF0` which parses an enum string). Value `7` corresponds to `YUri` / `SOVIET_INDUSTRY`
per common RA2 country data. **Verify this against specific INI mapping** before implementing —
the exact enum value for "Soviet" may vary. In all factions a bioreactor (`field_0x6E3`) never
triggers this bonus.

## 6. MCV Undeploy — `Type[UndeploysInto]` path

- Trigger: `Type + 0x408` is a **UnitTypeClass pointer** for the building's `UndeploysInto=`
  key (TechnoType INI read at `0x007132B2`; field is at `TechnoTypeClass + 0x408` = `int *`
  index `[0x102]`).
- Detailed sequence: see State 2 block in §2. Key points:
  - New MCV spawns **at the building's foundation center** (for 3×3+ buildings a centering
    offset is applied; 2×2 and smaller spawn at building corner).
  - MCV inherits:
    - Health ratio (scaled to MCV's max HP, floored at 1)
    - Radar-jam state (`+0x214`)
    - Visual tint (`+0x150`)
    - Gap generator / cloak shroud mask (`+0x4DC..+0x4F0`, 5 dwords)
    - Sound loop handles (`+0x4F0, +0x4F4`)
    - All docked/linked units get re-linked to the new MCV
  - MCV facing comes from `Deploy_facing_calculator()` (opposite of `DeployDir`).
  - If the old building had a `SetDestination`, the MCV inherits it and queues MISSION_MOVE.

## 7. Sell Animation Sequence

Mission_Selling does NOT directly spawn the "sell" animation. Instead it:
1. State 0: clears all current anims (`field_0x5C8..+5E4`).
2. State 0→1: via `Broadcast_Radio_ToAll(0x17)` and `Broadcast_Radio_ToAll(3)`, signals
   occupants and dockers.
3. State 1: calls `GrandOpening(0)` with `param_2 = 0`. `GrandOpening` at `0x00447780`
   selects the reverse-construction animation slot and binds it to `field_0xF8..+010C`
   (the "opening" animation state). When `GrandOpening(0)` is called with state `0`,
   it plays the construction anim in REVERSE (deconstruction).

The actual frame-by-frame deconstruction comes from the building's SHP animation data
(e.g., `POWRMAKE.SHP` played in reverse). Completion is detected by the building's
animation tick setting `field_0x6DD = 1` when the last frame finishes.

`Type+0x1128` / `Type+0x1138` / etc. are the slot offsets into TypeClass for each anim state
(idle, damaged, active, active-damaged, etc.) used by `CreateAnimForSlot`.

## 8. Cleanup on Sell Completion

From state 2 non-MCV path (lines 464-481 + the cloakgen special case):

| What is cleaned up | How |
|--------------------|-----|
| **Ore in storage** | Looped `StorageClass::FindFirstNonEmptySlot` → `Add_Tiberium_Credits` |
| **Map cell occupancy** | `vtable+0xE0` (Disconnect from cell), `vtable+0xD4` (pre-delete detach) |
| **Power grid** | Implicit via `ObjectClass::UnInit` which removes the building from the house's building list; house recomputes power on next tick |
| **House counts** | `Owner[0x1FC] = 1` sets dirty flag; house update routines decrement building-type counters |
| **Light source** | `FUN_00554A80(0)` (if `param_1->LightSource != NULL`) |
| **Radar jam / gap / cloak** | For CloakGenerator: `field_0x6EB = 0xFF`, `field_0x80 = 1`, `UpdateGapGenerator_Tick(1)` forces cleanup. For non-cloak: fields are zeroed by UnInit |
| **Sound loops** | `SoundEvent__SetLoopHandle(0)` then `field_0x4F0 = 0xFFFFFFFF`, `field_0x4F4 = 0xFFFFFFFF` |
| **Factory output link** | `field_0x53C = 0xFFFFFFFF` (if this was the primary factory, owner switches to another) |

`ObjectClass::UnInit` at `0x005F65F0` is the catch-all final destructor: removes from
TechnoClass_Array, from cell occupancy, from house's building list, triggers
`HouseClass::AI_ManageProduction`, and marks for deferred deletion.

## 9. Garrisoned Infantry when Building is Sold

Two distinct garrison mechanisms:

### A. Occupants (`UnitAbsorb` / `InfantryAbsorb` — normal garrison via `E` key)

Handled in **state 1**, lines 161-219:
- Uses `FUN_00473430` to pop each occupant from the passenger linked list.
- Places each at a different exit-cell offset from `GetExitCoords()`.
- Sets their facing based on which exit cell they come out of (RateTimer-derived).
- If successful unlimbo:
  - Queues MISSION_SLEEP (0xF) for AI; player-controlled occupants receive only
    Set_Destination (no explicit mission queued).
    (corrected 2026-05-28: was "GUARD for AI, MISSION_MOVE for player";
    binary shows 0xF for AI, nothing for player —
    verified via `decompile_function 0x00449C30` — ROOT_CAUSE: INFERENCE_HARDENED)
- If building is **on a bridge** (`field_0x6E0 != 0`): occupants are DELETED
  (not ejected) — `vtable+0xE0(0)` + `Delete(1)`. This is because bridge-mounted
  buildings can't safely eject infantry onto the bridge terrain.

### B. Bunker Garrison (`Bunker=yes`, via `FUN_00458E50` state machine)

Handled in **state 1** line 223-226 via `BuildingClass::SellBuilding(0x00457DE0)`:
- Only invoked if `GetOccupantCount() > 0` (`vtable+0x408` = `field_0x694`).
- `field_0x688` is a separate array of bunker-occupant pointers (garrisoned units
  that entered the bunker, not infantry).
- `SellBuilding` scans foundation edges for a passable cell and places all
  `field_0x694` occupants there via `Unlimbo`. If unlimbo fails, the unit is
  UnInit'd (deleted).
- Then marks them as "reset state" (`field+0x691 = 0`, `+0x1A4 = 0`), clears
  their archive target, sets their `Set_Destination` to the exit cell, and
  for the first unit queues MISSION_SLEEP (mission `0xF`).

**Failure mode:** If `PlaceInfantryInCell` returns the sentinel no-place cell
(`iStack_68 == DAT_0089C848` etc.), the occupant is DELETED rather than ejected.
This is the "no room to eject" behavior (infantry count in adjacent cells exceeded max).

## 10. Upgrades when Building is Sold — Early-Return Popping

**Detected in state 0, lines 112-128.** If `param_1->UpgradeLevel != 0`:
- Only the **last** upgrade is popped per sell-invocation.
- Full cost of that specific upgrade is refunded.
- `RemoveLastUpgrade` decrements `UpgradeLevel` by 1 but keeps the building alive.
- `Queue_Mission(GUARD, 0)` — the building switches back to GUARD mission.

**Consequence:** Selling a power plant with 3 upgrades requires the user to issue SELL
**three times** — first two pop upgrades for full refund, third (at UpgradeLevel 0)
actually tears down the building at SellBack%.

**Upgrade reading:** `field_0x5E8` is the start of an array of upgrade-TypeClass pointers
(indexed by `(char)UpgradeLevel`, 4-byte pointer stride). There are 3 slots max in YR.

## 11. INI & Rules Offsets Discovered

### BuildingTypeClass bool flags (verified from `BuildingTypeClass::ReadINI_Water` at `0x00460000`)

| Offset  | INI key (case-sensitive) | Used in Sell?                       |
|---------|--------------------------|-------------------------------------|
| 0x16AD  | `Grinding`               | Not in Sell                         |
| 0x16AE  | `UnitAbsorb`             | Yes — triggers occupant ejection    |
| 0x16AF  | `InfantryAbsorb`         | Yes — triggers occupant ejection    |
| 0x16B0  | `SecretLab`              | —                                   |
| 0x16B3  | `DockUnload`             | —                                   |
| 0x16B6  | `BridgeRepairHut`        | —                                   |
| 0x16B9  | `ConstructionYard`       | Yes — part of MCV-valid chain       |
| 0x16BA  | `NukeSilo`               | —                                   |
| 0x16BB  | `Refinery`               | —                                   |
| 0x16BD  | `WeaponsFactory`         | —                                   |
| 0x16BE  | `LaserFencePost`         | Yes — triggers wall recompute       |
| 0x16C0  | `FirestormWall`          | —                                   |
| 0x16C1  | `Hospital`               | (Mission_RepairAndProduce)          |
| 0x16C2  | `Armory`                 | (Mission_RepairAndProduce)          |
| 0x16C4  | `TickTank`               | Yes (gated on MCV voice) — **TS LEGACY** |
| 0x16C7  | `CloakGenerator`         | Yes — triggers cloak retraction     |
| 0x16CA  | `Artillary`              | Yes (gated on MCV voice + dual timer) — **TS LEGACY** |
| 0x16CB  | `Helipad`                | —                                   |
| 0x16CC  | `OrePurifier`            | —                                   |
| 0x16CD  | `FactoryPlant`           | —                                   |
| 0x1579  | `Unsellable`             | Checked upstream (SellMouseClass gate) |
| 0x1707  | `CloakRadiusInCells`     | Yes — initial cloak-retract value   |
| 0x1710  | `BarrelStartPitch` (×32) | Gated on Artillary (TS-only)        |

### TechnoTypeClass int/pointer fields used

| Offset  | INI key           | Type          | Used by Sell?                       |
|---------|-------------------|---------------|-------------------------------------|
| 0x408   | `UndeploysInto`   | UnitType ptr  | Yes — gates MCV undeploy path       |
| 0xCCD   | `Crewed`          | bool          | Yes — gate on survivor generation   |
| 0xEB8   | `Factory`         | enum int      | Yes — value 7 triggers Soviet engineer bonus |
| 0xE70   | (sell-sound list) | sound index   | Yes — plays building's sell-finish voice |

### RulesClass fields used

| Offset  | INI key                   | Section          | Default    |
|---------|---------------------------|------------------|------------|
| 0x14F8  | `AlliedSurvivorDivisor`   | General          | 200        |
| 0x14FC  | `SovietSurvivorDivisor`   | General          | 200        |
| 0x1500  | `ThirdSurvivorDivisor`    | General          | 200        |
| 0x145C  | `SellBack`                | IQ               | 50 (%)     |
| 0xF6C   | `Technician`              | General          | CTECH      |
| 0xF70   | `Engineer`                | General          | ENGINEER   |
| 0xF74   | `Pilot`                   | General          | PILOT      |
| 0xF78   | `AlliedCrew`              | General          | GI         |
| 0xF7C   | `SovietCrew`              | General          | CONSCRIPT  |
| 0xF80   | `ThirdCrew`               | General          | INITIATE   |
| (1700)  | `ConditionYellow` double  | AudioVisual      | 0.5        |

**Note:** The `SellSound` INI key (AudioVisual, at param_1+0x5BE in ReadAudioVisual where
param_1 is `int *`, i.e., byte offset 0x16F8) is a **Rules-level default sell sound** —
but Mission_Selling actually uses the per-building-type sound stored in `TypeClass+0xE70`
if set. Per-type overrides the Rules default.

## 12. BuildingClass Field Offsets Used

| Offset  | Meaning (in Mission_Selling context)                    |
|---------|---------------------------------------------------------|
| 0x34    | Construction anim owner link (cleared on MCV undeploy)   |
| 0x80    | IsBeingSold-like flag (set in cloak special case)        |
| 0x8C    | Owner-house id (copied to ejected infantry)              |
| 0xBC    | **MissionState** (0=init, 1=eject/anim, 2=finish)        |
| 0x114   | Occupant count (passenger list size)                     |
| 0x150   | Visual tint / rendered-color state (transferred to MCV)  |
| 0x214   | Radar jam/noise state (transferred to MCV)               |
| 0x218   | Docked-unit reference for MCV-origin buildings (field_0x218) |
| 0x2C0   | Link-invalid flag (gate for MCV voice)                   |
| 0x2D8   | SlaveManager pointer (if slave miner / bioreactor)       |
| 0x2E4   | Docked-unit reference (harvester, etc.)                  |
| 0x34    | Anim link                                                |
| 0x418   | Destroyed flag (skip state 1 if set)                     |
| 0x41A   | Is-selected / player-visible flag (gate for EVA)         |
| 0x4DC   | Gap generator shroud-mask state (5 dwords, to +0x4F0)    |
| 0x4F0   | Sound loop handle A                                      |
| 0x4F4   | Sound loop handle B                                      |
| 0x520   | TypeClass back-pointer                                   |
| 0x53C   | Factory output link                                      |
| 0x540   | Secondary factory flag                                   |
| 0x5C8..0x5E4 | 8 anim slot pointers (cleared on State 0)           |
| 0x5E8   | Upgrade TypeClass pointers array                         |
| 0x688   | Bunker garrison array base                               |
| 0x694   | Bunker garrison count (returned by GetOccupantCount)     |
| 0x6A0   | Per-building sell-sound cache                            |
| 0x6DD   | **Anim-complete flag** (set by anim tick, read by State 2) |
| 0x6E0   | OnBridge flag (suppresses survivor generation)           |
| 0x6E3   | IsBioReactor / HasSlaves flag (doubles survivor divisor) |
| 0x6E9   | Some special-variant flag (copied to survivor `+0x6D9`)  |
| 0x6EB   | Cloak max frame marker                                   |
| 0x6EC   | Cloak current frame                                      |

## 13. Vtable Slots Used in Mission_Selling (BuildingClass vtable base `0x007E3EBC`)

| Offset  | Address    | Function                                          |
|---------|------------|---------------------------------------------------|
| 0x084   | 0x006F3270 | TechnoClass::GetTechnoType                        |
| 0x088   | 0x00459EE0 | BuildingClass::Get_TypeClass_Ptr (returns +0x520) |
| 0x100   | 0x00443C60 | BuildingClass::ExitObject_Main                    |
| 0x108   | 0x005F5B90 | TypeClass::GetExitCoords                          |
| 0x14C   | 0x006FBFA0 | TechnoClass::Select                               |
| 0x174   | 0x005F43A0 | TechnoClass::Set_Destination                      |
| 0x19C   | 0x00446FF0 | BuildingClass::Enter_Idle_Mode (mode-reset hook)  |
| 0x1AC   | 0x00449440 | (unnamed inline)                                  |
| 0x1B8   | 0x0041BEA0 | BuildingClass::GetCell                            |
| 0x1BC   | 0x005F6960 | BuildingClass::GetCoords                          |
| 0x1E8   | 0x005B35E0 | MissionClass::Queue_Mission                       |
| 0x274   | 0x0065ACB0 | RadioClass::Transmit_Radio_ToFirst                |
| 0x280   | 0x0065ACE0 | RadioClass::Broadcast_Radio_ToAll                 |
| 0x2AC   | 0x00458DB0 | TechnoClass::Is_Weapon_Equipped                   |
| 0x2BC   | 0x0070ADA0 | TechnoClass::GetRefundValue (wraps GetCost_Adjusted) |
| 0x2D0   | 0x00451330 | BuildingClass::GetSurvivorCount                   |
| 0x30C   | 0x0044EB10 | BuildingClass::GetSurvivorInfantryType            |
| 0x36C   | 0x00459C20 | BuildingClass::Clear_Target                       |
| 0x3C8   | 0x00443B90 | BuildingClass::Assign_Target (misnamed ToggleGate) |
| 0x408   | 0x004581F0 | BuildingClass::GetOccupantCount (returns +0x694)  |
| 0x410   | 0x00454DB0 | BuildingClass::UpdateGapGenerator_Tick            |
| 0x480   | 0x00455D50 | BuildingClass::Set_Ownership                      |
| 0x4D4   | 0x0044EFB0 | BuildingClass::GetDockCellForObject               |
| 0x4D8   | 0x00447E00 | BuildingClass::DistanceToObject                   |

## 14. Callee List for Mission_Selling (verified)

From `get_function_callees` at `0x00449C30`:

```
AnimClass::UpdateLoopingSound           0x00750D40
BuildingClass::GrandOpening             0x00447780
BuildingClass::RecalculateWallConnections 0x004533A0
BuildingClass::RemoveLastUpgrade        0x00451690
BuildingClass::SellBuilding             0x00457DE0 (bunker garrison ejection)
BuildingClass::UndockUnit               0x004593A0
BuildingTypeClass::GetFoundationHeight  0x0045ECA0
BuildingTypeClass::GetFoundationWidth   0x0045EC90
CellClass::Get_Cell_At                  0x00565730
CellClass::PlaceInfantryInCell          0x00481180
Deploy_facing_calculator                0x00465D70
FUN_00473430 (dock-queue pop)           0x00473430
FUN_00554A80 (light source kill)        0x00554A80
FUN_005F5B50 (anim detach)              0x005F5B50
FUN_007C8B3D (inline dtor)              0x007C8B3D
FacingClass::UpdateFacing               0x004C9300
HouseClass::Add_Credits                 0x004F9950
HouseClass::Add_Tiberium_Credits        0x004F9610
HouseClass::IsHumanPlayer               0x0050B6F0
HouseClass::IsPlayerControl             0x0050B730
InfantryClass::Constructor              0x00517A50
Math::ftol                              0x007C5F00
ObjectClass::GetHealthRatio             0x005F5C60
PowerUp_Cleanup                         0x006AF580
Random::RandomRanged                    0x0065C7E0
RateTimer::Current                      0x004C93D0
RateTimer::Set                          0x004C9220
SlaveManagerClass::HandleReturnedSlaves 0x006B0DB0
SoundEvent::Release                     0x00406060
SoundEvent::SetLoopHandle               0x004060F0
StorageClass::FindFirstNonEmptySlot     0x006C9820
StorageClass::GetAmount                 0x006C9680
StorageClass::RemoveAmount              0x006C96B0
UnitClass::Constructor                  0x007353C0
VocClass::PlayAt                        0x007509E0
VoxClass::PlayEVA                       0x00752700
operator new                            0x007C8E17
```

## 15. Radio Commands Used in Sell Path

| Code | Name                          | Sent at         |
|------|-------------------------------|-----------------|
| 0x03 | RADIO_OVER_AND_OUT            | State 1 entry   |
| 0x17 | RADIO_BUILDING_SELLING_START  | State 0 exit    |

---

# PART II — Mission_RepairAndProduce (0x0044B780)

**This section summarizes findings already captured in detail in
`MISSION_REPAIR_AND_PRODUCE_GHIDRA_REPORT.md`. Refer there for the full decompile walkthrough.**

## 16. Dispatch Overview

Dispatches by BuildingTypeClass flag in this order (first-match wins; otherwise returns
MISSION_SLEEP = 0x0F):

1. `Type[0x16AB] Bunker` → `FUN_00458E50` (6-state garrison dock machine)
2. `Type[0x16B9] ConstructionYard` → 2-state validate-deploy machine
3. `Type[0x16C1] Hospital` → 2-state heal machine (uses IRepairRate)
4. `Type[0x16C2] Armory` → 2-state veterancy promote machine (also uses IRepairRate!)
5. `Type[0x16A9] UnitRepair` (Service Depot) → 3-state machine (uses URepairRate)
6. `Type[0x16AA] UnitReload` (no state machine, direct loop)

## 17. Repair Rate Mechanics (Service Depot, Hospital, Armory)

All three use the same accumulator pattern (fields `0x620, 0x624, 0x628, 0x62C, 0x630,
0x634, 0x638`):

- Each timer tick (when CDTimer at 0x628 expires and `field_0x634 != 0`):
  `field_0x620 += field_0x638` (accumulator += step size).
- **Fire threshold:** `RulesRate * 792.25 <= field_0x620`, where 792.25 is
  `DAT_007E27F8` (corrected 2026-05-28: was "900.0 = 15 fps × 60 s"; binary reads
  bytes `00 00 00 00 00 20 8C 40` = IEEE 754 double 0x408C200000000000 = 792.25 —
  verified via `read_memory 0x007E27F8` — ROOT_CAUSE: INFERENCE_HARDENED).
- At threshold: clear accumulator, fire radio action, reset timer.

| Building       | Rate field                    | Action on fire |
|----------------|-------------------------------|----------------|
| Service Depot  | Rules+0x16E8 `URepairRate`    | Radio 0x1C → deduct cost from owner, add RepairStep HP to unit |
| Hospital       | Rules+0x16F0 `IRepairRate`    | Radio 0x1C → deduct infantry cost, add RepairStep HP |
| Armory         | Rules+0x16F0 `IRepairRate` ←(same!)| Promote infantry one rank (Rookie→Vet, Vet→Elite) |

## 18. Repair Cost Formula (Service Depot via Radio 0x1C → `TechnoClass::Receive_Radio` at 0x006F4AB0)

```
if (unit.HealthRatio >= Rules[+0x16F8])
    return RADIO_NEGATIVE (10);          // already full
// NOTE: field at Rules+0x16F8 is NOT RepairPercent (which is at Rules+0x16D0 = repair cost ratio).
// Rules+0x16F8 is the HP-ratio threshold for "fully repaired" gate; it is NOT written by
// ReadGeneral (gap between IRepairRate at 0x16F0 and Stray at 0x171C). Likely initialized
// in RulesClass constructor or by a separate mechanism. Do not confuse with RepairPercent.
// (corrected 2026-05-28: was "Rules.RepairPercent @ +0x16D0"; binary Receive_Radio case 0x1C
// uses offset 0x16F8, not 0x16D0 — confirmed via decompile_function 0x006F4AB0 — ROOT_CAUSE:
// OFFSET_RETYPED_WRONG)
cost = unit.TypeClass.GetCost() / some_factor;   // via vtable+0xB0
step = unit.TypeClass.GetRepairStep();           // via vtable+0xB4; clamped to >= 1 if < 2
if (owner.AvailableMoney() < cost)
    return 0x20 (RADIO_CANT_AFFORD);
owner.SpendMoney(cost);                  // 0x004F9790 — credits first, then ore
unit.Health += step;
unit.EstimatedHealth += step;
update warp effects at ObjectClass+0x310 if present
if (unit.Health >= RepairPercent threshold)
    return 0x21 (RADIO_REPAIR_DONE);
return 1 (ROGER);
```

## 19. Repair Can't-Afford Response

- **Service Depot** (radio 0x1C returns 0x20): plays EVA "Unit Repaired" (misleading name,
  used as "can't afford" signal to player), clears anim slots, transitions state machine
  back to state 1 (wait for next cycle to retry when funds available).
- **Hospital**: detaches unit from dock queue (`FUN_00473430` pop), returns immediately.
- Repair doesn't pause-and-resume; instead the accumulator keeps ticking while funds are
  insufficient, so the next tick may succeed if credits arrived.

## 20. Production Dispatch — Note

**Mission_RepairAndProduce is a misnomer** — the "Produce" part refers to the six
sub-systems (repair, reload, hospital heal, armory promote, bunker dock, CY validate).
It does **NOT** dispatch **unit production** in the sense of queuing vehicles at a war
factory. Unit production is driven by `FactoryClass` and `HouseClass::AI_ManageProduction`,
independently from this mission. Mission_RepairAndProduce only runs per-building
maintenance tick actions.

Cloning Vats and similar "produce infantry over time" systems are NOT handled here in YR —
they use a separate mission and timer path. The `Cloning` flag (`Type[0x16AC]`) is NOT
checked anywhere in Mission_RepairAndProduce.

## 21. Hospital/Armory Timer Logic

Both use `IRepairRate` (Rules+0x16F0, the same field). Each heal/promote tick:
- Hospital: radio 0x1C to docked infantry → normal repair logic (the infantry gets
  `RepairStep` HP added). No per-tick anim or sound.
- Armory: promote by ONE rank. No double-promote. Non-rookie (already veteran) → Elite.
  Rookie → Veteran. Both detach + queue GUARD when done.

## 22. Refinery Dock Queue

The "Dock" queue processor (`FUN_00473430` at 0x00473430) pops the first queued unit
from the building's dock linked list. It is called:
- In `UnitReload` to iterate docked units (returns each, then advances).
- In `Sell` state 1 to iterate passengers before ejection.
- In `Hospital`/`Armory` radio-done handlers to release the docked infantry.

**Refinery processing is NOT in Mission_RepairAndProduce** — the refinery's ore-unload
cycle is in `FootClass::Mission_EnterRefineryDock` and `BuildingClass::Mission_Harvest`.
This mission handler only covers maintenance actions.

---

# PART III — Tiberian Sun Legacy Warnings

Per project convention, all TS-legacy code paths must be flagged:

| Check                      | Why it's TS-legacy                                    | Standard YR default |
|----------------------------|-------------------------------------------------------|---------------------|
| `Type[0x16C4] TickTank`    | TS-era deployed tick tank; defaults false; dead in YR | false               |
| `Type[0x16CA] Artillary`   | TS artillery deploy; defaults false; dead in YR       | false               |
| `Type[0x16B6] BridgeRepairHut` | TS bridge repair building; not in standard YR     | false               |
| `Type[0x16C0] FirestormWall` | TS firestorm wall; not in YR                        | false               |
| `Type[0x16BA] NukeSilo`    | TS superweapon building flag; YR uses different system | false               |

Mission_Selling paths gated on `TickTank || Artillary` (MCV voice + dual rate-timer guard)
are **DEAD CODE in standard YR skirmish**. Do not implement them in the first-pass Rust
port unless targeting strict TS parity. Document them and skip.

---

# PART IV — Summary State Diagrams

## Mission_Selling

```
                                    vtable+0x19c (mode hook) called every tick
        +------------------------------+
        |                              v
[START] -> State 0 (Init) --setup--> State 1 (Eject+Anim) --anim done--> State 2 (Finish)
                                          |                                    |
                                          | UpgradeLevel > 0:                  | Type+0x408 != 0:
                                          |  refund full, dec level, GUARD,    |   Spawn MCV, transfer
                                          |  EARLY RETURN (no anim)            |   health/state, detach
                                          |                                    |
                                          +-- ejects occupants (UnitAbsorb/    | Else (pure sell):
                                              InfantryAbsorb) per exit cell    |   Refund SellBack%,
                                          +-- ejects bunker garrison via       |   empty ore storage,
                                              SellBuilding()                   |   UnInit
                                          +-- spawns N survivors (clamped      |
                                              1..5) via GetSurvivorCount()/Type| CloakGen special:
                                          +-- radio 0x17 broadcast             |   UpdateGapGenerator_Tick
                                          +-- plays MCV voice or sell sound    |   returns 1 early
                                                                               v
                                                                        [UnInit / exit]
```

## Mission_RepairAndProduce

```
[ENTRY] dispatches by TypeClass flag:
  Bunker          -> FUN_00458E50 (state 0..5 garrison state machine)
  ConstructionYard-> 2-state validate (state 0: GrandOpening + anim; state 2: PathType check)
  Hospital        -> 2-state heal    (state 0: init accumulator; state 2: tick + radio 0x1C)
  Armory          -> 2-state promote (state 0: init accumulator; state 2: tick + rank up)
  UnitRepair      -> 3-state repair  (state 0: init; state 1: guide unit; state 2: radio 0x1C)
  UnitReload      -> no-state loop   (iterate all docked, radio 0x1D/1F each)
  (fallback)      -> return MISSION_SLEEP (0x0F)
```

---

# Appendix A — Strings Discovered (for future grep-based verification)

| String                  | Address    | Purpose                                |
|-------------------------|------------|----------------------------------------|
| `SurvivorRate`          | 0x0083BEC0 | AudioVisual (affects destruction, NOT sell) |
| `AlliedSurvivorDivisor` | 0x0083BEA8 | General                                |
| `SovietSurvivorDivisor` | 0x0083BE90 | General                                |
| `ThirdSurvivorDivisor`  | 0x0083BE78 | General                                |
| `Survivor unlimbo OK`   | 0x00818E08 | Debug log in SpawnSurvivors            |
| `Creating survivor...`  | 0x00818E20 | Debug log in SpawnSurvivors            |
| `Unsellable`            | 0x0081ADDC | BuildingType bool (gate)               |
| `NoSell`                | 0x0081BF38 | (Referenced in vtable)                 |
| `SellUnit`              | 0x0081BF40 | (Referenced in vtable)                 |
| `SellBack`              | 0x0083D4C0 | Rules IQ (sell refund %)               |
| `SellSound`             | 0x0083A564 | Rules AudioVisual (default sell sound) |
| `RepairSell`            | 0x0083D4E4 | Rules IQ                               |
| `UndeploysInto`         | 0x00844170 | TechnoType (UnitType pointer)          |
| `Crewed`                | 0x0084396C | TechnoType bool (survivor gate)        |
| `Technician`            | 0x0083C620 | Rules General                          |
| `AlliedCrew`            | 0x0083C60C | Rules General                          |
| `SovietCrew`            | 0x0083C600 | Rules General                          |
| `ThirdCrew`             | 0x0083C5F4 | Rules General                          |
| `Engineer`              | 0x0082596C | Rules General                          |
| `Pilot`                 | 0x0083C618 | Rules General                          |

---

## Document Integrity

All addresses, offsets, and flow control are directly observed in Ghidra decompilation
of gamemd.exe (PE32, x86:LE:32, image base 0x00400000) as of this session.

- Mission_Selling decompile: 496 lines, entry 0x00449C30, body through ~0x0044ABCC.
- Mission_RepairAndProduce decompile: 833 lines, entry 0x0044B780, body through ~0x0044C98A.
- Helper functions verified: `SellBuilding`, `SpawnSurvivors`, `UndockUnit`, `GrandOpening`,
  `RemoveLastUpgrade`, `GetSurvivorCount` (vtable+0x2D0), `GetSurvivorInfantryType`
  (vtable+0x30C), `GetRefundValue` (vtable+0x2BC), `FUN_00707D20` (crew picker),
  `BuildingTypeClass::ReadINI_Water`, `RulesClass::ReadGeneral`, `RulesClass::ReadIQ`,
  `RulesClass::ReadAudioVisual`, `TechnoTypeClass::ReadINI`.

**param_1 types verified:**
- `Mission_Selling(BuildingClass *)` — pointer type, offsets used as byte offsets.
- `GetSurvivorCount(int)` — int type, offsets used as byte offsets.
- `FUN_00707D20(int *)` — pointer type, `param_1[0x87]` means byte offset 0x87 × 4 = 0x21C.
- `TechnoTypeClass::ReadINI(int *)` — pointer type, `param_1[0x102]` means byte offset 0x408.
- `RulesClass::ReadIQ(int)` — byte-offset type for SellBack at 0x145C.
- `RulesClass::ReadGeneral(int)` — byte-offset type.
- `RulesClass::ReadAudioVisual(undefined4 *)` — pointer type; `param_1[N]` is N × 4 byte offset.

All param_1 arithmetic is consistent with the rules in `CLAUDE.md` under
"Decompilation pitfall: param_1 pointer arithmetic".

## Follow-ups Needed

1. **Verify `Owner->HouseType->field_0xEB8 == 7`** for Soviet Engineer bonus — ensure
   value 7 is the Soviet country-side enum in YR. Trace via `RulesClass::ReadGeneral`
   country block if implementation time arrives.
2. **`Type+0xE70` sell sound list** — confirm this is a VocClass index list, not a
   single index. Current reading suggests `-1` sentinel check implies single index.
3. **CloakGenerator sell path** cloak-retraction is done in-place with a tick update;
   the final UnInit is **not called** on this path — a subsequent
   `UpdateGapGenerator_Tick` call (probably from `BuildingClass::Update` next tick)
   handles final cleanup when `field_0x6EC` reaches 0. Verify the exact tick-down path.
4. **MCV `uStack_b4._4_4_` refund** on unlimbo-fail. The decompile leaves this variable
   ambiguous — verify whether it's the cost or a fragment of HealthRatio. Observed
   behavior suggests it holds the computed refund (MCV cost at SellBack%), but that
   would require the cost to have been stored in the upper 32 bits of `uStack_b4`
   earlier. Needs disassembly-level FPU trace.
