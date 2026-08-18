# MapClass::IsBridgeRampTile — Decode

**Function:** `MapClass::IsBridgeRampTile`
**Address:** `0x005746C0`
**Body range:** `0x005746C0 – 0x00574772`
**Calling convention:** `__fastcall` — `param_1` in ECX, `param_2` in EDX
**Input:**
  - `param_1` (int) — tile type index for the cell
  - `param_2` (int) — `CellClass *` pointer to the cell
**Output:** `undefined4` — returns `1` if the cell is a bridge ramp tile, `0` otherwise.
**Output doc:** `ra2-rust-game-docs/bridge-repair-mechanic/fn-IsBridgeRampTile.md`

Verified via:
- `decompile_function 0x005746C0` — full body returned
- `get_function_by_address 0x005746C0` — name and body range confirmed
- `get_function_callers 0x005746C0` — two callers identified
- `read_memory 0x00AA1548`, `0x00AA0740`, `0x00ABAD30`, `0x00ABC2B4`, `0x00AA1130`, `0x00AA1028`
  — all six DAT globals read (all zero at static time; runtime-populated tile indices)

**Active in YR:** YES — called from both `DestroyBridge_High_OnHutDeath` and
`DestroyBridge_Low_OnHutDeath` during bridge destruction. Fires on every bridge collapse.

---

## 1. Purpose

`IsBridgeRampTile` answers: "Is this cell a bridge **ramp** tile (as opposed to a flat bridge-body
tile)?" It checks a combination of tile-type index (the theater tile that fills the cell) and
sub-tile field (`CellClass + 0x11A`) to identify the six ramp configurations:

| Theater | Axis | Sub-tile (`+0x11A`) | Condition |
|---------|------|---------------------|-----------|
| Theater A ramp type 1 | (one-cell ramps) | `0x0C` (12) | Single tile match |
| Theater A ramp type 2 | (one-cell ramps) | `0x0C` (12) | Single tile match |
| Theater B ramp (4-tile set) | NS/EW | `0x04` | `DAT_00ABAD30 + 0/1/2/3` |
| Theater C ramp type 1 | (one-cell ramps) | `0x08` (8) | Single tile match |
| Theater C ramp type 2 | (one-cell ramps) | `0x08` (8) | Single tile match |
| Theater D ramp (4-tile set) | NS/EW | `0x02` | `DAT_00AA1028 + 0/1/2/3` |

The tile-type globals (`DAT_00AA1548`, `DAT_00AA0740`, etc.) are all **zero at static binary read
time** — they are runtime-populated during theater/tileset initialization when the map loads.

---

## 2. Signature

```c
undefined4 __fastcall MapClass__IsBridgeRampTile(int param_1, int param_2)
// param_1 = tile type index of cell (from theater tile table)
// param_2 = CellClass* pointer to the cell being tested
```

`__fastcall` convention: `param_1` = ECX register, `param_2` = EDX register.

Verified from decompile: function signature shows `__fastcall` with two int parameters.

---

## 3. Complete branch logic

```c
// Check 1: theater-A single-tile ramp #1, sub-tile 12 (0x0C)
if ((param_1 == DAT_00aa1548) && (*(char *)(param_2 + 0x11a) == '\f')) {
    return 1;
}
// Check 2: theater-A single-tile ramp #2, sub-tile 12 (0x0C)
if ((param_1 == DAT_00aa0740) && (*(char *)(param_2 + 0x11a) == '\f')) {
    return 1;
}
// Check 3: theater-B 4-tile ramp set, sub-tile 4 (0x04)
if ((param_1 == DAT_00abad30   ||
     param_1 == DAT_00abad30+3 ||
     param_1 == DAT_00abad30+1 ||
     param_1 == DAT_00abad30+2)
    && (*(char *)(param_2 + 0x11a) == '\x04')) {
    return 1;
}
// Check 4: theater-C single-tile ramp #1, sub-tile 8 (0x08)
if ((param_1 == DAT_00abc2b4) && (*(char *)(param_2 + 0x11a) == '\b')) {
    return 1;
}
// Check 5: theater-C single-tile ramp #2, sub-tile 8 (0x08)
if ((param_1 == DAT_00aa1130) && (*(char *)(param_2 + 0x11a) == '\b')) {
    return 1;
}
// Check 6: theater-D 4-tile ramp set, sub-tile 2 (0x02)
if ((param_1 == DAT_00aa1028   ||
     param_1 == DAT_00aa1028+3 ||
     param_1 == DAT_00aa1028+1 ||
     param_1 == DAT_00aa1028+2)
    && (*(char *)(param_2 + 0x11a) == '\x02')) {
    return 1;
}
return 0;
```

The C escape sequences used by Ghidra: `'\f'` = `0x0C`, `'\b'` = `0x08`.

---

## 4. `CellClass + 0x11A` — sub-tile index field

