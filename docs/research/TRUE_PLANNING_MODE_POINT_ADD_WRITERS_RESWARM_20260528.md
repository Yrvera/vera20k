# TRUE_PLANNING_MODE_POINT_ADD_WRITERS reswarm report

Date: 2026-05-28
Slot: 3
Target: TRUE_PLANNING_MODE_POINT_ADD_WRITERS
Mode: bounded coverage-map / partial writer trace
Verdict: PARTIAL

This report resolves the active tactical-click and EventClass path for true Planning
Mode authoring into per-unit planning-token storage, and separates it from the
House/player `WaypointPathClass` overlay storage. It does not prove the active
append writer that stores a new 0x0C coordinate point into `WaypointPathClass+0x2C`
and increments `WaypointPathClass+0x38`; that append writer remains the key
remaining uncertainty.

## Scope

In scope:

- True Planning Mode click/event handling from tactical click through
  `EventClass::Execute`.
- Event IDs `0x2A..0x2C` and planning-flagged ordinary command events.
- House/player `WaypointPathClass` slot storage, loop field, lookup, next-point,
  addability, and renderer consumers.
- Per-unit `Techno+0x514` planning-token add and loop/closure writers.
- Caps and ownership: House/player path caps versus per-house planning-token caps.

Out of scope:

- Normal second-click queued move behavior. Prior reswarm already verified normal
  second move click reissues `Foot` NavCom and is not the true Planning Mode writer.
- Rust implementation changes.

## Prior Context Used

- `DRIVE_QUEUED_CLICK_EVENT_PLANNING_MODE_OUTCOME_RESWARM_20260528.md`
  established that normal queued second-click behavior is not `Foot` NavQueue
  append and deferred exact true Planning Mode point-add writers.
- `PLANNING_QUEUED_WAYPOINT_LINES_AND_FLAGS_GHIDRA_REPORT.md` established that
  the planning overlay draws from House/player `WaypointPathClass` slots, not from
  `Foot+0x58C/+0x598`.
- `HOTKEY_SYSTEM_GHIDRA_REPORT.md` established active Planning Mode enter/exit
  globals and mouse-handler anchors.

## Active Standard YR Status

The tactical click/EventClass planning path is active in standard Yuri's Revenge
when Planning Mode is enabled. The House/player `WaypointPathClass` overlay
consumer path is also active because display/cursor/renderer code reads it during
planning overlay handling.

The unresolved item is not whether the structures exist or are consumed. The
unresolved item is the exact active writer that appends permanent coordinate points
to `WaypointPathClass+0x2C`.

## Verified Binary Evidence

### Planning Mode global lifecycle

Evidence type: decompile plus caller/xref evidence from documented mouse/hotkey
entry points.

- `FUN_006379C0`: enters Planning Mode. If `DAT_00AC4CF4 == 0`, sets it to `1`,
  plays the start-planning sound, and invokes the optional UI callback with
  `0x11C7`.
- `FUN_00637A10`: exits/commits Planning Mode. If active, constructs and queues an
  `EventClass` event type `0x2B` for the local player house, clears planning
  globals including `DAT_00AC4CCC`, `DAT_00AC4C38`, `g_SelectedObjectsVector`, and
  `DAT_00AC4CF4`, then plays the end-planning sound and invokes the optional UI
  callback with `0x11C8`.
- `FUN_00637AA0`: active getter; returns `DAT_00AC4CF4`.

### Tactical click classification

Evidence type: decompile plus caller evidence from DisplayClass action/cursor
paths.

- `DisplayClass::DetermineAction` at `0x00692610` contains a Planning Mode branch
  guarded by the planning overlay state.
- It resolves existing House/player path points through `FUN_005023B0` and
  `FUN_00502460`.
- If the clicked cell is not an existing point, the current path slot is valid,
  `FUN_005090F0()` allows adding, and the cell is in the playfield, the action is
  `0x2A`.
