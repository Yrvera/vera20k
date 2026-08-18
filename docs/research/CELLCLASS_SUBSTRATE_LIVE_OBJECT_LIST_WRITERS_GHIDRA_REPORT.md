# CellClass Substrate Live Object List Writers - Ghidra Research Report

**Address(es):** `0x0047E8A0`, `0x0047EA90`, `0x005683C0`, `0x005687F0`, `0x005F4EC0`, `0x005F4D30`, `0x005F4160`, `0x0047DD70`, `0x007441B0`, `0x00744210`, `0x005F60A0`, `0x005F6120`, `0x005F6250`, `0x005F5E80`
**Investigation Mode:** coverage-map
**Claimed Scope:** active writer inventory for per-cell live object-list membership (`CellClass+0xE4/+0xE8`), related occupancy bits (`CellClass+0x124/+0x128` where needed), and Rust `OccupancyGrid` migration implications for reveal/unlimbo, conceal/limbo, movement, destroy/uninit, bridge relayer, and save/load rebuild.
**Non-Scope:** consumer behavior except to prove writer liveness; full `Can_Enter_Cell`; full `CellRect` contracts; rendering/targeting; all exotic non-Techno objects that may occupy `AltObject`; full native savegame stream owner.
**Confidence:** Medium-High. Existing Ghidra reports provide high-confidence decompile/assembly evidence for core writer bodies and bridge relayers. This slot did not have a live Ghidra MCP exposed, so native save/load CellClass-list rebuild ownership remains partial.
**Active in YR:** Yes for ordinary reveal/unlimbo, movement, conceal/limbo, destroy/uninit, terrain lifecycle, and bridge collapse. Conditional for save/load and bridge/drop paths.

## 0. Working Notes Gate

**Target question:** Which active gamemd paths add, remove, relayer, destroy, or rebuild live per-cell object-list membership for `CellClass+0xE4/+0xE8`, and what must a Rust-native `OccupancyGrid` migration preserve?

**Non-goals:** Do not rediscover settled AddContent/RemoveContent ordering, do not investigate consumers except to prove writer liveness, do not edit Rust, and do not mutate Ghidra.

**Evidence needed to mark COMPLETE:** decompile/assembly or prior verified report evidence for each writer family; current Rust source scan for `OccupancyGrid` add/remove/move/rebuild and lifecycle call sites; implementation handoff with concrete test names.

**Stop conditions:** Stop after the writer map and Rust handoff are explicit; mark save/load native rebuild and exotic object families as Remaining Uncertainty if not provable from exposed tools.

## 1. Overview

`gamemd.exe` maintains live cell membership through selected-list writes, not by deriving a global occupancy set from object coordinates. The normal Techno path calls `CellClass::AddContent` / `RemoveContent` through enter/exit cell helpers, selecting ground `CellClass+0xE4` or bridge/deck `CellClass+0xE8` from the object's current `OnBridge` byte (`ObjectClass+0x8C`). Related occupancy bits at `+0x124/+0x128` are maintained by separate mark/clear virtuals and can disagree with the object-list layer in bridge edge cases.

Rust already has the right core shape for a future migration: `OccupancyGrid` stores layer-tagged entries, has structure-append/non-structure-prepend insertion, and uses `GameEntity::occupancy_list_layer()` as the `OnBridge`-style selector. The dangerous gaps are lifecycle ownership and rebuild semantics: some kill paths still remove from `EntityStore` directly, bridge fallout gathers victims by sorted entity order, and Rust save/load rebuilds skipped occupancy from `EntityStore` rather than from a native-proven cell-list reconstruction order.

## 2. Key Offsets / State

