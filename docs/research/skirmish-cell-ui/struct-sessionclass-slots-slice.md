# SessionClass — Slots-Slice Struct Layout

## Summary

Documents the per-slot data structures accessed by the Skirmish dialog
cell-UI functions. Three related structures are described:

1. **Slot entry struct** — objects pointed to by `DAT_00A8DA90[i]` (8 entries);
   cell-relevant field: `+0x6B` (closed/inactive flag).
2. **Player entry struct** — 0x85-byte objects allocated per active player
   in `FUN_006ACEE0` (0x006ACEE0); fields for startpos, color, team, type.
3. **DAT_00A8B3EC persistence array** — 7-slot × 3-dword (stride 12) array
   written by `FUN_006ACEE0` on game-start; fields for PlayerType, Country, StartPos.

Base addresses are inferred from decompilation of `FUN_006AE6E0` (0x006AE6E0)
and `FUN_006ACEE0` (0x006ACEE0).

## Active in YR

**Yes.** All three structures are live in YR offline Skirmish.

(confirmed via `decompile_function 0x006ACEE0` and `decompile_function 0x006AE6E0`)

---

## 1. Slot Entry Struct (DAT_00A8DA90[i] pointer targets)

`DAT_00A8DA90` at `0x00A8DA90` is an 8-element array of pointers to slot entry
structs. `DAT_00AC11B4` is the "null/absent" sentinel for this array.

The local player's slot pointer is `DAT_00AC11B4` (used as closed-slot test):
```c
if ((&DAT_00a8da90)[iVar2] == DAT_00ac11b4 || *(int *)((&DAT_00a8da90)[iVar2] + 0x6b) == -1)
    // slot is closed/spectator → use sentinel path
```

(verified via `decompile_function 0x004E49A0`)

### Cell-relevant fields

| Offset | Type | Role | Value semantics |
|--------|------|------|-----------------|
| `+0x6B` | int32 | Closed/inactive flag | -1 = closed or spectator slot |

`+0x6B` is the only slot-entry struct field accessed directly by the cell-UI
functions decoded in this task set. All other per-slot data goes through
the player entry struct (see section 2) or the persistence array (section 3).

---

## 2. Player Entry Struct (0x85 bytes, allocated per active player)

Allocated via `operator_new(0x85)` in `FUN_006ACEE0`. Stored in a dynamic
array at `SessionClass+0x2840` (pointer) / `SessionClass+0x284C` (count).

Constructor write sites (verified via `decompile_function 0x006ACEE0`):

```c
pvVar9 = operator_new(0x85);
*(undefined4 *)((int)pvVar9 + 0x4b) = DAT_00a8b3ac;  // field at +0x4B
*(undefined4 *)((int)pvVar9 + 0x53) = DAT_00a8b394;  // field at +0x53
*(undefined4 *)((int)pvVar9 + 0x5b) = DAT_00a8b39c;  // field at +0x5B
*(undefined4 *)((int)pvVar9 + 99)   = DAT_00a8b3a4;  // field at +0x63
*(undefined4 *)((int)pvVar9 + 0x73) = 0xffffffff;    // field at +0x73
```

### Cell-relevant fields

| Offset (dec) | Offset (hex) | Source global | Role |
|---|---|---|---|
| 75 | `+0x4B` | `DAT_00A8B3AC` | Unknown (color? type?) |
| 83 | `+0x53` | `DAT_00A8B394` | Color index (written by ProcessRandomAssignments) |
| 87 | `+0x57` | — | Random startpos flag (read by FUN_0069B7E0: `+0x57 == -2` test) |
| 91 | `+0x5B` | `DAT_00A8B39C` | StartPos index |
| 99 | `+0x63` | `DAT_00A8B3A4` | Team index |
| 115 | `+0x73` | — | Initialized to -1; role unknown |

### FUN_0069B7E0 reads from player list entry (confirmed via `decompile_function 0x0069B7E0`):

```c
piVar4 = *(int **)(param_1 + 0x2840);  // player list pointer
do {
    iVar2 = *piVar4;                    // player entry pointer
    if ((*(int *)(iVar2 + 0x57) == -2) && (*(int *)(iVar2 + 0x53) == -1)) {
        iVar2 = -2;  // random startpos sentinel
    } else {
        iVar2 = *(int *)(iVar2 + 0x53);  // startpos index
    }
    ...
}
```

Confirms:
- `player_entry + 0x53` = startpos committed value (-1 = random)
- `player_entry + 0x57` = random-startpos flag (-2 = random mode)

---

## 3. DAT_00A8B3EC — Session Persistence Array (PlayerType/Country/StartPos)

Written by `FUN_006ACEE0` on game-start, read by `FUN_006AE6E0` on dialog
re-open. 7 AI slots × 3 dwords stride = 21 dwords total (84 bytes).

### Base addresses