`*(char *)(param_2 + 0x11A)` reads a **signed byte** at `CellClass + 0x11A`. This is the
sub-tile (or "tile frame") index within the tile type — different frames of the same tile type
represent different orientations or variations. For ramp tiles:

| Sub-tile value | Ramp orientation |
|---------------|-----------------|
| `0x0C` (12) | Theater-A ramp faces (2 tile types share this) |
| `0x04` (4) | Theater-B ramp face 0–3 (4 consecutive tile types, same sub-tile) |
| `0x08` (8) | Theater-C ramp faces (2 tile types share this) |
| `0x02` (2) | Theater-D ramp face 0–3 (4 consecutive tile types, same sub-tile) |

The 4-tile sets (`DAT_00ABAD30 + 0/1/2/3` and `DAT_00AA1028 + 0/1/2/3`) cover the four cardinal
orientations of a bridge ramp in that theater — north, south, east, west faces of the ramp.

---

## 5. Runtime-populated tile globals

All six DAT globals read as zero at static binary load time. They are populated at runtime during
map/theater initialization by the tileset loader (the function that reads `.tmp` / theater `.mix`
files and fills tile-index tables). They cannot be read from the static binary.

| Global address | Runtime meaning |
|----------------|-----------------|
| `0x00AA1548` | Theater-A ramp tile #1 index |
| `0x00AA0740` | Theater-A ramp tile #2 index |
| `0x00ABAD30` | Theater-B ramp set base index (tiles +0, +1, +2, +3 = all 4 faces) |
| `0x00ABC2B4` | Theater-C ramp tile #1 index |
| `0x00AA1130` | Theater-C ramp tile #2 index |
| `0x00AA1028` | Theater-D ramp set base index (tiles +0, +1, +2, +3 = all 4 faces) |

`read_memory` at all six addresses confirmed `00 00 00 00` (zero at static time), consistent
with runtime initialization. **YELLOW: exact INI/theater keys that populate these globals are
not verified in this session.**

---

## 6. Callers

Verified via `get_function_callers 0x005746C0`:

| Caller | Address | Context |
|--------|---------|---------|
| `MapClass__DestroyBridge_High_OnHutDeath` | `0x00574000` | Checks ramp tiles during high-bridge hut-death collapse |
| `MapClass__DestroyBridge_Low_OnHutDeath` | `0x00574C20` | Checks ramp tiles during low-bridge hut-death collapse |

Both OnHutDeath functions call `IsBridgeRampTile` to determine whether adjacent cells require
ramp-specific collapse handling (ramp cells may have different collapse semantics from flat bridge
body cells).

---

## 7. Self-proof (exit gate step 4)

### Claim 1: Function at `0x005746C0` is `MapClass__IsBridgeRampTile`
`get_function_by_address 0x005746C0` → `MapClass__IsBridgeRampTile`, body `0x005746C0 – 0x00574772`. **MATCHES task spec.**

### Claim 2: Six DAT globals all zero at static time
`read_memory` at `0x00AA1548`, `0x00AA0740`, `0x00ABAD30`, `0x00ABC2B4`, `0x00AA1130`, `0x00AA1028`
all returned `00 00 00 00`. **CONSISTENT — runtime-populated, not compile-time constants.**

### Claim 3: Two callers only
`get_function_callers 0x005746C0` returned exactly 2 callers: `DestroyBridge_High_OnHutDeath @ 0x00574000`
and `DestroyBridge_Low_OnHutDeath @ 0x00574C20`. **VERIFIED — narrow use.**

---

## 8. Active-in-YR classification

| Finding | Active in YR? |
|---------|---------------|
| Called during bridge collapse on hut death | **YES** |
| 4-tile ramp set checks (DAT_00ABAD30, DAT_00AA1028) | **YES** — theater-specific, fires in all theaters |
| Single-tile ramp checks | **YES** |
| Runtime tile globals | **YES** — populated on every map load |

---

## Unverified (YELLOW)

- The INI / theater source keys that initialize the six tile globals (`0x00AA1548`, etc.) — these
  require tracing the tileset initialization code path, which is out of scope for this narrow decode.
- The "theater" labels (A/B/C/D) in §3 are interpretive — the function does not internally name
  theaters. The actual correspondence is: 2 single-tile pairs + 2 four-tile sets = 6 different
  theater ramp tile configurations, but which theater each corresponds to is not established here.

---

## Sources

**Ghidra MCP calls:**
- `decompile_function 0x005746C0`
- `get_function_by_address 0x005746C0`
- `get_function_callers 0x005746C0`
- `read_memory 0x00AA1548` (4 bytes)
- `read_memory 0x00AA0740` (4 bytes)
- `read_memory 0x00ABAD30` (4 bytes)
- `read_memory 0x00ABC2B4` (4 bytes)
- `read_memory 0x00AA1130` (4 bytes)
- `read_memory 0x00AA1028` (4 bytes)

**Prior docs cross-referenced:**
- `fn-DestroyBridge-Low-OnHutDeath.md` and `fn-DestroyBridge-High-OnHutDeath.md` (callers)