| Field / helper | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `CellClass+0xE4` | ground `FirstObject` list head | `CellClass::AddContent @ 0x0047E8A0`; `RemoveContent @ 0x0047EA90`; bridge occupancy report | Yes |
| `CellClass+0xE8` | bridge/deck `AltObject` list head | same as above | Conditional on bridge/list-layer selection; live in standard bridge play |
| `ObjectClass+0x30` | next pointer within selected cell list | Add/Remove reports | Yes |
| `ObjectClass+0x8C` | `OnBridge` / selected object-list layer for normal add/remove | callsites `0x005684BB`, `0x005688EB`; bridge reports | Yes |
| `CellClass+0x124` | ground occupancy bits | `Mark_Occupation @ 0x007441B0`; `CheckCellPassability`; terrain mark reports | Yes |
| `CellClass+0x128` | bridge/deck occupancy bits | `Mark_Occupation`, `Clear_Occupation`, bridge occupancy report | Conditional on bridge/deck height and flags; live |
| `CellClass+0x100` | hidden building occupancy counter, not normal list membership | `0x005683C0`, `0x005687F0`; hidden reader report | Conditional via `CanHideThings`; live for stock buildings |
| `ObjectClass+0x98` | LogicClass active-vector membership byte, not cell-list membership | `FUN_0055BAA0`, `FUN_0055BAE0` reports | Yes |

## 3. Writer Inventory

### 3.1 Reveal / Unlimbo / Initial Placement

Material finding: successful object reveal is active-vector registration, while cell-list membership is written by enter/mark helpers around placement. `ObjectClass::Reveal @ 0x005F4EC0` appends eligible logic objects through `FUN_0055BAA0`; `TechnoClass::EnterCell_AddToMultiCells @ 0x005683C0` calls `CellClass::AddContent @ 0x0047E8A0` for base foundation/current cell membership and passes the object's `+0x8C` list-layer byte. Active in YR: Yes. Evidence: `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`; `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`; callsite assembly `0x005684B1..0x005684BB`.

Material finding: building placement writes normal cell content for base foundation cells, not `AddOccupy`/`RemoveOccupy` adjusted cells. The hidden modifiers only affect `CellClass+0x100` under `CanHideThings`. Active in YR: Yes/Conditional (`CanHideThings`, default true). Evidence: `BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`; writers `0x005683C0`, `0x005687F0`.

Rust implication: live object storage, active LogicClass membership, and cell occupancy must stay separate. `Simulation::reveal` currently appends logic membership (`src/sim/world/mod.rs:703`), while spawns add occupancy directly (`src/sim/world/world_spawn.rs:260`, `:438`, `:441..449`). The first safe substrate boundary is a lifecycle helper that couples only the native-equivalent side effects required for each transition, without deriving list membership from storage alone.

### 3.2 Conceal / Limbo / Entering Transports

Material finding: `ObjectClass::Conceal @ 0x005F4D30` removes active-vector membership through `FUN_0055BAE0`, and Techno exit-cell helpers remove from exactly one selected CellClass object list via `CellClass::RemoveContent @ 0x0047EA90`. Active in YR: Yes. Evidence: `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`; `ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`; `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`.

Material finding: feature-specific limbo can alter the list-layer byte before conceal. Infantry enter has a live branch at `0x0051A407` that clears `ObjectClass+0x8C` before a successful enter/conceal path. Active in YR: Conditional on infantry enter flows, but stock YR uses infantry enter/garrison/transport-like paths. Evidence: `BRIDGE_OBJECT_ONBRIDGE_EXTRA_WRITERS_GHIDRA_REPORT.md`.

Rust implication: `Simulation::conceal` unregisters only active order (`src/sim/world/mod.rs:709`); callers that represent boarding/limbo must also remove or avoid re-adding occupancy at the correct time. Passenger boarding already calls `sim.conceal(pax_id)` (`src/sim/passenger.rs:487`), while unloading manually adds occupancy then reveals (`src/sim/passenger.rs:871`, `:881`). These flows need layer-specific tests before a shared substrate is introduced.

### 3.3 Movement Cell Crossing

Material finding: normal movement relayering must remove from the old selected list using old `OnBridge`, update coordinates/bridge state, then insert into the new selected list using new `OnBridge`. Active in YR: Yes for drive/walk/ship movement. Evidence: `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`; `BRIDGE_OBJECT_ONBRIDGE_FIELD` facts cited there; callsites `0x005684B1`, `0x005688E1`.

