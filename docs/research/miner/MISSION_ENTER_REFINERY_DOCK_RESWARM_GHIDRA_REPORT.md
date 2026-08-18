# Mission_Enter Refinery Dock — Re-Swarm Re-Establishment Report

**Target:** stock ore/war-miner Enter-mission chain into `GAREFN`/`NAREFN`: mission dispatch,
radio acceptance, dock-coordinate computation, unload start.
**Trigger:** `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` flagged structural-RED on
2026-07-10 (wrong primary function / TS-legacy framed as YR).
**Investigation mode:** targeted re-verification of four specific assertions, all
re-derived live against `gamemd.exe` this session (not copied from prior docs, though prior
docs on this exact topic already existed and are cited as corroboration).
**Confidence:** HIGH — every load-bearing claim below has an inline citation to a
`decompile_function` / `disassemble_function` / `get_xrefs_to` / `get_function_callers` call
made in this session, plus a live INI read.
**Active in YR:** Yes for the whole chain — core ore-economy loop, every skirmish/multiplayer match.

---

## 1. The Four Flagged Assertions — Verdicts

| # | Assertion (from 2026-07-10 audit) | Verdict |
|---|---|---|
| 1 | Stock `GAREFN`/`NAREFN` are `DockUnload`/`Refinery` (NOT `WeaponsFactory`) | **CONFIRMED** |
| 2 | Mission 7 (Enter) is at `0x004D9290` | **CONFIRMED** |
| 3 | Refinery `GetDockCoord` bypasses `DockingOffset` | **CONFIRMED** |
| 4 | Normal stock unload does NOT run `BuildingClass::MissionRepairAndProduce` or call `UndockUnit`/radio-7 | **CONFIRMED** |

---

## 2. The Chain, Verified Live

### 2.1 Mission dispatch: mission 7 (Enter) → `FootClass::Mission_Enter @ 0x004D9290`

`get_function_by_address 0x004D9290` returns `FootClass__Mission_Enter`, body
`0x004D9290–0x004D949B`. `get_xrefs_to 0x004D9290` (this session) returns three vtable DATA
xrefs: `0x007E8ED4`, `0x007EB298`, `0x007F5EB0` — i.e. `FootClass::Mission_Enter` is bound,
unmodified, into three different class vtables (no per-class override for mission 7 on this
inheritance chain).

`decompile_function 0x005B3060` (`MissionClass::Mission_Dispatch`) shows the mission-ID
switch: `case 7: (**(code**)(*param_1 + 0x240))();` — i.e. mission ID 7 dispatches through
vtable slot `+0x240`. `0x007F5EB0` (one of the three vtable hits above) is the UnitClass
vtable; `0x007F5EB0 − 0x240 = 0x007F5C70` = UnitClass vtable base. This closes the loop:
mission 7 on a `UnitClass` instance (miner) dispatches to `0x004D9290`.
**Active in YR:** Yes — dispatched every tick a unit is in Mission_Enter.

`FootClass::Mission_Enter` itself is the **outer per-tick handler**: it sends
`radio(0xE)` (CAN_DOCK) to the destination each dispatch, manages loco piggyback
release/dock-queue dequeue, and returns a `Random(0,2)`-jittered delay
(`MissionClass::GetMissionTimerEntry` → `Math__ftol` → `Random__RandomRanged(0,2)`, matching
prior doc `REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md` §2.3). It is **not** the
function that performs the physical dock-cell arrival choreography — that is the separate
per-cell hook `UnitClass::PerCellProcess @ 0x00739EC0` (historically mislabeled
`UnitClass::Mission_Enter`; not re-verified in this slot, out of scope per slot-4 ownership).

### 2.2 Radio acceptance: `0x0E` (CAN_DOCK) then `0x15` (DOCK_NOW)

`decompile_function 0x0043C2D0` (`BuildingClass::Receive_Radio`), this session:

