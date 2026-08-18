# Global: DAT_00A8B274 — active_ai_count

## Summary

A 4-byte integer global at `0x00A8B274` that tracks the number of active (non-empty,
non-observer) AI player rows currently shown in dialog 0x102. Written by the
WM_COMMAND dispatcher (`FUN_006ACEE0`) during Start-button validation; read by the
same function and by other game-mode setup functions to determine whether a valid
skirmish session can be launched.

## Address

`0x00A8B274` (verified via `get_xrefs_to 0x00A8B274`)

## Type and Range

`int` (4 bytes). Value: 0 when no AI rows are active; 1–7 when AI players are
configured. The Start button is gated on this count being ≥ 1 (at least one AI
opponent required for a skirmish).

## Writers

From `get_xrefs_to 0x00A8B274`:
- `CDFileClass__Constructor @ 0x0052D195` [WRITE] — mislabeled; actually a skirmish
  session init path; sets to 0 at start of validation
- `FUN_005B8CE0 @ 0x005B8EF4` [WRITE] — increments/sets during slot scan
- `FUN_005B8CE0 @ 0x005B8EF9` [READ + WRITE] — same function
- `FUN_005E5C52` [WRITE] — another session init path
- `FUN_005DEAD2` [WRITE] — unknown context
- `FUN_006ACEE0 @ 0x006AD052` [WRITE] — WM_COMMAND dispatcher (task #2); primary
  writer in the cell-UI scope; written during Start-validation sequence
- Various other writers: `FUN_006C72F2`, `FUN_0076EE65`, `FUN_005B9705`,
  `FUN_0078F8B7`, `FUN_0079068F`, `FUN_00790ECD`, `FUN_00791381`,
  `FUN_005C30FF`, `FUN_005C3A66`

## Readers

- `FUN_006ACEE0` — WM_COMMAND dispatcher; reads to validate Start button
- `ScenarioClass__Post_Map_Init @ 0x006868A1`, `0x006868E8` — reads during map
  load to configure player/house count
- `ScenarioClass__Create_Houses @ 0x0068814C` — reads to set up house slots
- `ScenarioClass__Gather_Start_Positions @ 0x00688415` — reads for start position setup
- `FUN_00685670`, `FUN_00685DC0` — additional scene-setup readers
- Various multiplayer/session functions
(All confirmed via `get_xrefs_to 0x00A8B274`)

## Role in Start Validation

In `FUN_006ACEE0` (WM_COMMAND dispatcher, task #2), when the Start button is
clicked, the dispatcher scans the active AI rows and writes the final count to
`DAT_00A8B274`. If the count is 0 (no AI opponents), the Start action is blocked.

## Active in YR

Yes. `FUN_006ACEE0` (the primary cell-UI writer) is a core YR skirmish dialog
function (confirmed task #2).
(Confirmed via `get_xrefs_to 0x00A8B274`)

## Out-of-scope refs

- `ScenarioClass__Post_Map_Init`, `ScenarioClass__Create_Houses`,
  `ScenarioClass__Gather_Start_Positions` — map loading layer, out of cell-UI scope
- Session setup functions (`FUN_005B8CE0`, etc.) — multiplayer session layer

## Unverified (YELLOW)

- Exact condition for Start-button block: inferred from the role of this global as
  "AI count" and from `FUN_006ACEE0` writing it; the precise comparison threshold
  (≥1 or other) is not traced from the WM_COMMAND code in this session.
- Type: declared as `int` (4-byte signed); possible it is `uint` — not confirmed by
  tracing the comparison in `FUN_006ACEE0`.