Material finding: occupancy bits are a separate writer surface. `ObjectClass::Mark_Occupation @ 0x007441B0` sets `+0x128` only if both the Z threshold and structural bridge flag are present; `Clear_Occupation @ 0x00744210` can clear bridge bit `0x20` by Z threshold even if the bridge flag has already been cleared. Active in YR: Yes. Evidence: `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`.

Rust implication: `movement_step.rs` now projects `on_bridge` before `occupancy.move_entity` and uses the projected occupancy layer (`src/sim/movement/movement_step.rs:1187..1197`). `movement_tick.rs` similarly projects the bridge state before moving occupancy (`src/sim/movement/movement_tick.rs:1238..1255`). That is directionally aligned, but `OccupancyGrid::move_entity` removes by entity id across the whole cell rather than by old selected layer (`src/sim/occupancy.rs:182`, `:192`), so it can silently repair or hide duplicate/cross-layer bugs that native `RemoveContent` would not.

### 3.4 Destroy / Uninit / Lethal Damage

Material finding: standard UnitClass lethal damage calls `vtable+0xF8` synchronously, leading through UnInit/Limbo/Conceal to active-vector removal; `FootClass::Destroy` (`vtable+0xDC`) is cell/list cleanup only and does not remove active-vector membership. Active in YR: Yes, with amphibious sinking/death-timer exceptions. Evidence: `TARGETDEATH_RECEIVEDAMAGE_DEATH_DISPATCH_REMOVAL_TIMING_RESWARM_20260528.md`.

Material finding: `ObjectClass::Destroy`/`Detach_From_All_Lists` removes object cell-list membership before full uninit/free, while pending delete defers physical memory free rather than active-list removal. Active in YR: Yes. Evidence: target-death report and active-vector remover report.

Rust implication: `Simulation::uninit` centralizes occupancy remove, radio cleanup, conceal, and `EntityStore` removal (`src/sim/world/mod.rs:821..839`), but combat still has a direct non-animated death path that calls `occupancy.remove`, clears radio contacts, and `entities.remove(dead_id)` without going through `unregister_live_object` (`src/sim/combat/mod.rs:1003..1009`). That is a live migration blocker: substrate APIs must forbid direct storage removal for live objects.

### 3.5 Bridge Collapse / DropIn Relayer

Material finding: `CellClass::BlowUpBridge @ 0x0047DD70` first walks ground list `+0xE4`, snapshots `Object+0x30`, and force-kills ground occupants. It then walks bridge/deck list `+0xE8`, snapshots next, and calls `ObjectClass::DropIn` (`vtable+0xEC`) instead of killing deck occupants. Active in YR: Yes for bridge collapse. Evidence: `BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md`; assembly ranges `0x0047DD84..0x0047DDC9`.

Material finding: `ObjectClass::DropIn @ 0x005F4160` relayers by calling mark/remove while `OnBridge==1`, clearing `ObjectClass+0x8C`, then calling mark/add while `OnBridge==0`. Active in YR: Yes for ordinary bridge-deck Techno objects. Evidence: `0x005F4178..0x005F41A1`; add/remove layer readers `0x005684B1`, `0x005688E1`.

Rust implication: `drop_in_bridge_deck_entities` clears `on_bridge`, sets ground state, then calls `occupancy.move_entity(rx, ry, rx, ry, ..., Ground, ...)` (`src/sim/world/bridge_orchestrator.rs:1370..1391`). Outcome is close, but old-layer removal is handled by broad id removal, not native selected-list removal. `kill_ground_occupants_at` gathers victims via `EntityStore::iter_sorted()` (`src/sim/world/bridge_orchestrator.rs:1023..1044`), not the `CellClass+0xE4` list order; if death side effects matter, this is order drift.

### 3.6 Terrain Objects And Non-Techno Occupancy Bits