- If adding is not allowed, the fallback action is `0x2B`.
- If the cursor is over an existing point, actions include existing/selection
  outcomes such as `0x2C` and `0x2E`.
- With Alt and an existing point in the current path, addability still true, and
  `FUN_00763BA0(point)` non-null, action `0x2F` is used as a loop/closure
  candidate action.

`DisplayClass::SetCursorFromAction` updates only hover/preview state for the
planning overlay when display Planning Mode is active. It can write a preview
coordinate through `Display+0x11BC` from cell center plus ground/bridge height,
but that write is not the permanent `WaypointPathClass` point append.

### Click dispatch into planning events

Evidence type: decompile plus caller chain from `DisplayClass::BandBox_LeftUp` to
selection dispatch and techno command issue.

- `DisplayClass::BandBox_LeftUp` at `0x004AB9B0` gates non-excluded actions through
  `FUN_00639040()` and `FUN_00639130()` before dispatch. Action `0x2A` is not
  excluded from this gating.
- `Selection__DispatchMultiUnitOrder` at `0x004AE750` resolves whether the click is
  on an existing `WaypointPathClass` point by calling `FUN_005023B0` and
  `FUN_00502460`.
- For each selected unit it invokes the vtable cell-order path. In Planning Mode,
  the techno cell-order path reaches `FUN_006FFBE0`, which intercepts the order
  before normal command queuing.

### Planning interception and EventClass IDs

Evidence type: decompile plus EventClass caller evidence.

- `FUN_006FFBE0` detects active Planning Mode via `FUN_00637AA0()`. When active,
  it builds an ordinary command event and sends it to `FUN_00637DD0`, then returns
  without normal immediate command issue.
- `FUN_00637DD0` marks ordinary event type `0x04` with a planning flag at the event
  byte used by `EventClass::Execute` offset `+0x1D`, then queues the event with
  `FUN_006521C0`. The decompiler shows a stack-byte write, so the exact local name
  is not reliable, but `EventClass::Execute` confirms the consumed effect.
- `EventClass::Execute` at `0x004C6CB0` routes ordinary event cases `0x04` and
  `0x05` into `FUN_00637E00()` when event byte `+0x1D == 1`.
- `EventClass::Execute` also routes event cases `0x2A`, `0x2B`, and `0x2C` directly
  into `FUN_00637E00()`.

Resolved event meanings for this path:

| Event | Active source | Planning meaning |
| --- | --- | --- |
| `0x04` / `0x05` with `+0x1D == 1` | `FUN_006FFBE0` -> `FUN_00637DD0` | Planning-flagged ordinary unit command add. |
| `0x2A` | `FUN_0063AD50` right-click/node path | Planning node loop/delete/selection-style event processed by `FUN_00637E00`. |
| `0x2B` | `FUN_00637A10` exit/commit | Exit/execute Planning Mode; iterates technos and posts eligible per-unit plans. |
| `0x2C` | `FUN_00637D00` via `FUN_00731A10` | Current planning command/node target event. |

### Per-unit true Planning Mode command writer

Evidence type: `FUN_00637E00` caller chain plus decompile of writer helpers.

`FUN_00637E00` is the central executor for Planning Mode events. The ordinary
planning-flagged command path reaches `FUN_00638120`.

`FUN_00638120` is the active command-add writer for true Planning Mode authoring:

- It rejects null objects and invalid target coordinates.
- It checks whether the event house matches the local player house.
- It gets the selected unit's per-unit planning token with `FUN_00705D20`.
- If absent, it allocates a 0x9C planning-token object via `FUN_00638A80` and
  stores it back to `Techno+0x514` through `FUN_00705D10`.
- It validates command-chain compatibility with `FUN_00638CE0`.
- It accounts the command against global/per-house planning state.
- It allocates or finds the relevant planning node, then appends the copied event
  through `FUN_00633FA0` or `FUN_00639A50`.
- Local-player success plays the planning add sound.

