# Country-Icon SHP Selector — Ghidra Research Report

**Primary function:** `StripClass__Draw` at `0x006a9540` (observer-mode branch starting `0x006aa059`)
**Switch address:** `0x006aa176`
**Load function:** `SidebarClass__LoadSHPs` at `0x006a5840`
**Confidence:** HIGH — full assembly verified via `disassemble_function 0x006aa140`, `decompile_function 0x006aa185`, read_memory at `0x006aa5c4`, xrefs to all 15 SHP globals
**Active in YR:** Yes — fires every frame observer mode is active and any player row is visible

---

## 1. Overview

In observer mode, `StripClass__Draw` iterates one row per player. For each row it draws two icons: (1) a **side icon** (OBSALLI/OBSSOVI/OBSYURI) keyed on `CountryTypeClass.Side` at offset `+0xBC`, and (2) a **country icon** from one of 10 SHP globals keyed on `CountryTypeClass.self_index` at offset `+0xB8`. The selector for the country icon is a compiled jump table at `0x006aa5c4`, dispatching on the country's 0-based index in the global `[Countries]` array. Only countries 0-9 produce icons; indices 10+ (GDI/Nod/Neutral/Special) hit the default path and draw nothing.

---

## 2. SHP Global → Filename Mapping (Verified)

Each global is written by `SidebarClass__LoadSHPs` at `0x006a5840`. Verified via `get_xrefs_to` on each filename string and each global address.

| Global Address | SHP Filename | Load site in LoadSHPs | String address |
|---|---|---|---|
| `0x00b0b490` | OBSALLI.SHP | `0x006a5ab7` | `0x0083f9e0` |
| `0x00b0b494` | OBSSOVI.SHP | `0x006a5ac6` | `0x0083f9d4` |
| `0x00b0b498` | OBSYURI.SHP | `0x006a5ada` | `0x0083f9c8` |
| `0x00b0b49c` | RANI.SHP | `0x006a5aee` | `0x0083f9bc` |
| `0x00b0b4a0` | OBSI.SHP | `0x006a5b02` | `0x0083f9b0` |
| `0x00b0b4a4` | USAI.SHP | `0x006a5b16` | `0x0083f9a4` |
| `0x00b0b4a8` | JAPI.SHP | `0x006a5b2a` | `0x0083f998` |
| `0x00b0b4ac` | FRAI.SHP | `0x006a5b3e` | `0x0083f98c` |
| `0x00b0b4b0` | GERI.SHP | `0x006a5b52` | `0x0083f980` |
| `0x00b0b4b4` | GBRI.SHP | `0x006a5b66` | `0x0083f974` |
| `0x00b0b4b8` | DJBI.SHP | `0x006a5b7a` | `0x0083f968` |
| `0x00b0b4bc` | ARBI.SHP | `0x006a5b8e` | `0x0083f95c` |
| `0x00b0b4c0` | LATI.SHP | `0x006a5ba2` | `0x0083f950` |
| `0x00b0b4c4` | RUSI.SHP | `0x006a5bb6` | `0x0083f944` |
| `0x00b0b4c8` | YRII.SHP | `0x006a5bca` | `0x0083f938` |

**Verdict:** The ordering in SIDEBAR_SYSTEM_GHIDRA_REPORT.md §13 is CORRECT for all 15 entries.

---

## 3. Country Icon Selector — Complete Switch

### Switch key source

The selector reads the switch key from `CountryTypeClass.self_index` at offset **`+0xB8`** (second self-index field in CountryTypeClass). This is the country's 0-based position in `g_HouseTypeClass_Array` / `[Countries]`.

Chain verified from assembly (`disassemble_function 0x006aa140`):

```
006aa0aa: MOV ESI, [EBP*4 + 0x884b94]  ; HouseClass* from per-player array
006aa0bf: MOV EAX, [ESI+0x34]           ; CountryTypeClass* from HouseClass+0x34
006aa0c2: TEST EAX,EAX
006aa0c4: JZ  006aa159                  ; null CountryTypeClass → skip both icons
006aa159: MOV EAX, [ESI+0x34]           ; (re-read for country switch path)
006aa15c: TEST EAX,EAX
006aa15e: JZ  006aa2ce                  ; null → no country icon
006aa164: MOV EAX, [EAX+0xb8]           ; CountryTypeClass.self_index (+0xB8)
006aa16a: LEA ECX, [EAX+3]              ; normalize: add 3 to make -3→0, ..., 9→12
006aa16d: CMP ECX, 0xc                  ; range check: ecx > 12 → default
006aa170: JA  006aa2ce                  ; out of range → no country icon
006aa176: JMP [ECX*4 + 0x6aa5c4]        ; jump table dispatch
```

