# InitLayoutConstants — Ghidra Research Report

**Primary address:** `0x006A5090` (`SidebarClass__InitLayoutConstants`)
**Secondary address:** `0x006A5130` (`SidebarClass__InitSidebarRect`) — sets the remaining 11 globals
**Confidence:** High (both functions fully decompiled; all globals xref-confirmed)
**Active in YR:** Yes — called unconditionally from `SidebarClass__Init @ 0x006A5310`

---

## 1. Overview

`SidebarClass__InitLayoutConstants` (`0x006A5090`) sets **6 of the 17 layout globals**
(`DAT_00b0b4e0`, `e4`, `ec`, `f0`, `f8`, `fc`) based on the local side index.
The remaining **11 globals** (`DAT_00b0b4dc`, `DAT_00b0b4e8`, `DAT_00b0b4f4`,
`DAT_00b0b500`, `DAT_00b0b504`, `DAT_00b0b508`, `DAT_00b0b50c`, `DAT_00b0b510`,
`DAT_00b0b514`, plus the two already set by the first function and then confirmed by
`InitSidebarRect`) are set by `SidebarClass__InitSidebarRect` (`0x006A5130`), which is
called immediately after by `SidebarClass__Init`.

> **Doc discrepancy:** SIDEBAR_SYSTEM_GHIDRA_REPORT.md §4 attributes all 17 globals to
> `FUN_006a5090`. This is WRONG. FUN_006a5090 sets only 6; `FUN_006a5130` (=
> `SidebarClass__InitSidebarRect`) sets the other 11. The doc also listed `FUN_006a5130`
> as a "depends on" predecessor that writes `SidebarX` — that predecessor is in fact the
> same function that sets the rest of the globals.

---

## 2. Branch Condition — Local Side 0 vs Nonzero

```c
int localSideIndex = *(int *)(g_ScenarioClass_Instance + 0x34B8);
if (localSideIndex == 0)  { /* Allied branch */ }
else                      { /* Soviet/Yuri branch */ }
```

Evidence: assembly at `0x006A5090`–`0x006A509D`:
```
006a5090: MOV EAX, [0x00a8b230]      ; g_ScenarioClass_Instance
006a5095: MOV EAX, [EAX + 0x34b8]   ; local side index
006a509b: TEST EAX, EAX
006a509d: JNZ 0x006a50dc             ; != 0 → Soviet/Yuri branch
```

`g_ScenarioClass_Instance = DAT_00a8b230`. SIDEBAR_SYSTEM §2 cited
`*(int*)(DAT_00a8b230 + 0x34B8)` which is correct. Fresh writer tracing establishes
that this field is the selected local house/country `HouseTypeClass+0xBC` side
index: Allied `0`, Soviet `1`, Yuri `2`. The branch therefore selects the
Soviet/Yuri layout on **any non-zero side** (`!= 0`, not `== 1`). Writer evidence:
`Read_Scenario @ 0x00684620`, `0x0068479D..0x006847C9`; `Full_Init`,
`0x00687794..0x00687833`. Consumer evidence: `decompile_function 0x006A5090`.

**`SidebarClass__InitSidebarRect` uses 3-way branching on the same side index:**

```c
int iVar1 = *(int *)(g_ScenarioClass_Instance + 0x34B8);
if (iVar1 == 0)  { /* Allied values */        }
else if (iVar1 == 1) { /* Soviet values */ }
else             { /* Yuri/other values (same as Soviet) */ }
```

For `DAT_00b0b4dc` and `DAT_00b0b4e8`, the `== 1` branch and the final `else` branch
write **identical values** — so in practice it's still binary (0 vs non-zero).
Verified: `decompile_function 0x006A5130`.

---

## 3. Full Global Table — Verified Values

All 17 globals confirmed via decompilation of both functions.
`g_SidebarWidth = 0x9E = 158` (DAT_00886f94); `g_SidebarX` = DAT_00886f90;
`g_SidebarTopClip = DAT_00886f98` (= `g_SIDEBAR_WIDTH_CONST = 168 = 0xA8`).

### Set by `SidebarClass__InitLayoutConstants @ 0x006A5090`

| Global | Name (SIDEBAR_SYSTEM §4) | Allied (side == 0) | Soviet/Yuri (side != 0) | Evidence |
|---|---|---|---|---|
| `DAT_00b0b4e0` | Repair button Y | `g_SidebarWidth + 8` = 166 | `g_SidebarWidth + 7` = 165 | decompile 0x006A5090 |
| `DAT_00b0b4e4` | Sell button X delta from repair X | `0x40` = **64** | `0x34` = **52** | decompile 0x006A5090 |
| `DAT_00b0b4ec` | Tab buttons Y | `g_SidebarWidth + 0x27` = 197 | `g_SidebarWidth + 0x27` = **197 (same)** | decompile 0x006A5090 |
| `DAT_00b0b4f0` | Tab button spacing | `0x1D` = **29** | `0x20` = **32** | decompile 0x006A5090 |
| `DAT_00b0b4f8` | Cameo area Y | `g_SidebarWidth + 0x45` = 227 | `g_SidebarWidth + 0x45` = **227 (same)** | decompile 0x006A5090 |
| `DAT_00b0b4fc` | Cameo column width | `0x3F` = **63** | `0x40` = **64** | decompile 0x006A5090 |