`FUN_00633FA0` appends one command item to a planning node and registers that node
with the owning planning token:

- Allocates a 0x10 command-entry object.
- Stores the object/id pointer and an allocated 0x6F copy of the original event.
- Appends that entry to the node dynamic vector.
- Appends the node pointer into the token dynamic vector at token `+0x08/+0x14`
  when needed.
- If token `+0x8C == -1`, initializes it to the first/current node index.

This writer belongs to the unit-side Planning Mode model at `Techno+0x514`. It is
not a direct append into House/player `WaypointPathClass+0x2C`.

### Per-unit loop/closure writers

Evidence type: `FUN_00637E00` event branches plus decompile of loop helpers.

True Planning Mode right-click loop/closure state is unit-side:

- `FUN_00639740` writes planning-token loop fields:
  - `token+0x94 = clicked_node_index`
  - `token+0x98 = 1`
  - `token+0x90 = token_count - clicked_node_index`
- `FUN_00639800` performs a more complex loop/closure rewrite and writes the same
  loop/closure field group:
  - `token+0x90`
  - `token+0x94`
  - `token+0x98 = 1`
- `FUN_00636CE0` clears token loop/selection ranges. It clears node-side loop
  fields and resets `token+0x98 = 0`, then clears `token+0x8C = -1`.

These fields are not House/player `WaypointPathClass+0x24`.

### Planning commit/execute

Evidence type: event `0x2B` branch plus caller evidence.

The `0x2B` branch of `FUN_00637E00` iterates the global techno array. For technos
owned by the event house, it resolves the per-unit planning token with
`FUN_00705D20`, clears/prepares planning state with helpers such as
`FUN_00636570`, `FUN_00636CE0`, and `FUN_00638C70`, then can call `FUN_006385C0`.

`FUN_006385C0` posts execution from a planning token:

- Requires a non-null token with node count > 0.
- Rejects invalid current-index state.
- Queues mission `0x1C` on the techno through the vtable mission entry.
- Copies the stored event data to a new 0x6F event.
- If the first event byte is `0x04`, writes event `+0x1D = 2`.
- Appends the event to the global event vector and marks token execution active.

### House/player WaypointPathClass layout and consumers

Evidence type: constructor decompile plus renderer/cursor/display callers.

House/player planning overlay storage uses `g_PlayerPtr`:

| Field | Meaning |
| --- | --- |
| `g_PlayerPtr+0x20C` | Current House/player waypoint path slot index. |
| `g_PlayerPtr+0x210 + slot*4` | Up to 12 `WaypointPathClass*` slots. |
| `WaypointPathClass+0x24` | Loop index for House/player path rendering/next-point helper; initialized to `-1`. |
| `WaypointPathClass+0x28` | Dynamic vector header/vtable. |
| `WaypointPathClass+0x2C` | Data pointer for 0x0C coordinate point entries. |
| `WaypointPathClass+0x38` | Point count. |
| `WaypointPathClass+0x3C` | Growth/capacity seed initialized to `10`. |

Verified helper functions:

- `WaypointPathClass__Constructor` at `0x00763730`/`0x00763810` initializes a 0x40
  object, sets `+0x24 = -1`, initializes the vector at `+0x28`, sets count
  `+0x38 = 0`, and registers valid slots `0..11`.
- `FUN_00504740` ensures a House/player path slot exists by allocating and storing
  a 0x40 `WaypointPathClass` for a caller-provided slot.
- `FUN_005090A0` scans the 12 slots for an empty path and lazily allocates missing
  path objects. The decompiler is ambiguous about the store in this function, so
  the allocation/store should be rechecked in assembly before implementation.
- `FUN_005090F0` is the House/player addability predicate. It returns addable only
  when the current path slot is valid, `WaypointPathClass+0x38 < Rules+0x90`, and
  `WaypointPathClass+0x24 == -1`.
- `FUN_00763980(path, index)` returns `path+0x2C + index*0x0C` when
  `0 <= index < path+0x38`, else null.
