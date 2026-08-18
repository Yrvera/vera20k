# InitSidebarRect — Ghidra Research Report

**Target:** `SidebarClass__InitSidebarRect` @ `0x006A5130`  
**Scope:** Four sidebar-position globals + input sources + call sites.  
**Verified via:** `decompile_function 0x006a5130`, `decompile_function 0x0072ad90`,  
`decompile_function 0x006abd30`, `decompile_function 0x006a5310`,  
`get_function_callers 0x006a5130`, `read_memory 0x007f5bf8/0x007f5bfc/0x00a8eb7c`.

---

## 1. Function Signature

```c
void SidebarClass__InitSidebarRect(char param_1)
```

`param_1 == '\0'` → normal-game path  
`param_1 != '\0'` → map-editor path (calls `FUN_0072ad90` to get viewport rect)

**Active in YR: Yes.** Called during sidebar init. No TS-only gate.

---

## 2. The Four Globals — Exact Formulas

### `0x00886f94` — `g_SidebarWidth` (documented name: SidebarWidth)

| Branch | Assignment |
|--------|-----------|
| Normal (param_1 == 0) | `g_SidebarWidth = 0x9E;` (158, hardcoded) |
| Map-editor (param_1 != 0) | `g_SidebarWidth = 0x9E;` (158, hardcoded) |

**Verdict:** Hardcoded to **158 (0x9E)** in BOTH branches. No input globals needed.  
Verified: `decompile_function 0x006a5130` — both assignment sites show the literal `0x9e`.

---

### `0x00886f90` — `g_SidebarX` (documented name: SidebarX)

**Normal-game branch** (`param_1 == '\0'`, `DAT_00a8eb7c != '\0'`):
```c
g_SidebarX = g_RadarViewportWidth + g_RadarViewportOffsetX;
```
Input globals: `g_RadarViewportWidth` (viewport pixel width) + `g_RadarViewportOffsetX` (viewport left-edge X).  
At 800×600 with no left-edge offset: `g_SidebarX = 800 - 158 + 0 = 642` (right edge of tactical area).

**Map-editor branch** (`param_1 != '\0'`):
```c
int *piVar2 = FUN_0072ad90();   // returns ptr to 4-int rect
g_SidebarX = piVar2[2] + *piVar2;   // piVar2[2] = width, *piVar2 = offsetX
```
`FUN_0072ad90` (verified via `decompile_function 0x0072ad90`) fills a local 4-int struct:
- `param_1[0]` = offsetX: 0x0 if `DAT_00a8eb7c == '\0'` (non-editor), `0xA8` (168) if editor/non-classic
- `param_1[1]` = offsetY: 0
- `param_1[2]` = width: `DAT_00a8eb84 - 0xA8` (screen_width minus 168)
- `param_1[3]` = height: `DAT_00a8eb88 - 0x20` (screen_height minus 32)

So in the editor path: `g_SidebarX = (screen_width - 168) + 168 = screen_width`.  
**Active in YR: Yes** (both branches active, branch selected by `param_1`).

---

### `0x00886f98` — `g_SidebarTopClip` (documented name: SidebarTopClip)

Both branches:
```c
g_SidebarTopClip = g_SIDEBAR_WIDTH_CONST;
```
`g_SIDEBAR_WIDTH_CONST` is at `0x007f5bf8`. Value confirmed by `read_memory 0x007f5bf8` = **0xA8 = 168**.

**Verdict:** Always set to the constant at `0x007f5bf8` = **168**. No other inputs.  
Active in YR: Yes.

---

### `0x00886f9c` — `DAT_00886f9c` (documented name: SidebarBottomY)

**Normal-game branch:**
```c
DAT_00886f9c = DAT_007f5bfc + g_RadarViewportHeight + (-0x9e) + g_RadarViewportOffsetY;
```
`DAT_007f5bfc` confirmed by `read_memory 0x007f5bfc` = **0x20 = 32**.

**Map-editor branch:**
```c
DAT_00886f9c = DAT_007f5bfc + piVar2[3] + (-0x9e) + piVar2[1];
// piVar2[3] = screen_height - 32, piVar2[1] = 0
// = 32 + (screen_height - 32) - 158 + 0 = screen_height - 158
```

Formula normalised (both branches are equivalent in non-editor mode):
```
SidebarBottomY = 32 + radar_viewport_height - 158 + radar_viewport_offsetY
               = DAT_007f5bfc + g_RadarViewportHeight - 0x9E + g_RadarViewportOffsetY
```
At 600px height (offsetY=0): `32 + 600 - 158 + 0 = 474`.  
Active in YR: Yes.

---

## 3. Input Source Globals

| Global Address | Ghidra Name | Role |
|---|---|---|
| `0x007f5bf8` | `g_SIDEBAR_WIDTH_CONST` | Constant = 168 (0xA8); used as SidebarTopClip |
| `0x007f5bfc` | unnamed | Constant = 32 (0x20); added into SidebarBottomY |
| `g_RadarViewportWidth` | (named in Ghidra) | Radar/tactical viewport pixel width |
| `g_RadarViewportOffsetX` | (named in Ghidra) | Radar/tactical viewport left-edge X |
| `g_RadarViewportHeight` | (named in Ghidra) | Radar/tactical viewport pixel height |
| `g_RadarViewportOffsetY` | (named in Ghidra) | Radar/tactical viewport top-edge Y |
| `DAT_00a8eb7c` | unnamed | Gate flag: `!= '\0'` enables the 4-globals assignment in normal branch |
| `DAT_00a8eb84` | unnamed | Screen width (used in map-editor path via FUN_0072ad90) |
| `DAT_00a8eb88` | unnamed | Screen height (used in map-editor path via FUN_0072ad90) |