### Set by `SidebarClass__InitSidebarRect @ 0x006A5130`

| Global | Name (SIDEBAR_SYSTEM §4) | Allied (side == 0) | Soviet/Yuri (side != 0) | Evidence |
|---|---|---|---|---|
| `DAT_00b0b4dc` | Repair/sell button X | `g_SidebarX + 0x14` = SidebarX+20 | `g_SidebarX + 0x21` = SidebarX+33 | decompile 0x006A5130 |
| `DAT_00b0b4e8` | Tab buttons X start | `g_SidebarX + 0x1A` = SidebarX+26 | `g_SidebarX + 0x14` = SidebarX+20 | decompile 0x006A5130 |
| `DAT_00b0b4f4` | Cameo area X | `g_SidebarX + 0x16` = SidebarX+22 | `g_SidebarX + 0x16` = **SidebarX+22 (same)** | decompile 0x006A5130 |
| `DAT_00b0b500` | Cameo row height | `0x32` = **50** | `0x32` = **50 (same)** | decompile 0x006A5130 |
| `DAT_00b0b504` | Cameo total height | `((DAT_00886f9c - DAT_00b0b4f8 - 0x1A - 7 + g_SidebarWidth) / 0x32) * 0x32` | same formula, `0x1A → 0x12` | decompile 0x006A5130 |
| `DAT_00b0b508` | Scroll button X | `g_SidebarX + 0x27` | `g_SidebarX + 0x27` **(same)** | decompile 0x006A5130 |
| `DAT_00b0b50c` | Scroll button Y | `DAT_00b0b4f8 + 7 + DAT_00b0b504` | same formula | decompile 0x006A5130 |
| `DAT_00b0b510` | Scroll button width | `0x2E` = **46** | `0x2D` = **45** | decompile 0x006A5130 |
| `DAT_00b0b514` | Scroll speed | `0x32` = **50** | `0x32` = **50 (same)** | decompile 0x006A5130 |

**Not set by either function** (neither decompiled body writes them):
- `DAT_00b0b4e0` through `DAT_00b0b4fc` fully accounted above (6 from InitLayoutConstants).
- No `DAT_00b0b4e8` (Tab X) write appears in `InitLayoutConstants` — only in `InitSidebarRect`.

---

## 4. DAT_00b0b504 Formula — Critical Detail

The cameo total height uses an asymmetric overhead constant:

```c
int iVar3 = (iVar1 == 0) ? 0x1A : 0x12;   // Allied: 26, Soviet/Yuri: 18
DAT_00b0b504 = ((DAT_00886f9c - DAT_00b0b4f8 - iVar3 - 7 + g_SidebarWidth) / 0x32) * 0x32;
```

Where:
- `DAT_00886f9c` = `g_SidebarBottomY` (derived from screen height)
- `DAT_00b0b4f8` = `CameoAreaY` (just set: `g_SidebarWidth + 0x45 = 227`)
- `iVar3` = 26 (Allied) or 18 (Soviet/Yuri) — tab overhead constant
- `g_SidebarWidth` = 0x9E = 158
- Division is integer floor; result is rounded DOWN to nearest multiple of 50

At 800×600: `g_SidebarBottomY` = 600 (approximately). Then:
- Allied: `floor((600 - 227 - 26 - 7 + 158) / 50) * 50 = floor(498/50)*50 = 9*50 = 450`
- Soviet/Yuri: `floor((600 - 227 - 18 - 7 + 158) / 50) * 50 = floor(506/50)*50 = 10*50 = 500`

Evidence: `decompile_function 0x006A5130`.

---

## 5. Call Sites (Callers)

`SidebarClass__InitLayoutConstants` has exactly **one caller**:
`SidebarClass__Init @ 0x006A5310`

Call sequence within `SidebarClass__Init`:
```c
FUN_00653010();                        // some pre-init
SidebarClass__InitLayoutConstants();   // sets 6 globals
SidebarClass__InitSidebarRect(0);      // sets remaining 11 globals; arg 0 = pre-game init
```

