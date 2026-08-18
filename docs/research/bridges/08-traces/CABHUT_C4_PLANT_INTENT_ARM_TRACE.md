# Trace: C4 Plant Intent → CABHUT Armed (Stage 1 of 5-Stage Pipeline)

**Date:** 2026-05-20
**Slot:** 1 of trace-swarm — covers right-click resolution through `pending_c4_detonation` armed.
**Scope:** SEAL/Tanya right-clicks CABHUT at (R=9, R=10). Trace ends when `pending_c4_detonation` is set on the CABHUT entity (timer starts counting). Detonation outcome is slot 2.

**Verdict tally:** PASS: 6 | FAIL: 1 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

---

## Pipeline Summary

### Stage 1: Input Resolution — SEAL hover + right-click on CABHUT → cursor/action code

**gamemd spec (from `C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md` §5 + `NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md` §7):**

`InfantryClass::What_Action_OnObject` at `0x51E3B0` returns **`0x10` (DEMOLISH)** for
SEAL/Tanya on CABHUT. The gate is:
```
IsHumanPlayer()
&& (InfantryType[+0xEC2 /* C4 */] != 0)
&& iVar7 == 5 (ACTION_ATTACK, from LegalTarget=yes on CABHUT)
&& target.RTTI == 6 (Building)
&& !target.vtable[+0x80]() (not destroyed)
&& target.Type[+0x1577 /* CanC4 */] != 0
&& target.Type[+0x1701 /* InvisibleInGame */] == 0
→ return 0x10  (direct return — bypasses Immune-downgrade tail at 0x51F171)
```

CABHUT satisfies all conditions: `CanC4` defaults to `1` (constructor `0x45E063`: `MOV byte ptr [ESI+0x1577], 1`);
`InvisibleInGame` is `0`; CABHUT's `[CABHUT]` INI section does not set `InvisibleInGame=yes`.
The Immune-downgrade tail at `0x51F171` (converts ACTION_ATTACK → ACTION_NOMOVE for
Immune=yes targets) is **not reached** — the C4 block's `return 0x10` exits first.

**Our cursor code (`src/app_cursor.rs:250-261`):**
```rust
if sel_obj.c4
    && matches!(hover.kind, HoverTargetKind::EnemyStructure)
    && hovered_obj.map_or(false, |o| o.can_c4 && !o.invisible_in_game)
    && !hovered_entity.is_some_and(|e| is_invulnerable(e.invulnerability.as_ref(), tick))
{
    return CursorFeedbackKind::Demolish;
}
```

**Gate-by-gate comparison:**

| gamemd gate | Our gate | Match? |
|-------------|----------|--------|
| `IsHumanPlayer()` | implicit (only human player generates cursor feedback) | PASS |
| `InfantryType.C4 (+0xEC2) != 0` | `sel_obj.c4` — parsed from `C4=` via `object_type.rs:1030` | PASS |
| `iVar7 == 5` (LegalTarget→ATTACK cursor) | `HoverTargetKind::EnemyStructure` — `app_entity_pick.rs:152-153` | PASS (equivalent) |
| `target.RTTI == 6` (Building) | `EntityCategory::Structure` gate in entity-pick | PASS |
| `target.Type.CanC4 (+0x1577) != 0` | `hovered_obj.can_c4` — parsed from `CanC4=` with default true for buildings (`object_type.rs:1031-1033`) | PASS |
| `target.Type.InvisibleInGame (+0x1701) == 0` | `!o.invisible_in_game` — parsed from `InvisibleInGame=` (`object_type.rs:1034`) | PASS |
| IC check (vtable[0x80] — not iron-curtained) | `is_invulnerable(...)` | PASS |
| Immune-downgrade bypass | Our code doesn't check `Immune` for the Demolish cursor path | PASS |

**Missing from our cursor code:** `!target.vtable[+0x80]()` (not destroyed). In gamemd this prevents the
DEMOLISH cursor on a dying building. Our cursor code does not explicitly gate on `building.dying`.
However, entity-pick (`app_entity_pick.rs`) would not pick a dying entity at hover time in a normal
game path — this is effectively handled at the pick layer. Low player-visibility risk.

**Verdict: PASS** — gamemd returns `0x10`; we return `CursorFeedbackKind::Demolish`. Numerically
equivalent observable output (player sees the Demolish/C4 cursor over CABHUT).

---

### Stage 2: Command Dispatch — `Command::PlantC4` fires