- `FUN_00763BA0(path, point)` advances to the next point. If the next index reaches
  `path+0x38` and `path+0x24 != -1`, it wraps to `path+0x24`; otherwise it returns
  null at the end.
- `FUN_00763A50(path, coord)` scans existing point coordinates and writes
  `WaypointPathClass+0x24 = matching_index` on a cell-coordinate match. This is a
  House/player loop-index writer, but an active caller for it was not found.
- `FUN_00763BE0(path)` clears the vector and resets `WaypointPathClass+0x24 = -1`;
  no direct active caller was found in this slice.
- `FUN_006DAD60` renders planning overlay lines/markers by iterating all 12
  House/player path slots, reading `+0x38`, `FUN_00763980`, and `FUN_00763BA0`.

### House/player WaypointPathClass cap

Evidence type: binary reader plus INI data source plus `FUN_005090F0`.

- `[General] MaxWaypointPathLength=15` appears in both `ini/rules.ini` and
  `ini/rulesmd.ini`.
- The parsed value is stored at `RulesClass+0x90`.
- `FUN_005090F0` compares `WaypointPathClass+0x38 < RulesClass+0x90`.
- Because `FUN_005090F0` also requires `WaypointPathClass+0x24 == -1`, a House
  path with a loop index set is not addable even if count is below the INI cap.

### Per-house PlanningToken cap

Evidence type: dispatch gate plus decompile of cap helper.

`FUN_00639130` is the true Planning Mode per-house planning-node cap check before
dispatch:

- It runs only while Planning Mode is active.
- It computes existing per-house planning count from `DAT_00AC4B84[player_house]`
  plus selected eligible objects that do not already have token commands.
- If the result exceeds `0x80`, it rejects the click and shows the max-planning
  message/sound.

This 128 count is separate from `[General] MaxWaypointPathLength` and separate from
the 12 House/player overlay path slots.

## Ownership Resolution

| State | Owner | Verified use |
| --- | --- | --- |
| `DAT_00AC4CF4` | Global/UI | True Planning Mode active flag. |
| `Display+0x11B3` | Display/UI | Planning display toggle used by cursor/preview handling. |
| `Display+0x11BC` | Display/UI | Hover preview coordinate pointer; not permanent path storage. |
| `g_PlayerPtr+0x20C` | House/player | Current overlay waypoint path slot. |
| `g_PlayerPtr+0x210 + slot*4` | House/player | 12 overlay `WaypointPathClass*` slots. |
| `WaypointPathClass+0x24` | House/player path | Overlay path loop index used by next-point/render helpers. |
| `WaypointPathClass+0x2C/+0x38` | House/player path | Coordinate point vector and count for overlay paths. |
| `Techno+0x514` | Unit/Techno | Per-unit Planning Mode token allocated by `FUN_00638A80`. |
| token `+0x90/+0x94/+0x98` | Unit/Techno planning token | True Planning Mode loop/closure state. |
| `Foot+0x520/+0x686/+0x528` | Unit/Foot follower state | Path id/node/current target while following a `WaypointPathClass`; not the click authoring store. |

## Key Negative Finding

The active tactical-click/EventClass writer proven in this slice is
`FUN_00638120` -> `FUN_00633FA0`/`FUN_00639A50`, which writes per-unit PlanningToken
command-node state under `Techno+0x514`.

That path does not directly append coordinate entries to
`WaypointPathClass+0x2C`.

Therefore the prior broad statement "true Planning Mode clicks write directly to
House/player `WaypointPathClass` points" is not proven by this slot. The more
precise verified statement is:

- Planning overlay rendering and cursor hit-testing consume House/player
  `WaypointPathClass` slots.
- True Planning Mode click authoring is actively routed through EventClass into
  per-unit PlanningToken command writers.
- The bridge or active writer that populates House/player `WaypointPathClass`
  coordinate vectors remains unresolved.

