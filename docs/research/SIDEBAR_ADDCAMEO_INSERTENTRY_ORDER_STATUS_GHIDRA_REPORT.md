# SIDEBAR_ADDCAMEO_INSERTENTRY_ORDER_STATUS - Ghidra Report

Date: 2026-05-27T11:15+02:00

## Working Notes

Target question: What do `SidebarClass::AddCameo` (`0x006A6300`) and `StripClass::InsertEntry` (`0x006A8710`) prove about build-palette insertion order, duplicate handling, per-tab routing, and initial `CameoEntry` status/progress/timer fields?

Non-goals: Do not re-investigate sidebar SHP loading, Soviet chrome palettes, ready text, cameo flash non-zero setters, `StripClass::Draw`, or the full production availability caller graph.

Evidence needed to mark COMPLETE: fresh read-only Ghidra decompile of `0x006A6300` and `0x006A8710`, assembly/context evidence for routing, duplicate scan, insert position/shift, and field writes, plus direct support decompile for the comparator used by insertion.

Stop conditions: Ghidra MCP unavailable; required function boundary missing; target expands into all sidebar production; evidence cannot distinguish sorted insertion from append-only behavior.

Status: COMPLETE.

## Verified Binary Findings

### 1. AddCameo routes by RTTI into four generic strips

Active in YR: Yes, when `g_IsMapEditor == 0`. `SidebarClass::AddCameo` returns `0` immediately when the map-editor guard is set; otherwise the generic route/insert path is live when callers invoke it. Fresh evidence: `decompile_function 006a6300`; assembly contexts `0x006A6301..0x006A630E` for the map-editor guard.

`AddCameo` maps original RTTI, not Soviet/Allied side, to a strip index:

- `0x0F` / `0x10` -> strip `2` (structures).
- `0x01` / `0x28` / `0x02` / `0x03` -> strip `3` (infantry/units).
- `0x06` / `0x07` -> `RTTI_Naval_Check() == 5 ? 1 : 0` (naval aircraft to strip 1, other aircraft to strip 0).
- `0x39` / `0x20` / `0x1F` -> strip `1` (superweapons/defense).
- Other RTTI -> `0xffffffff`; no safe default tab is assigned.

Evidence: `decompile_function 006a6300`; assembly context `0x006A633D..0x006A63B7` shows the RTTI compares, the aircraft `CALL 0x005004e0`, `CMP EAX,0x5`, `SETZ`, and literal strip assignments `0x1`, `0x2`, `0x3`, `0xffffffff`.

Soviet relevance: standard Soviet buildables use the same generic RTTI route when their cameos are added. No Soviet-specific branch exists inside these two functions.

### 2. Duplicates are rejected before insertion by exact `(RTTIType, TypeIndex)` match

Active in YR: Yes, on the non-map-editor AddCameo path.

After routing to a strip, `AddCameo` loads that strip count and rejects if the current count is above `0x4B`. It then linearly scans existing entries from `strip + 0x58` in `0x34`-byte steps and returns `0` if both fields match:

- existing `RTTIType` at entry `+0x04` equals the incoming RTTI.
- existing `TypeIndex` at entry `+0x00` equals the incoming type index.

Evidence: `decompile_function 006a6300`; assembly contexts `0x006A63D6..0x006A63D9` for `count > 0x4B` rejection, and `0x006A63E5..0x006A63FB` for `LEA [strip+0x58]`, `CMP [EAX+4], ESI`, `CMP [EAX], EBP`, return-on-match, then `ADD EAX,0x34`.

Support wrapper `FUN_006A87F0` repeats the same duplicate and count policy for a direct strip add wrapper, then calls `InsertEntry`; fresh decompile found it, but this slot did not prove current YR callers for that wrapper.

### 3. InsertEntry is sorted insertion, not append-only and not raw rules-array order

Active in YR: Yes, for every successful `AddCameo` insertion because `AddCameo` directly calls `0x006A8710`.

`InsertEntry` increments `StripClass +0x54` first, then scans from `strip +0x58` until `SidebarClass__CompareItems(new_rtti, new_type, existing_rtti, existing_type)` returns true. If the comparator returns false, it advances by one `0x34`-byte entry. If it reaches the new count without finding an insert point, it returns after having incremented the count; otherwise it shifts later entries down by copying `13` dwords (`0x34` bytes) per slot and writes the new entry into the found slot.

Evidence: `decompile_function 006a8710`; assembly contexts:

- `0x006A8719..0x006A8726`: load count, increment, store `+0x54`.
- `0x006A8742..0x006A875D`: call comparator, continue while false, `ADD ESI,0x34`, early return if scan reaches count.
- `0x006A8760..0x006A8789`: compute shift count and shift entries using `0x0D` dwords per entry.
- `0x006A8798..0x006A87DA`: write the new entry fields.

