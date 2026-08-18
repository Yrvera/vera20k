# FUN_005E6520 — Selected-Map Player/Start-Point Count

## Summary

Returns the number of valid start positions for the currently selected map. Ghidra
mislabels this as `CDFileClass__Constructor` — the label is wrong. Identity is
confirmed via the function body: it opens the map INI, counts `[Waypoints]` entries
0–7 that have a valid (non-−1) value, and returns that count. If no waypoints are
found it falls back to `[RandomMap] NumPlayers`. If that is also 0 it returns the
hard default of 8.

Called from dialog init, the row show/hide function, and the WM_COMMAND dispatcher
to determine how many player rows to show for the selected map.

## Address

`0x005E6520` (verified via `decompile_function 0x005E6520`)

## Active in YR

Yes. In-scope callers:
- `FUN_006ACEE0 @ 0x006ACEE0` — WM_COMMAND dispatcher (task #2)
- `FUN_006ADDF0 @ 0x006ADDF0` — row show/hide (task #14)
- `FUN_006AE6E0 @ 0x006AE6E0` — dialog init (task #1)

Out-of-scope callers: `CDFileClass__Constructor @ 0x005E7BF0` (mislabeled; unrelated),
`FUN_005B8CE0`, `FUN_005DC350`, `FUN_005E64C0`, `FUN_005E7160`, `FUN_005E9E70`,
`FUN_005EB060`, `FUN_005EC030`, `FUN_005EC3A0`.
(Confirmed via `get_function_callers 0x005E6520`)

## Signature / Parameters

```c
int __fastcall FUN_005e6520(int param_1)
```

`param_1` = scenario/map index (0-based). Returns the integer start-position count.
(verified via `decompile_function 0x005E6520`)

## Behavioral Analysis

### Step 1 — Bounds check

```c
if ((-1 < param_1) && (param_1 < DAT_00a8b8d8)) {
```

`DAT_00A8B8D8` is the total map count; out-of-range `param_1` returns 0 immediately.

### Step 2 — Open map file

```c
pcVar6 = (char *)(*(int *)(DAT_00a8b8cc + param_1 * 4) + 0x58);
// ... strlen + memcpy into local_84 ...
CCFileClass__Constructor(local_84);   // opens the map file by name
```

`DAT_00A8B8CC` is a pointer array of scenario objects; each entry at `+0x58` is the
map filename string. The function copies that string and passes it to
`CCFileClass__Constructor` to open the file.
(verified via `decompile_function 0x005E6520`)

### Step 3 — Count waypoints 0–7

```c
iVar2 = 0;
iVar7 = 0;
do {
    FUN_007c8ef4(local_a4, &DAT_00817f6c, iVar7);   // sprintf(buf, format, iVar7)
    iVar3 = CCINIClass__ReadInt("Waypoints", buf, -1);
    if (iVar3 != -1) {
        iVar2 = iVar2 + 1;
    }
    iVar7++;
} while (iVar7 < 8);
```

`DAT_00817F6C` is the format string for waypoint sub-keys (likely `"%d"` or `"WP%d"`).
Each `[Waypoints]` key whose value is not −1 contributes +1 to `iVar2`.
(verified via `decompile_function 0x005E6520`)

### Step 4 — Fall back to RandomMap

```c
if (iVar2 == 0) {
    iVar2 = CCINIClass__ReadInt("RandomMap", "NumPlayers", 0);
    if (iVar2 == 0) {
        iVar2 = 8;   // hard default
    }
}
```

Only if no waypoints were found: try `[RandomMap] NumPlayers`. If that is also 0,
return 8 (maximum player count for skirmish).
(verified via `decompile_function 0x005E6520`)

### Return value

`iVar2` — the detected start-position count (1–8). Returns 0 if `param_1` is out of
range.

## Globals Referenced

| Global | Address | Role |
|---|---|---|
| `DAT_00A8B8D8` | `0x00A8B8D8` | Total scenario count (upper bound for param_1) |
| `DAT_00A8B8CC` | `0x00A8B8CC` | Scenario pointer array; each entry + 0x58 = filename |
| `DAT_00817F6C` | `0x00817F6C` | sprintf format string for waypoint sub-key names |

## Callees

Confirmed via `get_function_callees 0x005E6520`:
- `CCFileClass__Constructor @ 0x004739F0` — opens map file by filename
- `CCINIClass__ReadInt @ 0x005276D0` — reads integer INI value with default
- `FUN_007C8EF4 @ 0x007C8EF4` — sprintf wrapper (formats waypoint key names)
- `FileStraw__Constructor`, `INIClass__Constructor`, `BufferIOFileClass__Constructor`,
  `PixelBuffer_Free`, `GenericNode__Constructor`, `FUN_0040E340` — INI open/close
  bookkeeping

## Out-of-scope refs

- `DAT_00A8B8CC` scenario pointer array — wider map-selection system, out of cell-UI scope
- `DAT_00A8B8D8` scenario count — same
- `DAT_00817F6C` format string — exact bytes not read in this session (YELLOW below)

## Unverified (YELLOW)

- `DAT_00817F6C` format string: inferred as `"%d"` (integer key) from the waypoint
  index loop using iVar7=0..7; not confirmed by `read_memory` in this session.
- Exact `[Waypoints]` key format: the section name `s_Waypoints_0082db0c` is visible
  as a string literal; the sub-key suffix format is the unknown part.
- Scenario struct layout: `*(DAT_00A8B8CC + param_1*4) + 0x58` inferred as the
  filename field of a scenario descriptor; not cross-checked against the scenario
  struct layout.