`SidebarClass__InitSidebarRect` is called twice from `SidebarClass__Init`:
once with `param_1 = 0` (pre-activation) and once with `param_1 = 1` (post-activation).
The `param_1` flag controls whether `g_SidebarX` is computed from viewport globals
(param_1 = 0) or from a surface rect query via `FUN_0072AD90` (param_1 = 1).
Both calls write all 11 globals. Evidence: `decompile_function 0x006A5310`.

---

## 6. Doc Error Summary

SIDEBAR_SYSTEM_GHIDRA_REPORT.md §4 claims all 17 globals are set by `FUN_006a5090`.
**Verified wrong:** only 6 are. The table of values in §4 is otherwise numerically
correct (the values match the binary) — the attribution to a single function is the error.

The context note "depends on FUN_006a5130 for SidebarX" in the original task scope
was also slightly wrong: `FUN_006a5130` IS `SidebarClass__InitSidebarRect`, which sets
the remaining 11 globals (not a mere "predecessor" that sets SidebarX).

---

## 7. Open Questions — Final State

- `[RESOLVED] OQ1` — Which function sets which globals? → `InitLayoutConstants` sets 6 (Y, Height, TabY, TabSpacing, CameoY, ColW); `InitSidebarRect` sets 11 (X, TabX, CameoX, RowH, TotalH, ScrollX, ScrollY, ScrollW, ScrollSpeed). Evidence: `decompile_function 0x006A5090`, `0x006A5130`.
- `[RESOLVED] OQ2` — Branch condition: `== 1`, `== 2`, or `!= 0`? → `TEST EAX, EAX` / `JNZ` → fires on **any non-zero side index**. Writer evidence: `0x0068479D..0x006847C9` and `0x00687794..0x00687833`; consumer evidence: assembly context `0x006A5090–0x006A509D`.
- `[RESOLVED] OQ3` — Is `DAT_00a8b230` = `g_ScenarioClass_Instance`? → Yes, confirmed by xrefs to `Main_Game`, `LogicClass__PerTickUpdate`, and many simulation functions. Evidence: `get_xrefs_to 0x00a8b230`.
- `[RESOLVED] OQ4` — Active in YR? → Yes. Called from `SidebarClass__Init` which is vtable slot 8 at `0x006A5310`, active in standard YR skirmish. No TS gate. Evidence: `decompile_function 0x006A5310`.
- `[RESOLVED] OQ5` — Does the 3-way flag check (`== 0`, `== 1`, else) in `InitSidebarRect` produce distinct behavior? → No. Both `== 1` and `else` write the same values to `DAT_00b0b4dc` and `DAT_00b0b4e8`. Functionally binary. Evidence: `decompile_function 0x006A5130`.
- `[RESOLVED] OQ6` — Is `DAT_00b0b500` (RowH=50) always 50 regardless of mode? → Yes. Written as `0x32` unconditionally before the branch, in `InitSidebarRect`. Evidence: `decompile_function 0x006A5130`.
- `[RESOLVED] OQ7` — What drives `DAT_00b0b504` (TotalH)? → Computed formula (see §4). Depends on `g_SidebarBottomY` (screen-height-derived) minus `CameoAreaY` minus a mode-specific overhead (26 RA2, 18 YR), divided by 50, then multiplied back — nearest lower multiple of 50. Evidence: `decompile_function 0x006A5130`.
- `[DEFERRED] OQ8` — Exact runtime value of `g_SidebarBottomY` (`DAT_00886f9c`) at various screen resolutions. (category: `needs-runtime-debugger`; reason: derived from `g_RadarViewportHeight` + `g_RadarViewportOffsetY` which require live viewport state; formula is verified, absolute pixel value is not. next-step: read DAT_00886f9c in debugger at 800×600 startup.)

---

## Sources

Ghidra functions decompiled:
- `SidebarClass__InitLayoutConstants @ 0x006A5090`
- `SidebarClass__InitSidebarRect @ 0x006A5130`
- `SidebarClass__Init @ 0x006A5310` (context / call sequence)

Assembly context read:
- Branch at `0x006A5090`–`0x006A509D` (TEST/JNZ for local side index)

XRefs checked:
- `DAT_00b0b4dc`, `DAT_00b0b4e8`, `DAT_00b0b500`, `DAT_00b0b504`, `DAT_00b0b508`,
  `DAT_00b0b50c`, `DAT_00b0b510`, `DAT_00b0b514`, `DAT_00b0b4f4`, `DAT_00886f94`
- `DAT_00a8b230` (g_ScenarioClass_Instance) — caller set xref-confirmed

Callers confirmed:
- `get_function_callers 0x006A5090` → sole caller: `SidebarClass__Init @ 0x006A5310`

Prior docs consulted:
- `SIDEBAR_SYSTEM_GHIDRA_REPORT.md` §2, §4 (doc error identified in §4 attribution)