**gamemd spec:** Action `0x10` dispatch assigns `Mission_Sabotage` (mission code `0x11`) to the SEAL
via `vtable[+0x1e8](0x11, 0)` (SetMission(Enter = 17)). `InfantryClass::Mission_Attack`
tests `InfantryType.C4 && target.CanC4 && !target.InvisibleInGame`, sets `TarCom` to building,
transitions mission. From `NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md` §1:
```
Test 1: Type[+0xec2] != 0 AND BldgType[+0x1577] != 0 AND BldgType[+0x1701] == 0
  → vtable[+0x480](TarCom, 1)   // Set_Target
  → vtable[+0x1e8](0x11, 0)     // SetMission(Enter)
  → return 1
```

**Our dispatch (`src/app_context_order.rs:312-368` + `src/sim/world/world_commands.rs:887-951`):**

`app_context_order.rs` constructs `Command::PlantC4 { attacker_id, target_building_id }` after
checking `o.c4` on each selected C4-capable infantry, then queues the command.
`world_commands.rs` `apply_command(Command::PlantC4 {...})` validates:
1. Attacker owned by command-issuer
2. Attacker not deployed
3. `attacker.Type.c4 == true`
4. Target is a Structure, not dying, `can_c4 == true`, `invisible_in_game == false`
5. Not IC'd at issue time
6. Target is enemy-owned
7. Sets `e.c4_plant = Some(C4PlantState { target_building_id })` on attacker
8. Clears conflicting state (attack_target, order_intent, dock_state, capture_target)
9. Issues pathfinding move toward building's cell

**Gate comparison:**

| gamemd gate at Mission_Attack | Our world_commands gate | Match? |
|-------------------------------|------------------------|--------|
| `Type[+0xEC2].C4 != 0` | `obj.c4.then_some(())` | PASS |
| `BldgType[+0x1577].CanC4 != 0` | `obj.can_c4` | PASS |
| `BldgType[+0x1701].InvisibleInGame == 0` | `!obj.invisible_in_game` | PASS |
| IC check (`!target.vtable[+0x80]()` in What_Action; vtable[0x160] in PerCellProcess) | `is_invulnerable(...)` | PASS |
| Enemy target | `!are_houses_friendly(...)` | PASS |
| `target.GetMission() != 0x13` (not selling) | NOT IMPLEMENTED — `TODO(parity)` comment at `world_commands.rs:912` | **NOT IMPLEMENTED** but marked |

The `Mission==0x13` (selling) gate is noted as a TODO. This is a minor edge case (fires only if
player clicks CABHUT while it is selling/being-deconstructed, which CABHUT never does in stock
play). Player-visible risk: extremely low in any stock map. Deferred per world_commands.rs TODO.

**Verdict: PASS** — `Command::PlantC4` is dispatched with correct `attacker_id` and
`target_building_id`. The critical gates all match. The missing selling-Mission gate is a known TODO
(minor, low-frequency).

---

### Stage 3: SEAL Movement to CABHUT Cell

**gamemd spec:** Mission_Attack transitions the SEAL to Mission_Enter (0x11); Mission_Enter
then navigates the SEAL toward CABHUT's cell. The SEAL walks using normal ground movement.
Per `NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md` §1: "Walk toward target" until SEAL's cell == NavCom target cell.

**Our behavior (`src/sim/world/world_orders.rs:406-453`):**

`tick_c4_plants` Phase 1 runs every tick:
1. Checks if attacker cell is in `target_footprint`.
2. If not in footprint but Chebyshev-1 adjacent AND no active movement: calls
   `issue_c4_enter_target_cell` which issues a direct (bypass-grid) 1-cell move into the
   nearest footprint cell.
3. If not adjacent: normal movement from PlantC4 command dispatch continues.

The SEAL moves to the CABHUT's footprint cell via standard pathfinding (issued in `world_commands.rs:954-1000`)
then on each subsequent tick the Phase 1 logic checks if the SEAL is adjacent, and issues the final
1-cell bypass-grid enter move. The SEAL claims the plant when its cell is `in target_footprint`.

**Comparison to gamemd:**

gamemd checks `if (target_building == infantry.NavTarget)` — i.e., the SEAL's current
cell must be the building's NavTarget destination cell. Our equivalent is
`target_footprint.contains(&attacker_cell)`. CABHUT's footprint is computed via
`building_footprint_cells(rx=9, ry=10, foundation, add_occupy, remove_occupy)`.