Material finding: TerrainClass map objects are live objects with lifecycle side effects. `TerrainClass::Unlimbo` places the object, increments neighbor terrain counters, clears flagged source overlays, and `TerrainClass::Mark_Occupation` sets `CellClass+0x124` bits from terrain type masks. `TerrainClass::Limbo` reverses those bits and cell/zone/radar state. Active in YR: Yes for TIBTRE and other terrain objects; ordinary TIBTRE damage removal is conditional because stock `Immune=yes`. Evidence: `TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md`; `0x0071D000`, `0x0071C930`, `0x0071C110`, `0x0071C070`.

Rust implication: the `OccupancyGrid` migration should not pretend all cell substrate writes are Techno object-list writes. Terrain mark/unmark is bitfield occupancy (`+0x124`), not `+0xE4/+0xE8` list membership.

### 3.7 Save / Load / Rebuild

Material finding: `ObjectClass::Save @ 0x005F6250` / `ObjectClass::Load @ 0x005F5E80` do not themselves persist the active-vector membership byte `+0x98`, do not call `ObjectClass::Reveal`, and do not directly re-register into `LogicClass`. Active in YR: Conditional on save/load, but those paths are live. Evidence: `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`; `OBJECT_98_SAVE_LOAD_FINAL_BYTE_PROVENANCE_RESWARM_20260528.md` cited there.

Material finding: this slot did not prove the native owner that rebuilds `CellClass+0xE4/+0xE8` after savegame load. The safe claim is negative: do not derive parity from `ObjectClass::Load` alone and do not sort entities as a replacement for native reconstruction order. Active in YR: Conditional save/load. Evidence: existing save/load reports defer the post-load active-vector rebuild owner.

Rust implication: `Simulation::occupancy` is `#[serde(skip)]` (`src/sim/world/mod.rs:369..370`) and `rebuild_caches_after_load` reconstructs it with `OccupancyGrid::rebuild(&self.entities)` (`src/sim/world/mod.rs:972..973`). Rebuild scans `EntityStore::values()` and re-adds using current category and `occupancy_list_layer()` (`src/sim/occupancy.rs:110..128`). This is deterministic, but not native-proven for save/load cell-list order. The logic vector is serialized verbatim, but occupancy is not.

## 4. Current Rust Status