### Jump table at `0x006aa5c4`

Verified by `read_memory 0x006aa5c4 52`. Table is 13 entries × 4 bytes, covering ECX=0..12 (EAX=-3..9):

| ECX | EAX (self_index) | Jump target | Global read | SHP | Country |
|-----|---|---|---|---|---|
| 0 | -3 | `006aa1cd` | `DAT_00b0b4a0` | OBSI.SHP | Observer slot |
| 1 | -2 | `006aa17d` | `DAT_00b0b49c` | RANI.SHP | Random country slot |
| 2 | -1 | `006aa2ce` | (none) | — | No icon (fallthrough) |
| 3 | 0 | `006aa185` | `DAT_00b0b4a4` | USAI.SHP | Americans |
| 4 | 1 | `006aa18d` | `DAT_00b0b4a8` | JAPI.SHP | Alliance (Korea) |
| 5 | 2 | `006aa195` | `DAT_00b0b4ac` | FRAI.SHP | French |
| 6 | 3 | `006aa19d` | `DAT_00b0b4b0` | GERI.SHP | Germans |
| 7 | 4 | `006aa1a5` | `DAT_00b0b4b4` | GBRI.SHP | British |
| 8 | 5 | `006aa1ad` | `DAT_00b0b4b8` | DJBI.SHP | Africans |
| 9 | 6 | `006aa1b5` | `DAT_00b0b4bc` | ARBI.SHP | Arabs |
| 10 | 7 | `006aa1bd` | `DAT_00b0b4c0` | LATI.SHP | Confederation |
| 11 | 8 | `006aa1c5` | `DAT_00b0b4c4` | RUSI.SHP | Russians |
| 12 | 9 | `006aa1d5` | `DAT_00b0b4c8` | YRII.SHP | YuriCountry |

Countries with `self_index >= 10` (GDI=10, Nod=11, Neutral=12, Special=13) hit the `JA` out-of-range check at `0x006aa170` and draw no country icon.

### Switch key field: CountryTypeClass+0xB8 vs +0xB4

CountryTypeClass has **two** self-index fields (verified from `COUNTRY_SIDE_TYPE_CLASSES.md` + `HouseTypeClass__Constructor` at `0x005113f0`):
- `+0xB4` (`param_1[0x2d]` in constructor) — index from first search loop
- `+0xB8` (`param_1[0x2e]` in constructor) — index from second search loop; **this is the switch key**

Both are set to the same position in `g_HouseTypeClass_Array`, but only `+0xB8` is read by the country icon switch. The constructor at `0x005113f0` confirms: `param_1[0x2e] = iVar4` (position in array) or `0xffffffff` (-1) if not found.

---

## 4. Side Icon Selector (separate path, same function)

The side icon draw runs BEFORE the country icon draw, at `0x006aa0ca`:

```
006aa0ca: MOV EAX, [EAX+0xbc]   ; CountryTypeClass.Side (+0xBC)
006aa0d0: SUB EAX, ECX           ; EAX - 0 (ecx=0)
006aa0d2: JZ  006aa0e8           ; side==0 → DAT_00b0b490 (OBSALLI)
006aa0d4: DEC EAX
006aa0d5: JZ  006aa0e1           ; side==1 → DAT_00b0b494 (OBSSOVI)
006aa0d7: DEC EAX
006aa0d8: JNZ 006aa159           ; side>=3 → skip side icon (jump to country icon)
006aa0da: MOV EAX,[0x00b0b498]   ; side==2 → OBSYURI
```

- Side 0 → OBSALLI.SHP (`DAT_00b0b490`)
- Side 1 → OBSSOVI.SHP (`DAT_00b0b494`)
- Side 2 → OBSYURI.SHP (`DAT_00b0b498`)
- Side ≥ 3 → no side icon

Side icon drawn with: `CC_Draw_Shape(iVar17, 0, &(row_x, row_y), &rect, 0x400, ...)` — frame 0, no centering offset (drawn at raw row coords). Uses palette stored in `DAT_0087f6d0` (CAMEO palette group).