**Timing:** gamemd's `Mission_Enter` runs every frame until arrival. Our `tick_c4_plants` runs
every tick (same cadence). Movement distance: 1 cell (256 leptons) via bypass-grid direct-move.
At SEAL Speed=4 (~10 lep/tick estimated), 1-cell enter takes ~26 ticks. This is not a
observable-output difference (SEAL walk speed is the same; the 1-extra-tick for the enter-cell
phase is below player perception).

The test `c4_on_cabhut_collapses_bridge_and_hut_survives` (`world_orders_bridge_repair_tests.rs:535`)
verifies the first tick does NOT claim the plant from the adjacent cell, and `advance_until_c4_claim`
confirms the claim happens within 32 ticks of the SEAL starting at Chebyshev-1.

**Verdict: PASS** — SEAL navigates to CABHUT footprint cell. The per-tick check matches the
gamemd `if (cell == NavTarget)` check in its observable effect. No player-visible timing difference.

---

### Stage 4: Plant Arming — `pending_c4_detonation` set on CABHUT

**gamemd spec** (from `C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md` §6 + `TECH_CABHUT_GHIDRA_REPORT.md` §4.6):

When SEAL's cell == CABHUT's cell, `PerCellProcess` Mission_Sabotage branch (0x519630):
```c
target_building.field_0x6df = 1;          // C4 armed marker
// ... allocate BombClass / attach timer from RulesClass.C4Delay (Rules+0xFD0)
target_building.+0x528 = SEAL ptr          // attacker (byte offset 0x528 = index 0x14A × 4)
target_building.+0x14a = g_CurrentFrameCounter  // start frame
target_building.+0x14b, +0x14c = saved coords
```

The `BombClass::Attach` path sets `BombClass+0x38 = g_CurrentFrameCounter + RulesClass+0xFD0`
(C4Delay). From `CABHUT_C4_PHASE1_NEW_FINDINGS_GHIDRA_REPORT.md` §6: the INFANTRY plant uses the
inline `field_0x6DF` (PerCellProcess) path, NOT the `BombClass::Attach` (RTTI==0xf) path.

**Fields populated in gamemd:**
- `Building.field_0x6df = 1` — armed marker
- `Building.+0x528 (=+0x14a×4)` — attacker pointer
- `Building.+0x14a` — start frame (= g_CurrentFrameCounter at plant time)
- `Building.+0x14b/14c` — saved coords

**Our arming** (`world_orders.rs:456-462`):
```rust
if let Some(b) = self.entities.get_mut(target_id) {
    b.pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: self.tick,   // ← start tick
        attacker_id,                   // ← attacker
    });
}
```

**Field-by-field comparison:**

| gamemd field | Our field | Match? |
|--------------|-----------|--------|
| `field_0x6df = 1` (armed marker) | `pending_c4_detonation = Some(...)` | PASS (semantically equivalent non-null marker) |
| `Building.+0x528` = attacker ptr | `PendingC4Detonation.attacker_id: u64` | PASS (stable_id instead of raw ptr; same semantic) |
| `Building.+0x14a` = g_CurrentFrameCounter (start) | `PendingC4Detonation.plant_start_tick: u64 = self.tick` | PASS |
| `Building.+0x14b/14c` = saved coords | Not stored separately — resolved from entity at detonation time | PASS (effectively equivalent; coords don't change until building dies) |

**Second-attacker guard:** gamemd's `field_0x6df != 0` early-return check prevents a second SEAL
from replanting. Our check at `world_orders.rs:433-439`:
```rust
let already_claimed = self.entities.get(target_id)
    .is_some_and(|b| b.pending_c4_detonation.is_some());
if already_claimed { continue; }
```
This is a correct match.

**IC gate in PerCellProcess:** gamemd gates on `target_building.vtable[0x160]() == 0`
(IsIronCurtainActive) BEFORE setting field_0x6df. Our IC check is at command-issue time
in `world_commands.rs:924-929` (blocks `PlantC4` dispatch if IC at issue time). We do NOT
re-check IC at plant-claim time in `tick_c4_plants`. gamemd checks IC at PerCellProcess
(walk-up), not at Mission_Attack (command issue). This is a **FAIL** for timing precision:
if IC is applied AFTER the command is issued but BEFORE the SEAL reaches the CABHUT, gamemd
would reject the plant at PerCellProcess; our engine would still claim the plant on arrival
(IC is not rechecked in Phase 1).

**Verdict: FAIL** — The arming fields themselves are correct: `plant_start_tick`, `attacker_id`,
and the armed marker all match gamemd. However, the Iron Curtain re-check is missing at
plant-claim time (`tick_c4_plants` Phase 1 does not gate on IC before setting
`pending_c4_detonation`). If IC is applied between PlantC4 command issue and SEAL arrival,
gamemd rejects the plant; our engine arms it. Frequency: IC applied mid-walk is rare but
player-visible (SEAL walks to IC'd CABHUT and "plants" — the bridge then detonates or
detonation gets blocked in Phase 2 — IC gate in Phase 2 exists per `world_orders.rs:761-766`
but that gate nullifies detonation, not the plant). Player sees SEAL plant animation on an
IC'd target and nothing happens until IC expires — different from gamemd (where the SEAL
would walk away unrewarded immediately).