- **Case `0x0E`:** for `DockUnload`/`Weeder` buildings once queue-contained, computes the
  accepted target as building NW cell packed from vtable `+0x1B8`, offset by `(+3,+1)`
  cells (`CONCAT22(psVar11[1] + 1, *psVar11 + 3)`), sends it via message `0x12`, and only
  sends `0x18`/`0x16` when the unit reports `0x14` (already there). This is a hardcoded
  cell offset — it does not read `QueueingCell` or `DockingOffset%d`.
- **Case `0x15`:** first rejects mission `0x13` (under construction). Then, in priority
  order: `UnitAbsorb`(`+0x16AE`)/`InfantryAbsorb`(`+0x16AF`) → return 1 immediately;
  `UnitRepair`(`+0x16A9`)/`UnitReload`(`+0x16AA`)/`Hospital`(`+0x16C1`)/`Armory`(`+0x16C2`)
  → `building->field_0x6DD = 1`, `building->SetMission(0x14, 0)` (MissionRepairAndProduce),
  `sender->SetMission(0, 0)`, return 1; `Bunker`(`+0x16AB`) → same building-side `0x14`
  queue, no sender mission change, return 1; **`DockUnload`(`+0x16B3`)** → only
  `sender->SetMission(0x10, 0)`, return 1 — **no write to `building->field_0x6DD`, no
  `SetMission(0x14,...)` on the building.**

