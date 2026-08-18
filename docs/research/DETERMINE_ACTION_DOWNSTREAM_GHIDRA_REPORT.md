# DisplayClass::DetermineAction Downstream — Ghidra Research Report (2026-04-24)

Extends `DISPLAYCLASS_GHIDRA_REPORT.md` §5.1 — which covered
`DetermineAction` (`0x692610`) itself — by decompiling **every
downstream What_Action polymorph** that the function dispatches to
via `best->vtable[0x70]` (cell path) and `best->vtable[0x74]`
(target path), plus the `SelectBestObjectForAction` priority
scorer, and enumerating the complete action-code set observed
across the hierarchy.

**Confidence:** HIGH for decompiled functions; MEDIUM for action
labels cross-referenced from SetCursorFromAction's switch. Some
action codes are still guess-labeled where the SetCursorFromAction
entry was "—".

**Active in YR:** Yes. Click resolution runs every hover tick and
every mouse event.

**Addresses (primary):**
- `0x00692610` — `DisplayClass::DetermineAction` (already documented upstream)
- `0x005353D0` — `SelectBestObjectForAction` — consolidates selected units → one "best" for action
- `0x00700600` — `TechnoClass::What_Action_OnCell` (base)
- `0x006FFEC0` — `TechnoClass::What_Action_OnObject` (base)
- `0x004DDDE0` — `FootClass::What_Action_OnCell` (shroud wrapper)
- `0x004DDED0` — `FootClass::What_Action_OnObject` (shroud wrapper)
- `0x0051F800` — `InfantryClass::What_Action_OnCell`
- `0x0051E3B0` — `InfantryClass::What_Action_OnObject`
- `0x007404B0` — `UnitClass::What_Action_OnCell`
- `0x0073FD50` — `UnitClass::What_Action_OnObject`
- `0x00417CC0` — `AircraftClass::What_Action` (unified cell + object)

---

## 1. Overview

Click resolution in gamemd.exe is a **two-stage polymorphic dispatch**:

```
(user clicks / moves cursor)
   │
   ▼
DisplayClass::DetermineAction(cell, target, modifier)     [0x692610]
   │
   ├── (1) Consult global UI mode flags (deploy/sell/chrono/place/enter)
   │       If active → inject specific override action codes
   │
   ├── (2) If any units selected → pick the "best" via
   │       SelectBestObjectForAction(cell, target)         [0x5353D0]
   │       → returns a single TechnoClass*
   │
   ├── (3) Virtual dispatch on the chosen unit:
   │       target == NULL ? best->vtable[0x70](cell, modifier)   // What_Action_OnCell
   │                     : best->vtable[0x74](target, 0)         // What_Action_OnObject
   │
   │       Dispatch resolves to the subclass override:
   │         Infantry → InfantryClass::What_Action_*
   │         Unit     → UnitClass::What_Action_*
   │         Aircraft → AircraftClass::What_Action         (unified)
   │         (Building doesn't override — uses TechnoClass base)
   │
   │       Each subclass calls the next-base-up and then applies its
   │       own specializations (engineer/harvest/deploy/land/dock/etc.)
   │
   └── (4) Apply UIModeLock override, final fallbacks
   │
   ▼
Action code (integer) — consumed by SetCursorFromAction for cursor
graphic + by BandBox_LeftUp for command dispatch.
```

The polymorphic dispatch is where **most of the ~30+ action-code
variety comes from**. The upstream DetermineAction only adds global
mode overrides (sell mode, deploy mode, chrono mode, etc.). The
real interesting decision logic is in each subclass's overrides.

---

## 2. SelectBestObjectForAction — 0x5353D0

Given `g_CurrentObjects_Data[0..g_CurrentObjects_Count]` (the current
selection), returns the single `TechnoClass*` that should *apply*
the action. Ties broken by distance to `(cell, target)`.

### Priority scoring (higher wins)