---

## 5. Country Icon Draw Details

### Draw position

Both the side icon and country icon share the same `(iVar10, iVar8)` base = observer row origin. The country icon is drawn at:
- X = `row_x + shp_xhotspot + 0x46`  (i.e., +70 pixels from row origin)
- Y = `row_y + shp_yhotspot + 0x46`  (i.e., +70 pixels from row origin)

Verified from assembly `0x006aa254`-`0x006aa28e`:
```
006aa254: MOVSX ECX, word ptr [ESI+0x2]   ; SHP header: xhotspot
006aa25a: LEA EDX, [ECX + EDI + 0x46]     ; X = row_x + xhotspot + 70
006aa265: MOVSX ECX, word ptr [ESI+0x4]   ; SHP header: yhotspot
006aa27b: ...
006aa287: MOV [ESP+0xb0], EDX             ; Y = row_y + yhotspot + 70
```

### Special case for YRII (case 9)

YRII icon calls `FUN_0072f4d0` **before** drawing (`0x006aa1e8`). This function loads/activates the YRII palette (`YRII.PAL` at `0x008453e8`). No other country icon makes this call. After the palette call, draw proceeds identically to non-YRII cases (same +0x46 offset).

```
006aa1e3: CMP EAX, 0x9   ; check if case == 9 (YRII)
006aa1e6: JNZ 006aa254   ; non-YRII → skip palette call
006aa1e8: CALL 0x0072f4d0 ; YRII palette activation
006aa1ed: MOVSX EDX, [ESI+0x2] ; then draw normally
```

### CC_Draw_Shape flags

Country icon drawn with `CC_Draw_Shape(iVar15, 0, &pos, &rect, 0x400, 0, 0, 0, 1000, ...)`:
- Flags: `0x400` (standard 2D draw, no depth/shadow/tint)
- Frame: 0 (always frame 0)
- Z param: 1000

---

## 6. Null Safety

Two null-pointer guards verified in assembly:

1. **HouseClass null check** (`0x006aa0b3`): If `(&DAT_00884b94)[iVar18]` == 0, skip entire player row.
2. **CountryTypeClass null check** (`0x006aa15e`): If `HouseClass+0x34` == 0, skip country icon (jump to `0x006aa2ce`).
3. **SHP pointer null check** (`0x006aa1db`): After resolving `iVar15` from the switch, `TEST ESI,ESI; JZ 006aa2ce` — if the loaded SHP pointer is null (file not found), skip draw silently.

---

## 7. INI Relationship

The switch key comes from `CountryTypeClass.self_index (+0xB8)` which equals the country's 0-based position in the `[Countries]` list in `rulesmd.ini`:

```ini
[Countries]
0=Americans   → USAI (switch case 0)
1=Alliance    → JAPI (switch case 1)
2=French      → FRAI (switch case 2)
3=Germans     → GERI (switch case 3)
4=British     → GBRI (switch case 4)
5=Africans    → DJBI (switch case 5)
6=Arabs       → ARBI (switch case 6)
7=Confederation → LATI (switch case 7)
8=Russians    → RUSI (switch case 8)
9=YuriCountry → YRII (switch case 9)
10=GDI        → (no icon, out of range)
11=Nod        → (no icon, out of range)
12=Neutral    → (no icon, out of range)
13=Special    → (no icon, out of range)
```

**Critical:** the SHP selection is NOT keyed on `Side=`, `Color=`, `Suffix=`, or `Prefix=`. It is keyed exclusively on the country's array index. Reordering `[Countries]` entries would silently swap icons. Stock YR never does this.

---

## 8. Callers and Context

Each country icon SHP global is read from exactly 3 callsites:
1. **Write:** `SidebarClass__LoadSHPs` (`0x006a5840`) — initial SHP load
2. **Read:** `StripClass__Draw` (`0x006a9540`) — observer mode draw
3. **Read (free):** `SidebarClass__FreeSHPs` — cleanup

No other function reads these globals. The country icon draw path is exclusive to observer mode (`g_PlayerPtr == DAT_00ac1198`). Non-observer play does not call this code path (verified: `CMP EBX, EBP; JNZ 0x006aa59b` at `0x006aa04f` skips the entire observer block).

**Active in YR:** Yes, conditional. Active only when observer mode is in use (standard skirmish: never; multiplayer with observer: yes).