| Surface | Status | Evidence | Migration implication |
|---|---|---|---|
| `OccupancyGrid::add` | matches settled AddContent order for current categories: structures append, others prepend within selected layer | `src/sim/occupancy.rs:30..36`, `:151..174` | keep |
| `OccupancyGrid::remove` | broad id removal from a cell, not selected-list-only `RemoveContent` | `src/sim/occupancy.rs:182..189` | useful cleanup, but too forgiving for parity diagnostics |
| `OccupancyGrid::move_entity` | remove plus add using one new layer | `src/sim/occupancy.rs:192..207` | needs old-layer/new-layer ownership for strict relayer tests |
| `GameEntity::occupancy_list_layer` | uses `on_bridge`, not locomotor layer; excludes Air/Underground | `src/sim/game_entity.rs:567..596` | matches normal `Object+0x8C` intent |
| `Simulation::uninit/despawn_entity` | conceal-before-storage-free centralized | `src/sim/world/mod.rs:821..844` | correct primitive, but bypassed by some callers |
| combat non-animated death | direct entity removal, no active unregister | `src/sim/combat/mod.rs:1003..1009` | blocker for lifecycle substrate migration |
| save/load occupancy | skipped cache rebuilt from store | `src/sim/world/mod.rs:972..973`, `src/sim/occupancy.rs:110..128` | deterministic but native save/load order unproven |
| bridge DropIn | clears state then same-cell ground move | `src/sim/world/bridge_orchestrator.rs:1370..1391` | outcome close; old-layer removal not pinned |

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CellClass::AddContent` / `RemoveContent` selected-list writes | verified | `0x0047E8A0`, `0x0047EA90`; existing ordering reports | none |
| Techno enter/exit normal object-list writers | verified | `0x005683C0`, `0x005687F0`; callsites `0x005684B1`, `0x005688E1` | exact non-Techno list writers deferred |
| reveal/unlimbo active registration vs cell occupancy | verified partial | `ObjectClass::Reveal @ 0x005F4EC0`; `TechnoClass::Unlimbo`; enter helper docs | complete call order for every derived class not drained |
| conceal/limbo active unregister and selected-list removal | verified partial | `ObjectClass::Conceal @ 0x005F4D30`; `FUN_0055BAE0`; exit helper docs | feature-specific layer-byte mutations need per-feature tests |
| movement relayer order | verified | bridge occupancy reports | old-layer exact Rust removal not implemented |
| destroy/uninit | verified partial | target-death and active-vector reports | infantry/aircraft edge families out of scope |
| bridge DropIn relayer | verified | `0x005F4160`; bridge collapse report | exotic `AltObject` occupants deferred |
| hidden AddOccupy/RemoveOccupy negative fact | verified | building foundation and hidden reader reports | no Rust hidden counter implemented |
| terrain occupation bits | verified for TIBTRE slice | terrain lifecycle report | all terrain types not exhaustively classified |
| native save/load CellClass list rebuild | deferred | save/load reports prove negative ObjectClass facts | trace native save/load stream owner/post-load rebuild |
| current Rust caller inventory | touched-not-exhausted | `rg` over `src/sim` | slot 4 owns full caller inventory |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-WR-001 - What are the normal per-cell object-list writers? -> Techno enter/exit helpers call CellClass AddContent/RemoveContent, selecting +0xE4/+0xE8 from Object+0x8C.` (evidence: `0x005683C0`, `0x005687F0`, `0x0047E8A0`, `0x0047EA90`)
- `[RESOLVED] OQ-WR-002 - Are AddOccupy/RemoveOccupy real list membership? -> No; they adjust Cell+0x100 hidden occupancy, not base foundation Cell+0xE4 membership.` (evidence: `BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-WR-003 - Does reveal alone mean cell-list membership? -> No; Reveal registers active logic membership, while cell occupancy is placed through mark/enter helpers.` (evidence: `0x005F4EC0`, `0x005683C0`)
- `[RESOLVED] OQ-WR-004 - Does conceal/uninit remove active membership synchronously? -> Yes through FUN_0055BAE0; standard UnitClass and BuildingClass deaths reach it synchronously in the verified paths.` (evidence: active-vector and target-death reports)
- `[RESOLVED] OQ-WR-005 - Does movement relayer need old and new list selectors? -> Yes; normal bridge transition removes with old OnBridge and adds with new OnBridge.` (evidence: bridge occupancy reports)
- `[RESOLVED] OQ-WR-006 - Does DropIn kill deck occupants? -> No; deck occupants are relayered from bridge list to ground list after OnBridge clear.` (evidence: `0x0047DD70`, `0x005F4160`)
- `[RESOLVED] OQ-WR-007 - Are +0x124/+0x128 just list mirrors? -> No; they are separate bitfields written by mark/clear routines and terrain mark/unmark.` (evidence: bridge occupancy and terrain lifecycle reports)
- `[RESOLVED] OQ-WR-008 - What does current Rust rebuild after snapshot load? -> It rebuilds skipped OccupancyGrid from EntityStore values using occupancy_list_layer/category insertion.` (evidence: `src/sim/world/mod.rs:972..973`; `src/sim/occupancy.rs:110..128`)
- `[DEFERRED] OQ-WR-009 - Which native save/load owner rebuilds CellClass+0xE4/+0xE8 exactly?` (category: `requires-different-system-context`; reason: existing reports defer post-load vector/list owner and no live Ghidra MCP was exposed; next-step-if-pursued: trace savegame object stream post-load init/reveal/mark pass)
- `[DEFERRED] OQ-WR-010 - All exotic non-Techno objects that can occupy +0xE8 and their vtable+0xEC relayer behavior.` (category: `out-of-scope`; reason: normal bridge-deck Techno path is enough for migration slice; next-step-if-pursued: classify AltObject occupants by constructor/mark vtables)
- `[DEFERRED] OQ-WR-011 - Full Rust caller inventory.` (category: `out-of-scope`; reason: parent assigned slot 4 to movement, production spawn, placement, scatter, bridges, AI inventory; next-step-if-pursued: consume slot-4 report)

## 7. Negative Facts / Do Not Do

| Do not | Evidence | Active in YR |
|---|---|---|
| Do not model cell occupancy as every entity in `EntityStore`. Limbo/stored objects can exist without active/vector/cell membership. | `Object+0x98` helper reports; `spawn_object_limbo_at_height` Rust split | Yes |
| Do not conflate `ObjectClass+0x98` active-vector membership with `CellClass+0xE4/+0xE8` cell-list membership. | `FUN_0055BAA0/FUN_0055BAE0` vs `AddContent/RemoveContent` reports | Yes |
| Do not use `AddOccupy` / `RemoveOccupy` as real foundation or cell-list blockers. They feed `Cell+0x100` hidden counter. | `0x005683C0`, `0x005687F0`; hidden reader report | Conditional via `CanHideThings`; stock active |
| Do not remove from both ground and bridge lists to be safe. Native RemoveContent selects one list and does not scan the other. | `0x0047EA90`; bridge occupancy reports | Yes |
| Do not clear `OnBridge` before removing a bridge-deck object during DropIn. | DropIn order `0x005F4178` before `0x005F418F` | Yes for deck collapse |
| Do not rebuild parity cell-list order after save/load by stable-id sort unless native post-load order proves it. | save/load reports; Rust `OccupancyGrid::rebuild` source | Conditional save/load |

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Normal cell-list membership is written by selected-list AddContent/RemoveContent using object `OnBridge`; active-vector membership is separate reveal/conceal state. | `0x005683C0`, `0x005687F0`, `0x0047E8A0`, `0x0047EA90`, `0x005F4EC0`, `0x005F4D30` | core pieces exist but lifecycle calls are scattered | `src/sim/occupancy.rs`; `src/sim/world/mod.rs`; spawn/passenger/movement callers | Introduce a substrate lifecycle boundary that performs only the native side effects for reveal/unlimbo/conceal/uninit and keeps storage, active order, and cell lists separate. | Limbo-created infantry is stored but absent from logic order and occupancy; unlimbo adds occupancy and tail-appends active order. Proposed test: `cell_substrate_unlimbo_adds_cell_list_and_logic_membership_only_on_success`. | Do not let `EntityStore` insertion imply cell occupancy or active AI membership. |
| Movement transition removes from old selected list before updating `OnBridge` and inserts into new selected list after update. | bridge occupancy reports; add/remove layer callsites `0x005684B1`, `0x005688E1` | Rust projects new layer before `move_entity`, but `remove` is broad id cleanup and can hide wrong old-layer state | `src/sim/occupancy.rs`; `src/sim/movement/movement_step.rs`; `src/sim/movement/movement_tick.rs` | Make strict relayer APIs accept old list layer and new list layer, or add diagnostics that fail on stale wrong-layer duplicates. | Unit crossing a bridgehead exits ground list and enters bridge list in one cell-crossing, with no stale ground entry. Proposed test: `cell_substrate_bridge_transition_removes_old_layer_before_add_new_layer`. | Do not use forgiving remove-from-any-layer as the only parity path. |
| Destroy/uninit vacates cell-list membership and active-vector membership synchronously for standard lethal Unit/Building paths; physical free is deferred separately. | target-death reports; active-vector remover `0x0055BAE0` | combat non-animated path still calls `entities.remove` directly | `src/sim/combat/mod.rs`; `src/sim/world/mod.rs::uninit` | Route live object death/despawn through a single uninit/despawn primitive that removes occupancy, contacts, active membership, then storage. | Kill a voxel unit and assert its id is absent from occupancy and logic order before the next live-object tick. Proposed test: `cell_substrate_lethal_uninit_clears_occupancy_and_logic_before_store_free`. | Do not directly `entities.remove` a live object. |
| Bridge collapse `DropIn` relayers deck occupants from bridge list to ground list; ground list occupants are killed first and deck occupants are not C4-killed. | `0x0047DD70`, `0x005F4160`; bridge collapse report | Rust outcome mostly present, but ground victims are sorted-store order and DropIn old-layer removal is forgiving | `src/sim/world/bridge_orchestrator.rs`; `src/sim/occupancy.rs` | Walk occupancy list order for ground deaths if side effects matter; relayer deck objects with old bridge layer then new ground layer. | Ground and deck units share one bridge cell; collapse kills only ground, relayers deck to ground, and preserves list-order event log. Proposed test: `cell_substrate_bridge_collapse_kills_ground_and_dropin_relayers_deck_lists`. | Do not merge ground/deck occupants into one fallout set. |
| Save/load Rust rebuild currently reconstructs skipped occupancy from `EntityStore`, but native CellClass list rebuild owner/order remains unproven. | Rust `src/sim/world/mod.rs:972..973`; save/load reports | deterministic, not native-proven | `src/sim/snapshot.rs`; `src/sim/world/mod.rs`; `src/sim/occupancy.rs` | Treat snapshot occupancy rebuild as a Rust cache rebuild, not a proven gamemd save/load contract; add tests that preserve current Rust order and leave native parity blocked on follow-up. | Save/load a cell containing building plus two mobile entrants and assert Rust cache rebuild is deterministic and documented, while native parity remains `UNCHECKED`. Proposed test: `cell_substrate_snapshot_rebuild_is_deterministic_but_not_native_order_claim`. | Do not market stable-id rebuild as byte-perfect gamemd save/load. |

## 9. Stale Docs / Follow-up Wording

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`: replace the current-Rust statement "OccupancyGrid appends all occupants" with "Current Rust `OccupancyGrid::add` takes `CellListInsertion` and preserves the verified structure-append / non-structure-prepend order within the selected layer; remaining risk is lifecycle/rebuild ownership and consumers that bypass `iter_layer`."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`: replace `OQ-7` wording saying Rust "breaks on first missing key" with "Current Rust `parse_numbered_cell_offsets` visits all 1..8 numbered slots independently and stores only valid pairs; this is observable-equivalent to the binary's sentinel-filled eight-slot storage for absent keys."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`: replace stale Rust-delta wording "no membership byte" / "`unregister_live_object` always edits vector" with "Current Rust has `GameEntity::in_logic_vector` and byte-gated register/unregister; remaining gaps are upstream reveal eligibility gates and callers that bypass `Simulation::uninit`."

## Sources

- Existing verified Ghidra reports: `CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`; `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`; `CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md`; `bridges/02-cell-state-layering-zones/BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`; `bridges/02-cell-state-layering-zones/BRIDGE_OBJECT_ONBRIDGE_EXTRA_WRITERS_GHIDRA_REPORT.md`; `bridges/05-damage-collapse-repair-cabhut/BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md`; `BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`; `CELLCLASS_0X100_HIDDEN_OCCUPANCY_READERS_GHIDRA_REPORT.md`; `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`; `ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`; `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`; `TARGETDEATH_RECEIVEDAMAGE_DEATH_DISPATCH_REMOVAL_TIMING_RESWARM_20260528.md`; `TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md`.
- Addresses cited from those reports: `0x0047E8A0`, `0x0047EA90`, `0x005683C0`, `0x005687F0`, `0x005684B1`, `0x005688E1`, `0x005F4EC0`, `0x005F4D30`, `0x0055BAA0`, `0x0055BAE0`, `0x005F4160`, `0x0047DD70`, `0x007441B0`, `0x00744210`, `0x005F60A0`, `0x005F6120`, `0x0071D000`, `0x0071C930`, `0x0071C110`, `0x0071C070`, `0x005F6250`, `0x005F5E80`.
- Rust files scanned read-only: `src/sim/occupancy.rs`, `src/sim/game_entity.rs`, `src/sim/world/mod.rs`, `src/sim/world/logic_vector.rs`, `src/sim/world/world_spawn.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/sim/movement/movement_step.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/combat/mod.rs`, `src/sim/passenger.rs`, `src/sim/snapshot.rs`.

Status: PARTIAL - writer families are implementation-ready for reveal/conceal/move/destroy/bridge-relayer migration, but native save/load CellClass object-list rebuild order remains unresolved and no live Ghidra MCP was exposed in this slot.