This is the exact branch structure the 07-10 audit asserted. Confirmed directly from the
live decompile this session (the function also carries a plate comment from a prior verified
session restating the same finding, corroborating rather than substituting for this
session's read).

### 2.3 Dock coordinate: `BuildingClass::GetDockCoord @ 0x00447B20` bypasses `DockingOffset` for refineries

`disassemble_function 0x00447B20`, this session. The function tests `TypeClass+0x16BC`
(Weeder) first, then `TypeClass+0x16BB` (Refinery) at `0x00447b9e`:

```
00447b9e: MOV CL,[EAX+0x16bb]      ; Refinery flag
00447ba4: TEST CL,CL
00447ba6: JZ 0x00447bc9            ; not refinery -> fall through to Bunker/Helipad/UnitRepair path
00447ba8: MOV EAX,[ESP+0x44]       ; sender arg
00447bb2: MOV ECX,ESI              ; ECX = THIS (the building)
00447bb4: CALL 0x005f6c80          ; FUN_005F6C80(this=building) -> building's own coord
00447bb9: MOV ECX,[EAX]            ; X
00447bbe: ADD ECX,0x80             ; X += 0x80 leptons (half-cell east)
00447bc4: JMP 0x00447ce1           ; -> straight to epilogue, writes output, RET
```

`decompile_function 0x005F6C80` shows it is a one-line COM-style wrapper:
`(**(code**)(*this + 0x48))(out)` — i.e. it calls the building's own `Get_Coord` vtable
slot `+0x48` on itself and copies the result out. No `DockingOffset` or `NumberOfDocks`
field is touched.

Critically, the `JMP 0x00447bc4 → 0x00447ce1` **skips over** the only code in this function
that reads `TypeClass+0x1780` (`NumberOfDocks`) and `TypeClass+0x1788` (`DockingOffset%d`
array) — that block lives at `0x00447d3d` onward and is reachable only from the
`Helipad`(`+0x16CB`)/`UnitRepair`(`+0x16A9`) branch starting at `0x00447cfc`, which the
Refinery branch's unconditional jump never reaches. **The `Refinery=yes` branch is
structurally incapable of reading `DockingOffset%d`, regardless of whether art.ini defines
one** — this is not merely "stock GAREFN/NAREFN happen not to set `DockingOffset0`," it is
a hard code-path bypass.

Net effect for a stock refinery: `GetDockCoord` returns `building.GetCoord() + (0x80, 0, 0)`
leptons — building center, offset half a cell east. This is a **different** coordinate from
the CAN_DOCK `NW+(3,1)` cell used by `Receive_Radio` case `0x0E` (§2.2); `GetDockCoord`
and the receiver's `0x0E` accepted-cell computation are two independently-hardcoded
values, neither derived from `DockingOffset%d`. (Corroborated by prior doc
`DOCKING_QUEUE_EXIT_REFERENCE_POINTS_GHIDRA_REPORT.md` §3.3/§3.4, itself Ghidra-sourced.)

**Active in YR:** Yes — `GetDockCoord` is called every time refinery dock-cell math is
needed (`UnitClass::PerCellProcess` dock-arrival check, per prior doc).

### 2.4 Unload start: `UnitClass::Mission_Deploy_Building @ 0x0073D630`, mission ID `0x10`

Radio `0x15`'s `DockUnload` branch (§2.2) calls `sender->SetMission(0x10, 0)`. Vtable-slot
arithmetic (this session) proves mission `0x10` on a `UnitClass` resolves to
`0x0073D630`:

- `get_xrefs_to 0x0073D630` → `0x007F5EAC [DATA]`.
- From §2.1, UnitClass vtable base = `0x007F5C70`.
- `0x007F5EAC − 0x007F5C70 = 0x23C`.
- `decompile_function 0x005B3060` case `0x10`: `(**(code**)(*param_1 + 0x23c))();` — exact
  match.

`decompile_function 0x0073D630`, this session (function carries a prior-session plate
comment corroborating the same read): entry branches on unit `+0x2E4` (`param_1[0xB9]`).
For the **normal stock zero-link path** (`+0x2E4 == 0`), the function runs a small state
machine on `param_1[0x2F]`:

- **State 0:** waits for facing-ready (`Rotate` state check via `+0x304`), then
  `param_1[0x2F] = 1`.
- **State 1:** waits on a deploy-anim flag (`+0x6AF`), then `param_1[0x2F] = 3`.
- **State 3 — this is where unload begins.** Re-finds the refinery via an adjacent-cell
  lookup (`g_refinery_unload_adjacent_lookup_dx/dy` seeded offset,
  `Look_up_building_in_cell`), then on a `RateTimer`-gated cadence calls
  `StorageClass__FindFirstNonEmptySlot` / `StorageClass__RemoveAmount` and
  `HouseClass__Add_Tiberium_Credits` (or `Add_Tiberium_To_Storage` for Weeders) — this is
  the literal ore-to-credit drain. Sets building anim slot 8 (`ProductionAnim`) when
  `Refinery=yes` and health-dependent frame selection is met. When storage empties,
  advances to state 4.
- **State 4:** clears `unit+0x6D1` (dock-active flag), waits only on building `+0x57C`
  (anim slot 8 non-null), then `SetMission(10, 0)` (re-queues Harvest) and returns to the
  Harvest loop. **No new destination, no `Force_Track`, no exit-cell move is installed.**

The reciprocal-link branch (`+0x2E4 != 0`, entry `else`) instead calls
`BuildingClass::ReleaseDockedHarvester @ 0x004595C0` — a **conditional** path, not the
stock zero-link ore-delivery exit (matches prior doc
`DOCKING_QUEUE_EXIT_REFERENCE_POINTS_GHIDRA_REPORT.md` §3.6).

**Active in YR:** Yes — every stock CMIN/HARV → GAREFN/NAREFN delivery.

### 2.5 `MissionRepairAndProduce` and `UndockUnit`/radio-7 are NOT in the normal chain

- `decompile_function 0x0043C2D0` case `0x15` (§2.2): the `DockUnload` branch never
  executes `building->SetMission(0x14, 0)` — that call only exists in the
  UnitRepair/UnitReload/Hospital/Armory/Bunker branches above it, none of which stock
  `GAREFN`/`NAREFN` set (confirmed §3 INI read below). `BuildingClass::MissionRepairAndProduce`
  is at `0x0044B780`; it is simply never reached for a stock DockUnload/Refinery-only
  building.
- `get_function_callers 0x004593A0` (`BuildingClass::UndockUnit`), this session, returns
  exactly three callers: `BuildingClass::Sell (0x00449C30 region)`,
  `BuildingClass::ReceiveDamage`, `TemporalClass::Update` — sell, destroy/damage, and
  chrono-wipe interrupts. **Normal cargo-empty exit (§2.4 state 4) never calls
  `UndockUnit`.**
- Radio `7` (DOCKING_COMPLETE): scanned across `FootClass::Mission_Enter` (§2.1),
  `UnitClass::Mission_Deploy_Building` (§2.4), `BuildingClass::ReleaseDockedHarvester`, and
  `BuildingClass::UndockUnit` — no `PUSH 0x7` before any `Transmit_Radio` vtable call
  (`+0x274`/`+0x278`/`+0x27C`) appears in any of them (cross-checked against
  `REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md` §5.4, which performed the
  instruction-level scan of the latter three). Since `MissionRepairAndProduce` — the one
  remaining candidate sender — is itself unreachable by stock refineries, radio 7 is moot
  for the stock zero-link chain regardless of where it is ultimately sent from.

---

## 3. INI Evidence

Live read this session, `ini/rulesmd.ini`:

| Key | GAREFN | NAREFN |
|---|---|---|
| `DockUnload=yes` | line 11726 | line 12519 |
| `Refinery=yes` | line 11727 | line 12520 |
| `NumberOfDocks=1` | line 11729 | line 12521 |
| `WeaponsFactory` | **absent** (grep of full block 11722–11769) | **absent** (grep of full block 12515–12560) |

Block boundaries confirmed by locating the next `[Section]` header after each
(`[GAWEAP]` at 11770, `[NAWEAP]` at 12561). No `WeaponsFactory=` line exists anywhere
inside either refinery's block.

---

## 4. A Label Trap Found In Passing (Do Not Reuse)

While resolving the UnitClass vtable slots in §2.1/§2.4, the same arithmetic exposes a
pre-existing mislabel risk: `get_xrefs_to` on `0x00740EF0` (Ghidra label
`UnitClass__Mission_Unload`, per prior doc `RADIO_REFINERY_DOCK_TS_LEGACY_AND_CONTEXT_GHIDRA_REPORT.md`
§2.6) shows it bound at vtable `0x007F5EBC` = base `0x007F5C70 + 0x24C`. Per
`MissionClass::Mission_Dispatch`'s own case table (§2.1), vtable `+0x24C` is the slot for
mission ID **`0x14`** ("Repair" family — same slot `BuildingClass` overrides with
`MissionRepairAndProduce`), **not** mission ID `0x10`. The Ghidra label `Mission_Unload`
is therefore misleading about which mission ID actually reaches it via vtable dispatch —
do not assume "Mission_Unload" == mission `0x10`. Mission `0x10` (the one radio `0x15`'s
`DockUnload` branch actually queues) resolves to `0x0073D630`
(`Mission_Deploy_Building`, §2.4), a completely different function. This is corroborating,
not novel, since prior doc `RADIO_REFINERY_DOCK_TS_LEGACY_AND_CONTEXT_GHIDRA_REPORT.md`
already concluded "It is NOT called during standard ore-harvester refinery docking" for
`0x00740EF0` — this session adds the exact vtable-slot proof of *why* (it isn't even bound
to the `0x10` dispatch slot).

---

## 5. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|
| Refinery `GetDockCoord` (`Refinery=yes`) is a hardcoded `building.GetCoord()+(0x80,0,0)` lepton offset and **structurally never reads `DockingOffset%d`**, even if art.ini defines one | `src/sim/miner/miner_dock_sequence.rs::refinery_pad_cell` currently branches to `pad_geometry::pad_cell_for` (which *does* consume `docking_offset`) whenever a `DockingOffset0` is present in art.ini for the building | `src/sim/miner/miner_dock_sequence.rs::refinery_pad_cell`, `src/sim/docking/pad_geometry.rs` | A modded `Refinery=yes` building with an explicit `DockingOffset0=` in artmd.ini must still dock at `center+(0x80,0,0)`, not at the configured offset | `refinery_pad_cell_ignores_docking_offset_when_refinery_flag_set` | Do not let a future art.ini `DockingOffset0` on a `Refinery=yes` building silently change dock-pad math via the generic `pad_geometry` path — the Refinery branch must stay a hard bypass, matching `0x00447B20`'s unconditional jump around the offset-reading block |
| CAN_DOCK (`0x0E`) accepted target is hardcoded `NW+(3,1)` cells; `GetDockCoord`'s refinery answer (`center+0x80` lepton on X) is a *separate* coordinate, not derived from the same computation | Confirm Rust keeps these as two distinct constants/paths, not one shared "dock cell" value | `src/sim/miner/miner_dock_sequence.rs::refinery_can_dock_queue_cell` vs `refinery_pad_cell` | Harvester's CAN_DOCK move target and its final GetDockCoord-derived pad alignment can legitimately differ by design; a test asserting they're equal would be asserting a false invariant | `refinery_can_dock_target_and_get_dock_coord_are_independent_values` | Do not merge the two into one "dock cell" concept during a future refactor |
| Radio `0x15` DockUnload branch queues only sender mission `0x10`; building never enters `MissionRepairAndProduce`/`0x14`, never sets `field_0x6DD` | Verify Rust's refinery dock-accept path does not model a building-side "repairing/producing" state machine for stock ore refineries | miner dock-acceptance code, any building FSM shared with `UnitRepair`/`Bunker` | Stock `GAREFN` docking a `HARV` never transitions the building into a repair-pad-style busy state | `stock_refinery_dock_accept_does_not_enter_repair_and_produce` | Do not reuse the UnitRepair/Bunker building-side busy-state code path for stock DockUnload buildings |

---

## 6. Negative Facts / Do Not Do

- Do not call `UndockUnit` (`0x004593A0`) from the normal cargo-empty unload exit. Its only
  three live callers are `Sell`, `ReceiveDamage`, `TemporalClass::Update` — interrupt/destroy
  paths only. Evidence: `get_function_callers 0x004593A0` (this session).
- Do not model stock `GAREFN`/`NAREFN` docking as reaching `BuildingClass::MissionRepairAndProduce`
  (`0x0044B780`) or building mission `0x14`. That branch requires `UnitRepair`/`UnitReload`/
  `Hospital`/`Armory`/`Bunker`, none of which stock refineries set. Evidence:
  `decompile_function 0x0043C2D0` case `0x15` (this session) + INI read (§3, this session).
- Do not treat `UnitClass::PerCellProcess` (`0x00739EC0`, historically mislabeled
  `Mission_Enter`) as the mission-7 dispatch target — that is `FootClass::Mission_Enter`
  (`0x004D9290`). PerCellProcess is a per-cell-crossing hook, invoked on a different trigger.
  Evidence: `get_xrefs_to 0x004D9290` vtable binding + `decompile_function 0x005B3060`
  dispatch table (this session).
- Do not equate the Ghidra label `UnitClass__Mission_Unload` (`0x00740EF0`) with mission ID
  `0x10`. Vtable-slot arithmetic places it at `+0x24C` (mission `0x14`'s slot), not `+0x23C`
  (mission `0x10`, which is `Mission_Deploy_Building @ 0x0073D630`). Evidence: `get_xrefs_to
  0x00740EF0` cited value cross-checked against `get_xrefs_to 0x0073D630` and
  `decompile_function 0x005B3060` (this session).
- Do not let refinery `GetDockCoord` consume `DockingOffset%d` in Rust even for modded
  content — the branch is structurally bypassed in the binary, not merely INI-empty in
  stock. Evidence: `disassemble_function 0x00447B20` (this session), §2.3.

---

## 7. Unverified / Out of Scope for This Slot

- The exact internal drain-rate math inside `Mission_Deploy_Building` state 3
  (`HarvesterDumpRate`-equivalent cadence, RNG use if any) — owned by slot-4 per dispatch
  instructions ("slot 4 owns drain granularity"). Not re-derived here.
- The chrono-miner teleport gate interacting with this chain — owned by slot-3 per dispatch
  instructions. Not re-derived here.
- Exact semantics of `UnitClass::PerCellProcess @ 0x00739EC0`'s dock-arrival radio-`0x15`
  send (already covered by prior docs, e.g. `UNITCLASS_PERCELLPROCESS_CHRONO_MINER_DOCK_ARRIVAL_00739EC0_GHIDRA_REPORT.md`)
  — out of this slot's re-verification scope; cited for context only, not re-checked live
  this session.
- Where radio `7` (DOCKING_COMPLETE) is ultimately transmitted from, if anywhere reachable
  in YR at all — prior docs leave this an open question; moot for stock GAREFN/NAREFN since
  the only candidate sender (`MissionRepairAndProduce`) is proven unreachable for them (§2.5),
  but the question itself remains formally open for non-stock building types.

---

## 8. Stale-Doc Replacement Wording

**File:** `docs/research/miner/MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`

The existing 2026-05-24 correction banner at the top of that doc already redirects readers
to `STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md` and correctly
separates `FootClass::Mission_Enter @ 0x004D9290` from `UnitClass::PerCellProcess @
0x00739EC0`. That banner is accurate and should stay. Add one more line to it:

> Old: (banner ends after the `0x16` / PerCellProcess `GetDockCoord` equality sentence)
>
> New, append: "For the accept→dock-coord→unload-start chain specifically (mission-7
> dispatch, radio `0x0E`/`0x15` receiver logic, `BuildingClass::GetDockCoord`'s Refinery
> bypass of `DockingOffset%d`, and `UnitClass::Mission_Deploy_Building` as the mission-`0x10`
> unload-start handler), see
> `MISSION_ENTER_REFINERY_DOCK_RESWARM_GHIDRA_REPORT.md` — all four re-verified live against
> `gamemd.exe`."

Additionally, this doc's own §6.4/§6.5 pseudocode (`case 0x15`) is directionally correct
(DockUnload → `sender->SetMission(0x10, 0)` only) but its §8/§9/§11 narrative prose still
describes the exit side as if `MissionRepairAndProduce`/`UndockUnit`/radio-7 participate in
the *normal* stock cycle ("Building finishes unloading (MissionRepairAndProduce state 1 →
state 5/Guard)... Calls BuildingClass::UndockUnit... Unit receives radio 7"). That prose
should be corrected or removed — replace with the `Mission_Deploy_Building` state-3/4 flow
in §2.4 of this report.

---

## Sources

- `get_function_by_address 0x004D9290`, `get_xrefs_to 0x004D9290` — this session.
- `decompile_function 0x005B3060` (`MissionClass::Mission_Dispatch`) — this session.
- `decompile_function 0x0043C2D0` (`BuildingClass::Receive_Radio`) — this session.
- `decompile_function 0x00447B20`, `disassemble_function 0x00447B20`
  (`BuildingClass::GetDockCoord`) — this session.
- `decompile_function 0x005F6C80` — this session.
- `decompile_function 0x0073D630` (`UnitClass::Mission_Deploy_Building`), `get_xrefs_to
  0x0073D630` — this session.
- `get_function_callers 0x004593A0` (`BuildingClass::UndockUnit`) — this session.
- Live INI read: `ini/rulesmd.ini:11722–11769` (`[GAREFN]`), `ini/rulesmd.ini:12515–12560`
  (`[NAREFN]`) — this session, including negative grep for `WeaponsFactory` across both
  blocks.
- Corroborating prior docs (not substituted for the live reads above):
  `docs/research/miner/REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md`,
  `docs/research/BUILDING_MISSIONREPAIRANDPRODUCE_DOCKUNLOAD_REACHABILITY_GHIDRA_REPORT.md`,
  `docs/research/DOCKING_QUEUE_EXIT_REFERENCE_POINTS_GHIDRA_REPORT.md`,
  `docs/research/miner/RADIO_REFINERY_DOCK_TS_LEGACY_AND_CONTEXT_GHIDRA_REPORT.md`,
  `docs/research/UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`.
- Rust surface scan: `src/sim/miner/miner_dock_sequence.rs` (read this session, lines
  260–310).

**Status: COMPLETE.**