| Slot | PlayerType addr | Country addr | StartPos addr |
|------|----------------|--------------|---------------|
| 0 (AI slot) | `0x00A8B3EC` | `0x00A8B3F0` | `0x00A8B3F4` |
| 1 | `0x00A8B3F8` | `0x00A8B3FC` | `0x00A8B400` |
| 2 | `0x00A8B404` | `0x00A8B408` | `0x00A8B40C` |
| 3 | `0x00A8B410` | `0x00A8B414` | `0x00A8B418` |
| 4 | `0x00A8B41C` | `0x00A8B420` | `0x00A8B424` |
| 5 | `0x00A8B428` | `0x00A8B42C` | `0x00A8B430` |
| 6 | `0x00A8B434` | `0x00A8B438` | `0x00A8B43C` |

(`DAT_00A8B3F0` = `&array[0].country`; confirmed as write base from `decompile_function 0x006ACEE0`)

### Field semantics (per slot dword[3])

| Word | Role | Encoding | Source |
|------|------|---------|--------|
| `[0]` (`puVar21[-1]`) | PlayerType | 1=Human, 4=Easy, 5=Medium, 6=Hard | From AI-combo item-data: -1→1, 0→4, 1→5, 2→6 |
| `[1]` (`*puVar21`) | Country index | 0–9 or -1/–2 | `FUN_004E4170(0xffffffff)` |
| `[2]` (`puVar21[1]`) | StartPos index | 0–8 or -1/–2 | `FUN_004E4E20(0xffffffff)` |

(verified via `decompile_function 0x006ACEE0`)

FUN_006AE6E0 reads back these fields to restore the AI-type combo selection:
```c
local_10 = &DAT_00a8b3f0;  // → &array[0].country
iVar5 = local_10[-1];      // PlayerType for slot 0
```

---

## 4. SessionClass Object — Session Writer Fields

Written by `FUN_0069B760` (country writer) and `FUN_0069B7E0` (startpos writer)
called on the **local player's session object** (the `this` pointer).

### Country fields (FUN_0069B760, verified via `decompile_function 0x0069B760`)

| Offset | Role | Semantics |
|--------|------|-----------|
| `+0x174` | Country committed | Committed country index (mirrored from working) |
| `+0x178` | Country random flag (committed) | -2 = random, -1 = fixed |
| `+0x184` | Country working | Current UI selection |
| `+0x188` | Country random flag (working) | -2 = random, -1 = fixed |

### StartPos fields (FUN_0069B7E0, verified via `decompile_function 0x0069B7E0`)

| Offset | Role | Semantics |
|--------|------|-----------|
| `+0x15C` | StartPos committed | Committed start-pos index (mirrored) |
| `+0x160` | StartPos random flag (committed) | -2 = random, -1 = fixed |
| `+0x17C` | StartPos working | Current UI selection |
| `+0x180` | StartPos random flag (working) | -2 = random, -1 = fixed |

### StartPos occupied array (FUN_0069B7E0)

| Offset | Role |
|--------|------|
| `+0x84..+0xA0` | 8-dword array of occupied start positions (searched during random assignment) |

---

## Globals Referenced

| Global | Address | Role |
|--------|---------|------|
| `DAT_00A8DA90` | 0x00A8DA90 | Array of 8 slot entry pointers |
| `DAT_00AC11B4` | 0x00AC11B4 | Null/absent slot sentinel |
| `DAT_00A8B3EC` | 0x00A8B3EC | Persistence array base (PlayerType slot 0) |
| `DAT_00A8B3F0` | 0x00A8B3F0 | Persistence array country slot 0 (Ghidra label) |
| `DAT_00A8B394` | 0x00A8B394 | Color global (written by ProcessRandomAssignments) |
| `DAT_00A8B39C` | 0x00A8B39C | StartPos global |
| `DAT_00A8B3AC` | 0x00A8B3AC | Unknown global (written at player_entry+0x4B) |
| `DAT_00A8B3A4` | 0x00A8B3A4 | Team global |

## TS-filter

All structures are accessed by live YR dialog code. **TS-legacy score: 0.0.**

## Unverified (YELLOW)

- `player_entry + 0x4B` → `DAT_00A8B3AC`: role not confirmed. Adjacent to
  `+0x53` (color/startpos); possibly player name pointer or an AI difficulty field.
- `player_entry + 0x53` as "color" vs "startpos": `DAT_00A8B394` is written by
  `SessionClass__ProcessRandomAssignments`; the assignment to `pvVar9+0x53` and
  later `iVar2 + 0x53` in FUN_0069B7E0 (where it's tested against `-1` as startpos
  sentinel) suggests `+0x53` = **startpos committed**. Color may be at `+0x5B`.
  The exact mapping of globals `DAT_00A8B394` (color) vs `DAT_00A8B39C` (startpos)
  to player_entry offsets +0x53 and +0x5B was not independently verified by
  cross-referencing ProcessRandomAssignments in this session — see table note.
- Slot entry struct size: only `+0x6B` has been verified as a cell-UI-accessed field;
  total struct size and other fields are unknown in this decode scope.
- `DAT_00A8B3EC` = PlayerType slot 0: computed as `DAT_00A8B3F0 - 4`; the actual
  Ghidra label for `0x00A8B3EC` was not verified (no `get_xrefs_to 0x00A8B3EC` call).