## Current Rust Status

Rust touchpoints checked during the reswarm:

- `src/app_context_order.rs`
- `src/app_cursor.rs`
- `src/app_target_lines.rs`
- `src/sim/components.rs`
- `src/sim/movement/movement_commands.rs`
- `src/sim/movement/movement_tests.rs`

Observed mismatch:

- Rust currently has queued-order behavior tied to Shift/queued move concepts and
  `MovementTarget`/navigation path storage.
- No verified Rust model for true Planning Mode `DAT_00AC4CF4`-style active state,
  House/player `WaypointPathClass` overlay slots, or per-unit `Techno+0x514`
  PlanningToken command nodes was found in this slice.
- Existing Rust movement queue tests should not be treated as parity coverage for
  true Planning Mode.

## Implementation Handoff

For parity work, implement these as separate concepts:

1. True Planning Mode UI state.
   - Global active flag equivalent to `DAT_00AC4CF4`.
   - Display planning toggle/preview state equivalent to `Display+0x11B3` and
     `Display+0x11BC`.

2. House/player overlay waypoint paths.
   - Current path slot equivalent to `House/player+0x20C`.
   - 12 `WaypointPathClass` slots equivalent to `House/player+0x210`.
   - Path point vector and count equivalent to `+0x2C/+0x38`.
   - Loop index equivalent to `+0x24`.
   - Addability must require both `count < Rules.MaxWaypointPathLength` and
     `loop_index == -1`.

3. Per-unit PlanningToken command graph.
   - `Techno+0x514` token pointer.
   - Planning command-node append behavior equivalent to `FUN_00638120` and
     `FUN_00633FA0`.
   - Loop/closure fields equivalent to token `+0x90/+0x94/+0x98`.
   - Exit/commit behavior equivalent to Event `0x2B` and `FUN_006385C0`.

4. Do not use normal move queue or NavQueue storage as true Planning Mode storage.

5. Keep the caps separate.
   - House/player overlay path addability: `[General] MaxWaypointPathLength`,
     default `15`, at `Rules+0x90`, plus `loop_index == -1`.
   - Per-house PlanningToken cap: `0x80` from `FUN_00639130`.
   - Overlay slots: 12 House/player path slots.

## Rust-facing Acceptance Scenario Suggestions

- `true_planning_click_creates_planning_token_not_nav_queue`:
  Enable true Planning Mode, select a unit, click a valid destination, and assert a
  per-unit planning token/command-node is created while normal movement queue state
  is not used as the authoring store.

- `planning_overlay_uses_house_waypoint_paths`:
  Seed House/player path slots and assert the planning overlay/target-line renderer
  reads those slots, not `MovementTarget.path` or `NavigationState.nav_queue`.

- `planning_path_addability_blocks_after_loop_index`:
  Seed a House/player path with `loop_index != -1` and count below
  `MaxWaypointPathLength`; assert the add action is rejected.

- `planning_path_addability_blocks_at_ini_cap`:
  Set `MaxWaypointPathLength=15`, seed 15 points and `loop_index=-1`; assert new
  point action is rejected.

- `planning_per_house_128_cap_blocks_command_add`:
  Seed per-house planning count to exceed `0x80`; assert `FUN_00639130`-equivalent
  dispatch gating blocks new true Planning Mode click authoring.

- `planning_exit_posts_event_0x2b_and_clears_active_state`:
  Exit Planning Mode and assert an event equivalent to `0x2B` is posted, then
  planning active/display selection globals are cleared.

## Negative Facts / Do Not Do

- Do not implement true Planning Mode clicks as `Foot` NavQueue appends.
- Do not implement true Planning Mode clicks as `MovementTarget.path` appends.
- Do not merge the House/player `WaypointPathClass+0x24` loop index with the
  per-unit PlanningToken loop fields `+0x90/+0x94/+0x98`.