The comparator at `0x006A8420` proves the order is comparator-driven. It treats existing RTTI `0` as an insertion sentinel, has a special superweapon group `{0x1F,0x39,0x20}` ordered by `SuperWeaponTypeClass +0xB0` then `+0x60` string/name pointer, prefers items matching the player's side/owner field before nonmatching items for ordinary nonsuper entries, applies infantry/unit flag subordering through type flags at `+0xD96` and `+0xCCE`, then compares TypeClass `+0x634`, then a virtual `+0x84(g_PlayerPtr)` value, then TypeClass `+0x60` through `FUN_007CA5D3`.

Evidence: `decompile_function 006a8420`; assembly contexts `0x006A8448..0x006A8460` for existing RTTI zero sentinel, `0x006A8470..0x006A84E1` for superweapon grouping and `+0xB0/+0x60` comparisons, `0x006A84E6..0x006A8523` for player-side comparison against TypeClass `+0x6D0`, and `0x006A8682..0x006A86F5` for `+0x634`, vtable `+0x84`, and `+0x60` tiebreak.

### 4. InsertEntry initializes only part of CameoEntry state

Active in YR: Yes, for successful insertions through `0x006A8710`.

For the inserted entry at `strip + slot*0x34 + 0x58`, `InsertEntry` writes:

- `+0x00 TypeIndex = param_3`.
- `+0x04 RTTIType = param_2`.
- `+0x08 NavalCheck = RTTI_Naval_Check()` only when `RTTIType == 7`; otherwise this field is not written by `InsertEntry`.
- `+0x0C FactoryPtr = 0`.
- `+0x10 Status = 0`.
- `+0x14 ProgressValue = 0`.
- `+0x1C CameoTimer.StartTime = g_CurrentFrameCounter`.
- `+0x20 CameoTimer.pad = uninitialized local`.
- `+0x24 CameoTimer.TimeLeft = 0`.
- `+0x28 CameoTimer.Duration = 0`.
- `+0x30 FlashEndFrame = 0`.

`+0x18 IsProgressingThisTick` and `+0x2C StepIncrement` are not explicitly written by `InsertEntry` in the fresh decompile; the canonical struct doc already warns these are not initialized by InsertEntry.

Evidence: `decompile_function 006a8710`; assembly contexts `0x006A8798..0x006A87DA` for field writes, including conditional aircraft call `0x006A87A1..0x006A87AA`, zero writes at `+0x64`, `+0x68`, `+0x6C`, `+0x7C`, `+0x80`, `+0x88`, and frame counter write through `+0x74`.

### 5. AddCameo dirties UI state after insertion but does not set progress/status

Active in YR: Yes, after successful insertion.

After `CALL 0x006A8710`, `AddCameo` sets the owning strip dirty byte at `strip +0x3C`, sets global redraw/dirty flags, may start the tab flash animation when sidebar SHP animation data exists and all strips were previously empty, updates scroll buttons if the inserted tab is active, and may switch active tab if there was no previous populated tab. None of this writes the inserted entry's status, factory pointer, progress, timer duration, step increment, or flash end frame.

Evidence: `decompile_function 006a6300`; assembly context `0x006A641A..0x006A6429` for `CALL 0x006A8710`, strip dirty byte, and `DAT_00884B8F = 1`; contexts `0x006A6432..0x006A6467` for populated-strip scan/tab-flash branch.

## Negative Facts / Do Not Do

- Do not append new visible cameos in rules-array/build-option order; native `InsertEntry` uses `CompareItems` and shifts the array at the first comparator-true slot. Evidence: `0x006A8742..0x006A8789`.
- Do not merge entries by display name, cameo image, category, or queue kind. Duplicate rejection is exact `(RTTIType, TypeIndex)`. Evidence: `0x006A63E5..0x006A63EF`.
- Do not initialize a new cameo as building, ready, on-hold, or with progress already active. `InsertEntry` writes `Status = 0`, `FactoryPtr = 0`, `ProgressValue = 0`, `Duration = 0`. Evidence: `0x006A8798..0x006A87DA`.
- Do not implement cameo-level "new item" flash from `AddCameo`. Fresh `AddCameo` and `InsertEntry` only leave `FlashEndFrame = 0`; this matches `CAMEO_FLASH_END_FRAME_WRITER_GHIDRA_REPORT.md`. Evidence: `0x006A87DA`.
- Do not add Soviet-specific sorting or routing inside the Rust equivalent of these functions. The binary route is RTTI/type/player-side based, not sidebar theme based. Evidence: `0x006A633D..0x006A63B7`, `0x006A8420` comparator decompile.