```
for each selected techno in g_CurrentObjects:
    score = 0

    if obj is NULL or (obj->field_0x14 & 1) == 0:
        score = 0      // invalid / not active
    elif obj->field_0x298 != 0:
        score = 0      // flagged disabled (in crate, demo, etc.)
    elif obj->vtable[0x37C]():
        score = 1      // some "can't act" condition
    elif obj->vtable[0x2AC]():            // "Is Techno Selectable For Action"
        score = 3      // default for any viable techno
        if obj->vtable[0x2C]() != 6:       // not a building (AbstractType != 6)
            score = 4                       // mobile unit baseline
            if TechnoClass::GetWeaponRange(obj, -1) > 0:
                score = 5                  // armed mobile unit (highest)
```

### Tie-break by distance

If two objects share the highest score, the one closer in 3D
Euclidean distance to the cursor wins. Distance source:
- If cell coord non-NULL: `(cell.X * 0x100 + 0x80, cell.Y * 0x100 + 0x80, 0)` — cell center in leptons
- Else: `target->vtable[0x48]()` — target's world leptons

Distance formula: `sqrt(dx² + dy² + dz²)` via `Sqrt_Approx` then
`Math__ftol`.

### Implication

A mixed selection of `[MCV, Tanya, Grizzly]` applying an attack on
an enemy tank → Grizzly (armed mobile, score 5) beats Tanya (armed,
score 5, same) tied on score → distance breaks it. MCV (mobile but
unarmed, score 4) loses.

A mixed selection of `[2× Construction Yard, 1× Miner]` applying a
move-click → Miner (mobile unarmed, score 4) beats both CYs (score 3,
building). So move orders on a selection always route through the
most mobile unit.

This matters for **determining which cursor to show**. The shown
action reflects what the best unit could do, not necessarily what
every selected unit will do.

---

## 3. The What_Action inheritance chain

| Subclass | OnCell addr | OnObject addr | Notes |
|----------|-------------|---------------|-------|
| TechnoClass (base) | `0x00700600` (unnamed `FUN_00700600`) | `0x006FFEC0` | Modifier/hotkey probes; base attack/dock logic |
| FootClass | `0x004DDDE0` | `0x004DDED0` | Thin wrapper: base result + shroud override |
| InfantryClass | `0x0051F800` | `0x0051E3B0` | Garrison, engineer capture, hospital, harvest-minor, sell |
| UnitClass | `0x007404B0` | `0x0073FD50` | Deploy, harvest, crush, repair-building |
| AircraftClass | (unified) `0x00417CC0` | (unified) `0x00417CC0` | Airstrip land, aircraft dock |
| BuildingClass | — | — | No override; uses TechnoClass base. Buildings don't issue orders. |

### Dispatch pattern — each override does:

```
int <Subclass>::What_Action_OnObject(target, modifier):
    base = FootClass::What_Action_OnObject(target, modifier)  // → TechnoClass base
    if base == 8: return 8                                    // force-attack short-circuits everything
    // then subclass-specific overrides mutate `base` based on:
    //   - class-specific flags on self / target TypeClass
    //   - hotkey modifiers (ctrl/shift/alt)
    //   - health ratios (e.g. repair-if-damaged)
    //   - ally/enemy/self relationships
    //   - target AbstractType (Unit=1, Infantry=2, Aircraft=3, Building=6, Terrain=15, etc.)
    return final_action
```

### FootClass's only contribution: shroud handling

FootClass's OnCell and OnObject just wrap their TechnoClass
equivalents and add ONE rule:

```
if target is in a SHROUDED cell AND base returned non-zero AND solo campaign:
    return TypeClass[+0xC8D] ? 1 : 2
    // i.e. attack-into-shroud if type allows, otherwise move-into-shroud
```

The `0xC8D` flag is likely the `IgnoreFog=yes` or similar
ivision-gated attack flag — tanks generally have it (so shroud-attacks
work); infantry generally don't.

---

## 4. Action-code enum — consolidated

Action codes observed across all decompiled functions, cross-referenced
against `SetCursorFromAction`'s switch from the existing DisplayClass
doc §5.2.