- Do not treat `[General] MaxWaypointPathLength=15` as the 128 PlanningToken cap.
- Do not treat the 12 House/player path slots as the per-house 128 planning-node
  cap.
- Do not treat `Display+0x11BC` hover preview coordinate writes as permanent path
  point additions.
- Do not use YRpp labels or guessed names as proof of field ownership.

## Remaining Uncertainty

1. Exact active permanent append writer for `WaypointPathClass+0x2C/+0x38`.
   The layout, renderer, lookup helpers, addability predicate, and loop-index helper
   are verified, but this slot did not find the active click/EventClass caller that
   appends a new coordinate entry to the House/player path vector.

2. Active caller for `FUN_00502290`.
   `FUN_00502290` wraps the current House/player path and calls
   `FUN_00763A50` to set `WaypointPathClass+0x24`, but direct xref/caller searches
   did not find an active caller in this slice. It may be virtual, indirect, unused,
   or behind a missed function boundary.

3. `FUN_005090A0` lazy-allocation store.
   The decompiler showed allocation of an empty `WaypointPathClass` while obscuring
   the exact store into the slot. Its intended role is clear from callers and the
   sibling `FUN_00504740`, but the store should be verified in assembly before
   relying on this function for exact implementation.

4. Bridge between PlanningToken nodes and House/player overlay points.
   The active authoring writer found here stores per-unit command-node/event state.
   The mechanism that synchronizes or derives House/player overlay path points from
   that state was not proven.

## Source Anchors

- `FUN_004AC700`: display planning toggle and current path slot handling.
- `FUN_004AB9B0`: `DisplayClass::BandBox_LeftUp` dispatch gate.
- `FUN_004AE750`: `Selection__DispatchMultiUnitOrder`.
- `FUN_004C6CB0`: `EventClass::Execute`.
- `FUN_00502290`: current House/player path loop-index wrapper; no active caller
  found.
- `FUN_005023B0`: search House/player path slots for point by coordinate.
- `FUN_00502460`: resolve point pointer to path slot/index.
- `FUN_00504740`: ensure House/player `WaypointPathClass` slot exists.
- `FUN_005090A0`: find/allocate empty House/player path slot.
- `FUN_005090F0`: House/player path addability predicate.
- `FUN_006379C0`: enter Planning Mode.
- `FUN_00637A10`: exit/commit Planning Mode and queue event `0x2B`.
- `FUN_00637AA0`: Planning Mode active getter.
- `FUN_00637DD0`: planning flag for ordinary command events.
- `FUN_00637E00`: central Planning Mode event executor.
- `FUN_00638120`: active per-unit PlanningToken command-add writer.
- `FUN_00633FA0`: append command entry/node to PlanningToken.
- `FUN_00636CE0`: clear PlanningToken loop/range fields.
- `FUN_006385C0`: execute/post planned command events.
- `FUN_00639040`: planning dispatch validation gate.
- `FUN_00639130`: per-house 128 planning-node cap gate.
- `FUN_00639740`: PlanningToken loop/closure writer.
- `FUN_00639800`: PlanningToken loop/closure rewrite writer.
- `FUN_0063AD50`: right-click event `0x2A` constructor/queue path.
- `FUN_006DAD60`: House/player `WaypointPathClass` overlay renderer.
- `FUN_006FFBE0`: techno cell-order planning interception.
- `FUN_00705D10`: `Techno+0x514` PlanningToken setter.
- `FUN_00705D20`: `Techno+0x514` PlanningToken getter.
- `FUN_00763730` / `FUN_00763810`: `WaypointPathClass` constructors.
- `FUN_00763980`: `WaypointPathClass` indexed point lookup.
- `FUN_00763A50`: `WaypointPathClass+0x24` loop-index writer helper.
- `FUN_00763BA0`: next-point helper with loop-index wrap.
- `FUN_00763BE0`: `WaypointPathClass` clear helper.
- `RulesClass+0x90`: `[General] MaxWaypointPathLength`, default `15`.