## Implementation Handoff

1. Verified behavior: successful `AddCameo` inserts into a persistent per-strip `CameoEntry` array through comparator-sorted insertion and exact duplicate rejection. Rust delta: `src/sidebar/sidebar_view.rs` currently rebuilds transient `BuildEntry` lists from `build_options`, queue items, ready buildings, and superweapon views, with superweapons simply prepended and no native `CompareItems` equivalent. Affected surface: sidebar ordering and duplicate/ready merge behavior. Acceptance scenario: reveal several Soviet structure/unit buildables in a non-rules order and verify the visible order matches `CompareItems`, not `rulesmd.ini` iteration. Proposed test: `test_sidebar_addcameo_uses_native_compare_order_for_soviet_buildables`. Risk: high for screenshot/UI parity because every build palette is ordered by this path.

2. Verified behavior: insertion initializes `Status=0`, `FactoryPtr=0`, `ProgressValue=0`, timer duration `0`, and only later production links can make progress visible. Rust delta: separate buildable availability from active queue/ready state instead of deriving the initial visible entry directly from current queue progress. Affected surface: `SidebarItem.progress`, `is_building_this_type`, `is_ready`, `queued_count` construction in `src/sidebar/sidebar_view.rs`. Acceptance scenario: a newly available Soviet buildable appears with no progress/ready overlay until production starts. Proposed test: `test_sidebar_newly_available_cameo_starts_empty_until_factory_link`. Risk: medium-high because current transient view can conflate availability, queue state, and completed building state.

3. Verified behavior: duplicate scan keys by `(RTTIType, TypeIndex)` within the routed strip. Rust delta: when a persistent model is added, keep RTTI/type-index identity instead of using strings/cameo ids alone for sidebar entry identity. Affected surface: future sidebar model, `BuildEntry` collection, and superweapon/buildable merge logic. Acceptance scenario: two entries with distinct RTTI but colliding display/cameo names do not incorrectly collapse, while the exact same RTTI/type pair is rejected. Proposed test: `test_sidebar_duplicate_key_is_rtti_and_type_index_not_cameo_id`. Risk: medium for mods and superweapon/buildable collisions.

## Remaining Uncertainty

- This slot did not exhaustively trace every caller that causes ordinary Soviet buildables to become available; it proved the requested functions' generic behavior and found fresh caller evidence for `HouseClass::AI_ResumeProduction` invoking `AddCameo(0x1f, index)` for player superweapons.
- `CompareItems` field names are inferred from established TypeClass layout conventions. The exact semantic names of TypeClass `+0x634`, `+0x6D0`, `+0xD96`, `+0xCCE`, and vtable `+0x84` should be named from the canonical TypeClass docs before code implementation.
- `InsertEntry`'s pre-increment plus scan-to-new-count early return is a native edge behavior. Standard stock strips are not expected to hit the full boundary in normal Soviet play, but exact overflow-safe Rust modeling needs a separate decision.

## Stale-Doc Replacement Wording

- `docs/research/SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md`, AddCameo flow bullet 4e: replace "Sets CameoEntry.FlashTimer for 'new item' flash effect" with "Initializes `CameoEntry.FlashEndFrame` (`+0x30`) to 0; fresh AddCameo/InsertEntry evidence found no non-zero cameo-level new-item flash setter."
- `docs/research/SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md`, Cameo Sort Order: replace the compressed "Priority order" list with "Order is determined by `SidebarClass__CompareItems` (`0x006A8420`): existing RTTI 0 sentinel inserts before empty slots; superweapon group `{0x1F,0x39,0x20}` has a special `SuperWeaponTypeClass +0xB0` then `+0x60` order; ordinary entries prefer player-side matches through TypeClass `+0x6D0`, then apply unit/infantry flag subordering, TypeClass `+0x634`, vtable `+0x84(g_PlayerPtr)`, and TypeClass `+0x60` string/name tiebreak."

## Source Cross-Checks

- `docs/research/FACTORYCLASS_AND_CAMEOENTRY_STRUCT_LAYOUT.md` already matches the field-layout finding for `+0x28` duration and `+0x30` FlashEndFrame.
- `docs/research/CAMEO_FLASH_END_FRAME_WRITER_GHIDRA_REPORT.md` matches the no-nonzero-writer finding.
- `src/sidebar/sidebar_view.rs` currently constructs transient lists in `collect_build_entries`, so it does not yet model a native persistent `CameoEntry` array or `CompareItems` insertion.