| Code | Hex | Cursor (default) | Source | Meaning |
|------|-----|------------------|--------|---------|
| 0 | 0x00 | (default tinted) | all | Invalid / do nothing |
| 1 | 0x01 | 0x12 | base | Attack (generic) |
| 2 | 0x02 | 0x13 | base | Move |
| 3 | 0x03 | 0x19 | Infantry/Unit | Repair ally building (health < threshold) |
| 4 | 0x04 | 0x1B | base | Self-select |
| 5 | 0x05 | 0x14 | base | Attack via best weapon |
| 6 | 0x06 | — | UnitClass::OnCell | **Harvest** — ore cell + miner type |
| 7 | 0x07 | (select)  | base | Hover select |
| 8 | 0x08 | (force-attack SHP) | base | Force-attack (ctrl held) |
| 9 | 0x09 | 0x19 | InfantryClass | Enter building (garrison/soylent/etc) |
| 0xA (10) | 0x0A | 0x22 | DetermineAction | Deploy mode / Unload |
| 0xB | 0x0B | 0x19 | UnitClass | Repair (remote variant) |
| 0xC (12) | 0x0C | 0x1E | BandBox_LeftUp | Attack-move |
| 0xD (13) | 0x0D | 0x1F | BandBox_LeftUp | Guard area |
| 0xE (14) | 0x0E | — | DetermineAction | (in enter/gather mode) Move variant |
| 0xF (15) | 0x0F | — | DetermineAction | Deploy/Unload (deploy mode) |
| 0x10 | 0x10 | 0x34 | Infantry | **Capture** (engineer → enemy structure with `1577` flag) |
| 0x11 | 0x11 | 0x3C | Aircraft | **Airstrip land** (allied airport) |
| 0x14 | 0x14 | 0x35 | ??? | Repair-unit (SHP doesn't match other repair) |
| 0x1A | 0x1A | FUN_00731CC0 | base | Dock (enter as repair/refuel ally) |
| 0x1B | 0x1B | — | Infantry | Enter structure (repair state) |
| 0x1C | 0x1C | — | Infantry | Guard target |
| 0x1D | 0x1D | 0x21 | Infantry | Attack / harvest-refinery |
| 0x1E | 0x1E | — | Unit | **Deploy** (MCV → CY; unit deploys) |
| 0x1F | 0x1F | 0x1A | Unit/Infantry | Repair / guard-active |
| 0x20 | 0x20 | — | Infantry | **Garrison** civilian building (bunker) |
| 0x21 | 0x21 | 0x3C | DetermineAction | Sell-specific |
| 0x22 | 0x22 | 0x3C | DetermineAction | Sell-generic |
| 0x23 | 0x23 | 0x19 | Infantry | Garrison alternate |
| 0x24 | 0x24 | 0x1A | Infantry/Unit | **Low-bridge entry** |
| 0x25 | 0x25 | 0x39 | ??? | Deploy-variant |
| 0x26 | 0x26 | 0x32 | ??? | Select-area |
| 0x27/0x28 | | 0x3A | ??? | — |
| 0x29 | 0x29 | 0x2F | ??? | — |
| 0x2A-0x2F | | 0x3C | DetermineAction | Waypoint/chrono modes (6 variants) |
| 0x30 | 0x30 | 0x3C | DetermineAction | Select-waypoint |
| 0x33 | 0x33 | — | TechnoClass::OnCell | **Force-attack cursor** (ctrl on cell with targets) |
| 0x34 | 0x34 | 0x1B | base | Enter-T (generic) |
| 0x35 | 0x35 | 0x26 | Infantry | Enter grinder (disqualified) |
| 0x36 | 0x36 | — | Infantry | Enter grinder (qualified) |
| 0x37 | 0x37 | 0x34 | base | **Stop/halt** — self-click idle veteran |
| 0x38 | 0x38 | 0x34 | ??? | — |
| 0x39 | 0x39 | 0x3B | Infantry | Engineer-specific disqualified |
| 0x3A | 0x3A | — | BandBox_LeftUp | — |
| 0x3B | 0x3B | — | Infantry/Unit | **No-move** cursor (no weapon, no path) |
| 0x3C | 0x3C | 0x4E | DetermineAction | Place-building ghost |
| 0x3D | 0x3D | — | BandBox_LeftUp | Waypoint drop |
| 0x3E/0x3F | 0x3E | FUN_00731CB0 | FUN_0070F0B0 | Veteran attack variants |
| 0x40 | 0x40 | — | Infantry | **Special attack** via weapon flag 0x139 |
| 0x41-0x48 | | various | BandBox_LeftUp | Additional order codes |
| 0x47 | 0x47 | 0x52 | Infantry | **Special attack building** via weapon flag 0x13A |

**Key distinctions**:
- **1 vs 5**: both are "attack". `1` is the generic attack fallback
  (no weapon range check). `5` is "attack with best weapon selected"
  (GetBestWeapon returned a usable choice).
- **1 vs 0x33**: `0x33` is the force-attack cursor specifically for
  CLICKING ON A CELL with units in it while holding ctrl. `1` is
  target-click attack.
- **4 vs 0x37**: `4` is single-unit self-select. `0x37` is
  single-unit self-click while idle AND veteran — triggers "halt"
  voice line (stops all orders).

**Unconfirmed codes** (seen in cursor switch but not in What_Action
decompilation): `0x14`, `0x25`-`0x29`, `0x2A`-`0x2F`, `0x30`, `0x38`,
`0x3A`, `0x41`-`0x48`. These are likely injected by DetermineAction's
mode-flag branches or by BandBox_LeftUp's action-code parameter
pathway directly, without going through a What_Action virtual.

---

## 5. Decision-predicate flag inventory

Action-code selection is gated by a large set of flag bits on
TypeClass structs. Offsets observed in this pass:

### TechnoTypeClass flags (param_1[0x1B0] or param_1[0x148] depending on subclass)

| Offset | Flag | Effect on action codes |
|--------|------|------------------------|
| +0x0692 | (on TypeClass) | `param_1[0xAA] != 0 &&  !TypeClass+0x692` → short-circuit 0 (something is disabled) |
| +0x067C | MovementZone | used for pathing decision (`0x3` = Fly, clamped comparisons) |
| +0x06AC | | gates harvest-vs-move fallback (UnitClass OnCell/OnObject, line ~0x140427) |
| +0x06C9 | NeedsEngineer on unit | gates `4` override when selecting unit (Infantry self-click) |
| +0x0C8D | IgnoreFog-ish | FootClass shroud override: `1` (attack) vs `2` (move) into shroud |
| +0x0C94 | | InfantryClass: target of disguised-spy behavior — returns `7` (select) |
| +0x0CCC | | InfantryClass: target is enter-eligible building — returns `9` |
| +0x0D2C | | Base moves despite path count (returns `2` force-move) |
| +0x0D6A | | Unit skips repair dispatch logic |
| +0x0D94 | | Infantry: forces cell-click to `0` unless shroud-cell rules apply |
| +0x0DFC | | Aircraft: gates allied-airport logic |
| +0x0E0D | | Aircraft: if set, converts `0x1A` dock result to `0` (no auto-dock) |
| +0x0E0E/+0x0E0F | **OreCell / GemCell miner flags** | Unit returns `6` (harvest) for ore vs gem overlays |
| +0x0E13 | | Harvester-gating — unit can't be given attack orders directly |
| +0x0EAE | | InfantryClass: weapon-0x22E-gated `0x35`/`0x36` split |
| +0x0EBE | Deployer flag | on Infantry: `5` → `0x10` conversion for engineer-capture |
| +0x0EC2 | | Infantry: triggers weapon ability 0xE check |
| +0x0EC3 | Engineer | Infantry: gates the entire "engineer" action branch (repair/capture/bridge) |
| +0x0EC4 | | Neutral disguise gate |
| +0x0EC6 | | Infantry: terrorist? Enemy-target attack returns 9 instead of 5 |
| +0x0EC8 | | Infantry: 4/0x1B/0x1C/0x1D/0x1E mission-state forces `0x1F` |

### BuildingTypeClass flags (on target + 0x148 for buildings)

| Offset | Flag | Effect on action codes |
|--------|------|------------------------|
| +0x1572 | **Grinding=yes** (TS-era inherited; also Hospital) | Infantry enter building → `9` (repair if damaged, `0x1C` otherwise) |
| +0x1576 | Capturable variant | Infantry enter → `9` / `0x1C` |
| +0x1577 | **NeedsEngineer doorway** | Engineer target → `0x10` (capture) or `0x47` (weapon 0x13A) |
| +0x157B | | Disguise-allowed target gate |
| +0x16A9 | Bridge repair hut | Unit hover → converts to `1` (attack) via path-valid check |
| +0x16AB | Repair hub | Similar to 16A9 for a different action |
| +0x16AD | **Bunker=yes** (Soylent bunker / standalone infirmary) | Infantry enter → `0x20` / `0x23` (garrison or alt) |
| +0x16B6 | Hospital (healing behavior) | Infantry returns `0x20` or `0x23` based on radar color |
| +0x16BD | | Aircraft: disables cell targeting — returns `0` |
| +0x16C1 | Infantry auto-heal entry | Infantry repair flow returns `3` when health low |
| +0x16C2 | Elite-only doorway | Similar to 16C1 but veteran-gated |
| +0x1701 | ImmuneToPsionics-ish | Blocks capture-sell flow (returns `5` instead of `0x10`) |

### WeaponType flags (on the WeaponTypeClass pointed at by the weapon slot)

**Correction (2026-04-24, Task 13):** Flags `+0x139` and `+0x13A` are
on **WeaponTypeClass**, NOT on WarheadTypeClass as this table originally
said. The dispatch chain is:
`param_1->vtable[0x2E4](target)` (GetBestWeaponSlot) →
`param_1->vtable[0x3F8](idx)` (GetWeaponAtSlot) →
`*piVar9` = `WeaponTypeClass*`, and `iVar8 + 0x139` / `iVar8 + 0x13A`
are byte reads on that WeaponType.
INI key names verified in `WeaponTypeClass::ReadINI` at `0x7721B8` /
`0x7721CC`. See `MAPCLASS_COMPLETE_DECODE.md` §J for the full trace.

| Offset | Owner | INI key | Default | Effect |
|--------|-------|---------|---------|--------|
| +0x0139 | **WeaponTypeClass** | `SabotageCursor` | `no` | Triggers `0x40` attack variant when infantry hovers an enemy infantry with this weapon |
| +0x013A | **WeaponTypeClass** | `MigAttackCursor` | `no` | Triggers `0x47` attack variant against structure with `NeedsEngineer=yes` (and not ImmuneToPsionics) |
| +0x014B | WarheadTypeClass | (not re-verified) | — | Attack-through-armor check bypass |
| +0x014C | WarheadTypeClass | (not re-verified) | — | Force-attack-while-disabled |
| +0x0157 | WarheadTypeClass | (not re-verified) | — | Disables panic / scatter reaction from attack |
| +0x0231 | WarheadTypeClass | (not re-verified) | — | "VeteranAttack alternate" — changes attack code progression |
| +0x0233 | WarheadTypeClass | (not re-verified) | — | Attack → converts code `5` to `2` (move) when weapon can't target |

### Rules flags (global g_RulesClass_Instance)

| Offset | Flag | Effect |
|--------|------|--------|
| +0x0FD4/+0xFD5 | `AlliesAllowedToHeal`? | Enable `0x37` (halt) on idle self-click during veteran |
| +0x0B40/+0xB4C | Special "can be attacked" list | Infantry target→7 if in list, 9 otherwise |
| +0x16F8 | `ConditionYellow` health ratio | Repair threshold for `3` repair action |
| +0x17F8 | `ConditionRed` health ratio | Secondary repair threshold (`9` vs `0x1C`) |

### Scenario flags

| Bit | Source | Effect |
|-----|--------|--------|
| 0x0800 | `g_ScenarioClass_Instance[0]` | Neutral house → target returns `9` except for specific list |
| 0x1000 | `g_ScenarioClass_Instance[0]` | **TS-era tag-targeting** — cell's tag type `0x14` / `0x06` enables modifier-held alternate actions |

The `0x1000` bit is Tiberian Sun legacy. Not set in stock YR.

---

## 6. Concrete examples — tracing the decision

### Example A: Select one Grizzly tank, click on hostile Rhino tank

```
DetermineAction(cell=rhino_cell, target=rhino, modifier=1):
   g_CurrentObjects_Count = 1, best = Grizzly (score 5)
   target != NULL → dispatch Grizzly->vtable[0x74](rhino, 0)
      → UnitClass::What_Action_OnObject(Grizzly, rhino, 0)
         → FootClass → TechnoClass base returns 5 (has weapon, enemy, armed)
         → Grizzly has weapon range > 0, target not in sight → 5
         → no shroud, no capture flag, no repair, not disguise → 5
         → not ally → skip ally-repair logic
         → return 5
   apply mode overrides — none active
   → final action = 5
   SetCursorFromAction(5) → cursor 0x14 (attack-best-weapon)
```

### Example B: Select one Engineer, click on damaged allied Barracks

```
DetermineAction(cell=barracks_cell, target=barracks, modifier=0):
   best = Engineer (score 4 — no weapon, mobile)
   target != NULL → InfantryClass::What_Action_OnObject(Engineer, barracks, 0)
      → FootClass → TechnoClass base returns 7 (select — target is ally)
      → TypeClass[0xEC3] set (Engineer flag) AND target is building AND ally
      → target[1].vtable[0x16C1] = 1 (has auto-heal entry flag)
      → HealthRatio(barracks) < Rules[0x16F8] (damaged) → return 3
   mode checks — none
   → final action = 3
   SetCursorFromAction(3) → cursor 0x19 (repair / enter-to-repair)
```

### Example C: Select one Chrono Miner, click on ore cell

```
DetermineAction(cell=ore_cell, target=NULL, modifier=0):
   best = Miner (score 4)
   target == NULL → UnitClass::What_Action_OnCell(Miner, ore_cell, 0, 0)
      → FootClass → TechnoClass base returns 1 (reachable cell, can move)
      → cell.type = 5 (ore) AND Miner.Type[+0xE0E] = 1 (OreMiner)
      → return 6 (harvest)
   → final action = 6
   SetCursorFromAction(6) → cursor ??? (not in the mapping — likely 0x17 or a
                                        miner-specific frame, not yet confirmed)
```

### Example D: Select one Construction Yard (hotkey), click on own CY

```
DetermineAction(cell=cy_cell, target=cy, modifier=0):
   best = CY (score 3 — it's a building)
   target != NULL AND target == best AND g_CurrentObjects_Count == 1
   → TechnoClass::What_Action_OnObject(CY, CY, 0)
      → type != 6? no, it IS type 6 → fall-through
      → target == param_1 AND selection==1 AND veteran (RulesClass.AllowVeteran) → return 0x37
      → OR if MCV-convertable AND g_GameMode==0 (solo) AND type==1 (Unit) → return 4
   → action = 0x37 (halt) or 4 (self-select) depending on veteran state
```

---

## 7. Integration points

| Caller | Consumed action | Purpose |
|--------|-----------------|---------|
| `SetCursorFromAction` (0x4AAE90) | action → cursor SHP frame | Cursor icon update per hover tick |
| `BandBox_LeftUp` (0x4AB9B0) | action → command packet | Click → order dispatch (see BANDBOX doc) |
| `DisplayClass::Dispatch` (0x6922E0) | wrapper | Top-level input flow |
| `CommandBar_Dispatch` | — | Command bar button → UI-mode-flag setters |

### Callee breakdown (What_Action's own callees)

Important helpers, not decompiled in full:
- `TechnoClass::GetWeaponRange(obj, -1)` — returns `< 0` if no
  weapon, `>= 0` range otherwise. Used by every subclass to check
  "does this unit even have a weapon?"
- `TechnoClass::HasWeaponAbility(0xE)` — generic warhead-capability
  check (bit 0xE = some specific ability, probably
  AntiArmor/AntiBuilding)
- `HouseClass::IsHumanPlayer()` — most action codes gated behind
  "only show for human player"
- `HouseClass::Is_Ally_ByObject(target)` — friend/foe check
- `CellClass::IsLowBridgeCell()` — triggers `0x24` bridge-entry
  cursor
- `ObjectClass::GetHealthRatio()` — returns health fraction as
  double, compared against rules thresholds for repair decision

---

## 8. Current Rust implementation status

Updates the existing DisplayClass report's Rust status table.

| gamemd feature | Rust location | Status |
|----------------|---------------|--------|
| SelectBestObjectForAction priority scoring | `src/app_cursor.rs::select_best_for_action` | Implemented — matches the 5→4→3→1→0 priority ladder. Distance tie-break present. |
| TechnoClass::What_Action_OnObject base | `src/app_cursor.rs::capability_cursor_for_hover` | Partial — handles deploy/sabotage/engineer/occupier/friendly split, but not the full base branch |
| FootClass shroud adjustment | `src/app_cursor.rs` | Not confirmed — Rust shroud model differs from gamemd's |
| InfantryClass engineer/capture/repair | `src/app_context_order.rs::try_queue_context_order_at_screen_point` (CaptureBuilding, EngineerRepair) | Partial — code-path exists, not all flag gates (0x16C1/16C2/1572/1576) replicated |
| InfantryClass garrison | `src/app_context_order.rs` (EnterTransport) | Present but triggered off Occupier flag, not gamemd's 0x16AD bunker flag |
| UnitClass deploy | `src/app_context_order.rs` (DeployMcv) | Implemented |
| UnitClass harvest (action 6) | `src/app_context_order.rs` (HarvestCell) | Implemented (ore clicks), but not the `+0xE0E/+0xE0F` type-flag gate |
| AircraftClass airstrip dock (action 0x11) | — | Not implemented |
| Low-bridge entry (action 0x24) | — | Not implemented (bridge state machine present but cursor/action path missing) |
| Veteran halt self-click (action 0x37) | — | Not implemented |
| Force-attack cursor (action 0x33) | — | Not implemented — Rust has ctrl-attack but as a modifier on existing attack, not a distinct action code |
| `0x40`/`0x47` weapon-flag specific attacks | — | Not implemented |

### Parity-critical gaps

1. **Aircraft airstrip dock (0x11)** — clicking on own airstrip with
   an allied aircraft selected should show a distinct cursor. Rust
   currently shows generic select/move.

2. **Low-bridge entry (0x24)** — a specific cursor appears when
   clicking a low-bridge cell that a ground unit can enter. Missing.

3. **Halt/stop idle veteran (0x37)** — single-unit self-click on a
   veteran stops all orders AND plays a voice line. This is the
   "Yes, sir!" veteran acknowledge — a parity-feel detail.

4. **Force-attack on cell (0x33)** — ctrl-clicking an empty cell
   with enemy units in it should ground-target-fire. The cursor is
   distinct from normal ctrl-attack on a single target (0x08).

5. **Garrison flag distinction** — Rust uses `Occupier=yes`; gamemd
   uses both `Bunker=yes` (0x16AD) for auto-garrison and `Hospital`
   / `Grinding` (0x16B6/0x1572) for entry-to-heal/sell. The cursor
   changes based on which flag is set on the target.

---

## 9. Open questions

1. **Action codes 0x14, 0x25, 0x26, 0x27-0x29, 0x38, 0x3A, 0x41-0x48**
   are never returned from any What_Action override I decompiled,
   but appear in SetCursorFromAction's switch and in BandBox_LeftUp's
   dispatch. They must be injected either:
   - By DetermineAction's mode-flag branches (sell/chrono/deploy) —
     most likely
   - By BandBox_LeftUp directly (order-code parameter distinct from
     action-code)

   Confirming requires decompiling the remaining mode-flag branches
   of DetermineAction and BandBox_LeftUp in full. Left as a
   follow-up.

2. **The `0x1000` scenario flag** — "tag-targeting" TS-era behavior
   where a cell can have a tag that enables alternate click
   actions. Not set in stock YR. Can be safely ignored unless
   campaign missions turn out to set it.

3. **BuildingClass does NOT override What_Action**. A selected
   building falls through to TechnoClass base, which returns `0x37`
   (halt) on self-click or `7` (select) on click elsewhere. Other
   cursors for buildings (sell, upgrade-accept) come from
   DetermineAction's mode flags, not the virtual dispatch.

4. **TechnoClass base at FUN_00700600** is unnamed in Ghidra but
   serves as the OnCell base. Proposed rename: `TechnoClass::What_Action_OnCell`.

5. ~~**The weapon-specific 0x40/0x47 codes** correspond to Warhead
   flags `0x139` and `0x13A`.~~
   → **Resolved (2026-04-24, Task 13):** These flags live on
   **WeaponTypeClass**, not WarheadTypeClass, and correspond to
   INI keys `SabotageCursor` (`+0x139` → action `0x40`) and
   `MigAttackCursor` (`+0x13A` → action `0x47`). Both default to
   `no`. See `MAPCLASS_COMPLETE_DECODE.md` §J for the trace through
   `WeaponTypeClass::ReadINI` at `0x772080` and attribution evidence.
   §5 of this report above has been updated with the correction.

6. **FUN_0070F0B0 veteran substitution** — converts action 1→0x3E,
   5→0x3F when hovering a veteran unit. Is the threshold for
   "veteran" the TargetObj's veterancy or the Selected Unit's? The
   function name and context suggest Selected Unit, but worth
   confirming.

7. **SelectBestObjectForAction vtable slots**:
   - vtable[0x37C] returns "cannot act" bool
   - vtable[0x2AC] returns "is actionable techno" bool
   - vtable[0x2C] returns AbstractType (1=Unit, 2=Infantry, 3=Aircraft, 6=Building, 15=Terrain, etc.)
   Full vtable[N] enumeration is a separate investigation.

---

## 10. Sources

### Newly decompiled (7 functions)

- `0x005353D0` SelectBestObjectForAction
- `0x006FFEC0` TechnoClass::What_Action_OnObject
- `0x00700600` TechnoClass::What_Action_OnCell (base, unnamed)
- `0x004DDDE0` FootClass::What_Action_OnCell
- `0x004DDED0` FootClass::What_Action_OnObject
- `0x0051F800` InfantryClass::What_Action_OnCell
- `0x0051E3B0` InfantryClass::What_Action_OnObject
- `0x007404B0` UnitClass::What_Action_OnCell
- `0x0073FD50` UnitClass::What_Action_OnObject
- `0x00417CC0` AircraftClass::What_Action

### Function-search queries
- `search_functions("What_Action")` → 8 labeled functions

### Referenced docs
- `DISPLAYCLASS_GHIDRA_REPORT.md` (primary — this extends §5.1)
- `DISPLAYCLASS_BANDBOX_AND_MI_CORRECTION_GHIDRA_REPORT.md` (BandBox_LeftUp action codes)
- `BUILDINGCLASS_SPECIAL_BUILDINGS_GHIDRA_REPORT.md` (flags 0x16AD, 0x1572, 0x1577 context)
- `DISGUISE_SYSTEM_GHIDRA_REPORT.md` (0x1D4 disguise virtual; 0xC94 disguise-block flag)
- `SELECTION_SYSTEM_GHIDRA_REPORT.md` (g_CurrentObjects model)
- `AIRCRAFTCLASS_GHIDRA_REPORT.md` (§36 — action code 0x11 airstrip)

### INI keys referenced (confirmed active in YR)
- `Grinding=yes` — flag 0x1572
- `Bunker=yes` — flag 0x16AD
- `NeedsEngineer=yes` — flag 0x1577
- `BridgeRepairHut=yes` — flag 0x16A9
- `Capturable=yes` — flag 0x1576
- `Crushable=yes` — influences 5→1 conversion in UnitClass via CanCrushCheck
- `Engineer=yes` — flag 0xEC3