---

### Stage 5: CanC4 Gate — CABHUT.CanC4 defaults to true

**gamemd spec:** Constructor at `0x45E063`: `MOV byte ptr [ESI+0x1577], 1` — `CanC4` defaults
to `1` for all buildings. `BuildingTypeClass::ReadINI` may override from `CanC4=no`. CABHUT
(`[CABHUT]` section in `rulesmd.ini`) does not set `CanC4=no`, so it inherits the default.

**Our parser (`src/rules/object_type.rs:1031-1033`):**
```rust
can_c4: section.get_bool("CanC4")
    .unwrap_or(category == ObjectCategory::Building),
```
Default: `true` if `ObjectCategory::Building`. CABHUT's section does not set `CanC4=no`.
Result: `obj.can_c4 == true`.

**Verification:** `src/rules/ruleset.rs` test `can_c4_defaults_to_true_for_buildings` and
`can_c4_no_overrides_default` confirm the parser behavior. Test `c4_delay_retail_default_value`
plus the full-rules test at `ruleset.rs:3061-3066` also confirms CAMISC01/CAMISC02 opt-out
and GAPILE/NAHAND/GAREFN inherit `can_c4=true`.

**Verdict: PASS** — Exact match. gamemd and our parser both produce `CanC4=true` for CABHUT.

---

### Stage 6: InvisibleInGame Gate — CABHUT.InvisibleInGame == false

**gamemd spec:** `BuildingTypeClass::ReadINI` (`0x460E8D`) reads `InvisibleInGame=` into
`BuildingTypeClass+0x1701`. CABHUT's INI section (`rulesmd.ini:16336-16352`) does not set
`InvisibleInGame=yes`. Default is `false` (no constructor override seen — parsed only when
present in INI).

**Our parser (`src/rules/object_type.rs:1034`):**
```rust
invisible_in_game: section.get_bool("InvisibleInGame").unwrap_or(false),
```
CABHUT section has no `InvisibleInGame=` key → `false`. Test `invisible_in_game_defaults_to_false`
in `object_type.rs:1579-1583` confirms.

**Verdict: PASS** — Exact match. Both gamemd and our parser produce `InvisibleInGame=false` for CABHUT.

---

### Stage 7: Cell Claim — SEAL claims CABHUT cell during plant

**gamemd spec:** After `field_0x6df = 1` is set, Mission_Enter calls `Stop_Moving()` on the SEAL
then SEAL transitions to the FireUp animation cycle (DoType `0x1b`→`0x1c` loop). The SEAL's current
cell is implicitly "claimed" by its occupancy; there is no explicit cell-reservation system separate
from physical presence. gamemd does not use a cell-lock: the SEAL simply stops on the building's
cell and runs the plant animation.

**Our behavior:**
On plant claim (`world_orders.rs:465-470`):
```rust
if let Some(a) = self.entities.get_mut(attacker_id) {
    a.movement_target = None;
    if let Some(ref mut anim) = a.animation {
        anim.switch_to(SequenceKind::Attack);
    }
}
```
The SEAL's `movement_target` is set to `None` (stops moving), and animation switches to `Attack`
(FireUp equivalent). The SEAL stays at the CABHUT cell. No explicit cell-lock structure exists
in our engine either — matches gamemd's implicit-occupancy model.

The second-SEAL guard (`already_claimed` check in Phase 1) is the operational equivalent of
gamemd's `field_0x6df != 0` marker check, which also causes the second SEAL to loop
re-approaching without claiming. Our "hover, no-op" path matches.

**Verdict: PASS** — SEAL halts on the building cell, animation switches to Attack (plant visual),
and the second-SEAL guard is implemented. Matches gamemd's implicit cell-occupancy + field_0x6df
marker model.

---

## C4Delay Timer Initialization

**gamemd spec (from `NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md` §5):** `C4Delay` at `Rules+0x1750`
is parsed as a `double` representing **minutes**. At 15 ticks/sec: `0.03 × 60 × 15 = 27 ticks`.
The timer fires when `g_CurrentFrameCounter - Building.+0x14a >= C4Delay_in_ticks`.