`DAT_00a8eb7c` at `read_memory` = 0 (default off). The normal-game 4-globals block is
gated on this flag being nonzero. In the game, this flag is set before the sidebar is
initialized — the condition is the "new sidebar active" flag for the normal (non-map-editor)
code path. See also: `SidebarClass__InitSurface` which checks the same flag to set `g_SidebarX`
to 0 vs `g_RadarViewportWidth + g_RadarViewportOffsetX`.

---

## 4. Call Sites of `SidebarClass__InitSidebarRect` (0x006A5130)

Verified via `get_function_callers 0x006a5130`:

### Caller 1: `SidebarClass__Init` @ `0x006A5310`

Called **twice** in the same function body (verified via `decompile_function 0x006a5310`):
1. Early call: `SidebarClass__InitSidebarRect(0)` — before layout widget initialization
2. Late call: `SidebarClass__InitSidebarRect(1)` — after the first layout pass (calls vtable
   slot 0x88) — this second call uses the map-editor branch (`param_1 != '\0'`)

### Caller 2: `SidebarClass__InitSurface` @ `0x006ABD30`

Called once: `SidebarClass__InitSidebarRect(0)` — at the top of `InitSurface` before any
surface creation, to refresh the 4 position globals before positioning all child widgets.
Verified via `decompile_function 0x006abd30`.

---

## 5. Conditional Branches Inside the Function

```
if (param_1 == '\0') {
    if (DAT_00a8eb7c != '\0') {
        // normal 4-globals assignment
    }
    // else: nothing written (globals unchanged)
}
else {
    // map-editor 4-globals assignment via FUN_0072ad90()
}
```

After both branches, unconditional code runs to compute **additional cameo/button layout
globals** (`DAT_00b0b500`, `DAT_00b0b504`, `DAT_00b0b50c`, `DAT_00b0b510`, etc.) using
`g_ScenarioClass_Instance + 0x34B8` (side index, 0=Allied/2=Yuri).

**No NewSidebar/resolution branch inside `InitSidebarRect` itself.** The layout variation
is entirely in the post-assignment block based on side index.

---

## 6. 158 vs 168 Disambiguation

| Constant | Value | Address | Role |
|----------|-------|---------|------|
| SidebarWidth (`g_SidebarWidth`) | **158 (0x9E)** | `0x00886f94` | Pixel width of the sidebar chrome region |
| SIDEBAR_WIDTH_CONST / SidebarTopClip | **168 (0xA8)** | `0x007f5bf8` | Height of the top (radar) chrome area; used as top-of-cameo-strip clip; also called "SIDEBAR_WIDTH" in SIDEBAR_RADAR_POSITIONING.md |

The naming collision in `SIDEBAR_RADAR_POSITIONING.md` ("SIDEBAR_WIDTH = 0xA8 at 0x007f5bf8")
is misleading. In `InitSidebarRect`, that constant (`0x007f5bf8` = 168) is used exclusively
as the **Y-coordinate clip** for the top of the sidebar (radar area height), not a width.
The actual chrome pixel width is 158 (hardcoded literal `0x9E`). These are two distinct concepts.

---

## 7. Open Questions (Out of Scope)

- `FUN_006a5090` (`SidebarClass__InitLayoutConstants`) — called by `SidebarClass__Init` just
  before the first `InitSidebarRect(0)` call; sets `DAT_00b0b4e0` (repair button Y), etc.
  Not investigated here.
- `FUN_006a5310` full init body contains complex multiplayer house-enumeration logic — out of scope.
- Exact addresses of `g_RadarViewportWidth/OffsetX/Height/OffsetY` not resolved to raw hex
  addresses from this session; the Ghidra labels are trusted but unchecked in isolation.
- `DAT_00a8eb7c` exact semantic (which game flag sets it nonzero) not traced.

---

## 8. Key Verified Facts

1. **SidebarWidth = 158 (0x9E), hardcoded.** Both branches set `g_SidebarWidth = 0x9E`. No input
   globals. (Verified: `decompile_function 0x006a5130`, both assignment sites.)

2. **SidebarX = radar_viewport_right_edge.** Normal path: `g_SidebarX = g_RadarViewportWidth +
   g_RadarViewportOffsetX`. (Verified: `decompile_function 0x006a5130`.)

3. **SidebarTopClip = 168, from constant at 0x007f5bf8.** Always `g_SIDEBAR_WIDTH_CONST` (=
   `*(int*)0x007f5bf8`). (Verified: `read_memory 0x007f5bf8` → `a8 00 00 00` = 168.)

4. **SidebarBottomY = 0x007f5bfc (32) + radar_height - 158 + radar_offsetY.** At 600px:
   `32 + 600 - 158 = 474`. `DAT_007f5bfc = 32` confirmed by `read_memory 0x007f5bfc` → `20 00 00 00`.

5. **Two callers; three total call sites.** `SidebarClass__Init` (0x6a5310) calls it twice
   (param=0, then param=1); `SidebarClass__InitSurface` (0x6abd30) calls it once (param=0).
   (Verified: `get_function_callers 0x006a5130`, both decompiled.)