---

## 9. Special Index Values (-3, -2)

Cases -3 and -2 map to OBSI and RANI. These are not standard `[Countries]` indices. They must be set externally (e.g., network lobby assigns a synthetic "observer" HouseClass with `CountryTypeClass.self_index = -3`, or a "random" country slot with -2). The constructor at `0x005113f0` initializes `param_1[0x2e]` = -1 by default if not found in the array; -3 and -2 must be assigned by lobby/setup code outside the constructor.

**Investigation scope boundary:** tracing where -3 and -2 are assigned to `CountryTypeClass+0xB8` is out of scope for this investigation; the switch behavior is fully characterized.

---

## 10. Open Questions — Final State

- `[RESOLVED] Q1` — Which global holds each country SHP? → Full table in §2, all 15 globals verified. (evidence: `get_xrefs_to` on each string + `disassemble_function 0x006aa140`)
- `[RESOLVED] Q2` — What is the switch key? → `CountryTypeClass.self_index (+0xB8)`, normalized by +3 for the jump table. (evidence: `0x006aa164`, `disassemble_function 0x006aa140`)
- `[RESOLVED] Q3` — Are -3 and -2 handled? → Yes, case -3 → OBSI, case -2 → RANI. (evidence: jump table `0x006aa5c4`, targets `0x006aa1cd` and `0x006aa17d`)
- `[RESOLVED] Q4` — What happens for indices > 9? → Out-of-range check at `0x006aa170` (CMP ECX,0xC; JA) → no icon drawn.
- `[RESOLVED] Q5` — Is the draw path active in non-observer play? → No, guarded by `g_PlayerPtr == DAT_00ac1198` check at `0x006aa04f`.
- `[RESOLVED] Q6` — Does YRII have any special behavior? → Yes: calls `FUN_0072f4d0` (YRII.PAL activation) at `0x006aa1e8` before drawing. No other country icon does this.
- `[RESOLVED] Q7` — What are the draw offsets? → +0x46 (70 pixels) on both X and Y from row origin, plus SHP header hotspot. Verified `0x006aa254`.
- `[RESOLVED] Q8` — What are the CC_Draw_Shape flags? → `0x400`, frame 0, Z=1000.
- `[RESOLVED] Q9` — Side icon vs country icon selector field? → Side: `CountryTypeClass+0xBC`; Country: `CountryTypeClass+0xB8`. Different offsets, different fields.
- `[RESOLVED] Q10` — Doc table (SIDEBAR_SYSTEM §13) correct? → Yes, all 15 entries verified as matching the binary.
- `[DEFERRED] Q11` — Where are self_index values -3 and -2 assigned? (category: `out-of-scope`; reason: requires tracing multiplayer lobby/network setup code; next-step: search for `MOV [...+0xb8], -3` or `MOV [...+0xb8], 0xFFFFFFFD` patterns in lobby/session setup)
- `[DEFERRED] Q12` — Observer-mode draw for non-observer players (normal play sidebar icon draw path)? (category: `out-of-scope`; reason: separate draw path in StripClass__Draw non-observer branch, noted in SIDEBAR_SYSTEM §20 as future work)

---

## Sources

- `disassemble_function 0x006aa140` — full assembly of `StripClass__Draw` observer branch (critical switch at `0x006aa159`-`0x006aa1db`)
- `decompile_function 0x006aa185` — decompiled C of `StripClass__Draw` (confirmed switch cases and SHP globals)
- `read_memory 0x006aa5c4 52` — raw jump table bytes, decoded all 13 entries
- `get_xrefs_to` on each of 15 SHP filename strings → all xref to `SidebarClass__LoadSHPs`
- `get_xrefs_to` on each of 15 SHP globals → all confirm `StripClass__Draw` as sole reader
- `decompile_function 0x005113f0` — `HouseTypeClass__Constructor`, confirmed `+0xB4`/`+0xB8` self-index fields
- `COUNTRY_SIDE_TYPE_CLASSES.md` — CountryTypeClass struct layout (+0xB4/+0xB8/+0xBC offsets)
- `SIDEBAR_SYSTEM_GHIDRA_REPORT.md` §13, §20 — prior SHP global table and observer draw
- `ini/rulesmd.ini` lines 959-976 — `[Countries]` list confirming index-to-country mapping