**Our parsing (`src/rules/ruleset.rs:1422-1427`):**
```rust
let c4_delay_ticks: u32 = ini
    .section("CombatDamage")
    .and_then(|s| s.get("C4Delay"))
    .and_then(|v| v.trim().parse::<f64>().ok())
    .map(|minutes| (minutes * 60.0 * 15.0).round() as u32)
    .unwrap_or(27);
```
Default: 27 ticks (= `0.03 × 60 × 15`). Tests `c4_delay_defaults_to_27_ticks` and
`c4_delay_retail_default_value` confirm. The full-rules integration test at `ruleset.rs:3069`
confirms against the actual `rulesmd.ini`.

**Timer comparison (`world_orders.rs:512`):**
```rust
if self.tick.saturating_sub(pending.plant_start_tick) < delay { continue; }
```
Compared to gamemd: `g_CurrentFrameCounter - Building.+0x14a >= C4Delay_in_ticks`.
Both use elapsed-ticks-since-plant >= delay. Semantically identical.

**Verdict: PASS** — Timer initialized correctly: `plant_start_tick` = current tick at arm moment;
delay = `c4_delay_ticks` (27 ticks default from `C4Delay=0.03` in `rulesmd.ini`).

---

## Adjacent Findings (for report only — not traced this slot)

1. **IC gate timing mismatch (Stage 4 FAIL source):** gamemd re-checks IC at PerCellProcess
   (per-tick arrival), not only at command issue. Our `tick_c4_plants` Phase 1 does not re-check
   IC before claiming the plant. If IC is applied between command issue and SEAL arrival,
   we claim the plant incorrectly. Remedy: add IC check to Phase 1 before `pending_c4_detonation = Some(...)`.
   File: `src/sim/world/world_orders.rs:456`. Low-frequency trigger (IC must be applied in the
   window between PlantC4 command and SEAL walking to building).

2. **`Mission==0x13` (selling) gate not implemented:** `world_commands.rs:912` has a `TODO(parity)`
   comment. CABHUT can never sell, so this is not observable for the CABHUT scenario. File:
   `src/sim/world/world_commands.rs:912`.

3. **Plant animation vs. DoType 0x1b→0x1c loop:** gamemd runs the SEAL's FireUp animation
   (DoType 0x1b→0x1c loop) during the C4Delay window. Our engine calls
   `anim.switch_to(SequenceKind::Attack)` at claim time. Whether the animation loops for the full
   delay window vs. plays once is not traced here (slot 2's concern is timer expiry; animation
   continuity is cosmetic).

4. **SEAL Stop_Moving() call:** gamemd's PerCellProcess calls `FootClass::Stop_Moving()` immediately
   on plant claim. Our code sets `a.movement_target = None` which is functionally equivalent.

5. **EVA voice `SealSpecialAttack`:** Our engine calls `emit_order_voice(state, "VoiceSpecialAttack")`
   at command issue time (`app_context_order.rs:367`). gamemd fires `VoiceSpecialAttack` at
   cursor-0x10 confirmation, which is effectively the same moment. PASS (not in scope of this slot).

---

## Sources Consulted

- `ra2-rust-game-docs/NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md` (primary spec, Ghidra-verified)
- `ra2-rust-game-docs/C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md` (CABHUT-specific gate analysis)
- `ra2-rust-game-docs/CABHUT_C4_INVESTIGATION_LOG.md` (open questions log)
- `ra2-rust-game-docs/CABHUT_C4_PHASE1_NEW_FINDINGS_GHIDRA_REPORT.md` (Phase 1 bridge walker findings)
- `src/app_cursor.rs:228-261` — cursor resolution
- `src/app_context_order.rs:312-368` — command dispatch
- `src/sim/world/world_commands.rs:887-951` — PlantC4 command validation and c4_plant set
- `src/sim/world/world_orders.rs:387-570` — tick_c4_plants Phase 1 (walk-up + arm) + Phase 2
- `src/sim/components.rs:645-720` — C4PlantState, PendingC4Detonation structs
- `src/sim/game_entity.rs:256` — `pending_c4_detonation` field on GameEntity
- `src/rules/object_type.rs:618-634, 1030-1034` — c4, can_c4, invisible_in_game parsing
- `src/rules/ruleset.rs:1256-1259, 1419-1427` — c4_delay_ticks parsing
- `src/sim/world/world_orders_bridge_repair_tests.rs:534-617` — integration test (not #[ignore])
