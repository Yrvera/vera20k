# Global: DAT_00A8B23C — SelectedMode_AlliesAllowed_Ptr

## Summary

A 4-byte pointer global at `0x00A8B23C` that holds a pointer to the currently
selected game mode object. Offset `+0x3C` from this pointer (i.e., `*(byte*)(DAT_00A8B23C + 0x3C)`) is the `AlliesAllowed` flag: when non-zero, team combos
are enabled; when zero, team combos are hidden/disabled.

## Address

`0x00A8B23C` (verified via `get_xrefs_to 0x00A8B23C`)

## Type

Pointer (4 bytes). Points to a game-mode object whose `+0x3C` byte field is the
`AlliesAllowed` flag.

## Assignment in Dialog Init

In `FUN_006AE6E0` (dialog init, task #1):

```c
local_4 = (int *)FUN_005e2f80();   // returns selected game mode object ptr
// ... use local_4 ...
DAT_00a8b23c = local_4;            // write to global
```

`FUN_005E2F80` returns the pointer to the active game mode / scenario settings object.
The pointer is stored to `DAT_00A8B23C` twice during init (before and after team-combo
setup).
(verified via `decompile_function 0x006AE6E0`)

## AlliesAllowed Flag

From `FUN_006AE6E0` team-combo loop:

```c
if ((DAT_00a8b23c == NULL) || ((char)DAT_00a8b23c[0xf] == '\0')) {
    uVar17 = 0xFFFFFFFE;   // random/no-team sentinel
} else {
    uVar17 = 3;            // team index 3 (default team assignment)
}
FUN_004e5ed0(uVar17);
```

`DAT_00a8b23c[0xf]` = `*(char*)(ptr + 0xF * 4)` = `*(char*)(ptr + 0x3C)` — the
`AlliesAllowed` byte at offset `+0x3C` from the mode pointer. When `AlliesAllowed`
is set, team combos get a real initial assignment; when clear, they get the random
sentinel.
(verified via `decompile_function 0x006AE6E0`)

## Readers in Cell-UI Scope

- `FUN_006AE6E0 @ 0x006AE6E0` — dialog init (task #1): reads to gate team combo init
- `FUN_006ACE80 @ 0x006ACE80` — team-enable helper: reads to gate team control enable
- `Minimap_Chat_Dispatch` — reads AlliesAllowed to control chat behavior

Out-of-scope readers: numerous — the global is used throughout the game to query
whether allied play is permitted.
(Confirmed via `get_xrefs_to 0x00A8B23C`)

## Writers

- `FUN_006AE6E0 @ 0x006AE6E0` — dialog init assigns `local_4` (from `FUN_005E2F80`)
- `SimpleWonlineDialogControl__Constructor @ 0x00789E77` — online session assignment
- Various out-of-scope session / map-load writers
(Confirmed via `get_xrefs_to 0x00A8B23C`)

## Active in YR

Yes. The primary cell-UI consumer is `FUN_006AE6E0` (dialog init, task #1) and
`FUN_006ACE80` (team-enable helper).
(Confirmed via `get_xrefs_to 0x00A8B23C`)

## Out-of-scope refs

- `FUN_005E2F80` — returns selected mode object pointer; not in cell-UI scope
- Game mode object layout — the struct at the returned pointer is part of the
  session/scenario layer, not decoded in cell-UI scope

## Unverified (YELLOW)

- `FUN_005E2F80` identity: inferred as "get selected game mode object" from its
  use as the sole source of the pointer written to `DAT_00A8B23C`; not decompiled
  in this session.
- Field name `AlliesAllowed`: inferred from context (team combo gating on this
  field, and standard YR INI key name); not confirmed by reading the game mode
  struct definition.
- Offset `+0x3C` computation: `local_4[0xf]` with `local_4` typed as `int*` means
  byte offset = `0xF * 4 = 0x3C`. Confirmed as an `int*` parameter type from the
  Ghidra decompile of `FUN_006AE6E0`.
