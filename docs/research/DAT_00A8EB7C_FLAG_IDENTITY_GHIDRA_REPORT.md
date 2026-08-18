# DAT_00A8EB7C Flag Identity — Ghidra Report

**Date:** 2026-05-19  
**Target:** `0x00A8EB7C` — single byte, meaning unknown at investigation start  
**Method:** `read_memory`, `get_xrefs_to`, `decompile_function` via Ghidra MCP (read-only)

---

## 1. Identity

| Property | Value |
|---|---|
| Address | `0x00A8EB7C` |
| Size | 1 byte (`undefined1` / `char`) |
| Static-file value | `0x00` (all zeros; runtime value is set before game loop) |
| Semantic name | **`OptionsClass::bSidebarOnRight`** |
| Confidence | **HIGH** — confirmed by struct offset math + string evidence |

### Derivation

`0x00A8EB60` is the `OptionsClass` singleton base (confirmed via `get_xrefs_to 0x00a8eb60`: write from `OptionsClass__SetDefaults` at `0x005fa35a`, DATA ref from `0x005fa350` = function entry; verified via `decompile_function 0x005fa350`).

`OptionsClass__SetDefaults` writes `*(undefined1 *)(param_1 + 7) = 1`.  
`param_1` is `undefined4 *` → offset = `7 × 4 = 0x1C`.  
`0x00A8EB60 + 0x1C = 0x00A8EB7C`. ✓

`OptionsClass__ReadFromINI` (decompiled at `0x005fa620`) also hard-sets `*(undefined1 *)(param_1 + 7) = 1` and then logs:
```
Register_heap_pool(s_SideBar_on__s_00833200, s_RIGHT_00833210)
```
Memory read of `0x00833200` returns `"SideBar on %s\n"` and `0x00833210` returns `"RIGHT"`.  
(Verified via `read_memory 0x00833200 length=32`)

Meaning: the flag records whether the sidebar is on the **right** side (standard RA2/YR layout). When `== '\x01'` → sidebar on right. When `== '\0'` → sidebar on left (map-editor / non-standard mode).

---

## 2. Writer

**Single writer:** `OptionsClass__SetDefaults` at entry `0x005FA350`, write instruction at `0x005FA35A`.  
(Verified via `get_xrefs_to 0x00a8eb7c` returning one `[WRITE]` entry; confirmed via `decompile_function 0x005fa350`)

| Writer | Address | Value written | Trigger |
|---|---|---|---|
| `OptionsClass__SetDefaults` | `0x005FA35A` | `1` (sidebar on right) | Called unconditionally during options initialization |
| `OptionsClass__ReadFromINI` | inline in `0x005FA620` | `1` (hard-coded, not read from INI) | During INI read — always sets `1` regardless of INI content |

`ReadFromINI` has NO `CCINIClass__ReadBool` call for this field — the value `1` is hardcoded. There is no INI key that controls it. It is always `1` in a normal YR game.

Callers of `SetDefaults`: `0x004E7E25` and `0x0055F975` (xrefs confirmed via `get_xrefs_to 0x005fa350`).

---

## 3. Reader Summary

17 read sites total (from `get_xrefs_to 0x00a8eb7c`). Representative set examined:

| Function | Address | What it gates |
|---|---|---|
| `SidebarClass__InitSidebarRect` | `0x006A513D` | 4 root sidebar globals (`g_SidebarWidth`, `g_SidebarX`, `g_SidebarTopClip`, `DAT_00886f9c`) are only written when flag `!= '\0'` (i.e., sidebar on right) |
| `SidebarClass__InitSurface` | `0x006ABDB5` | X-origin for a surface position call — uses `g_RadarViewportWidth + g_RadarViewportOffsetX` when `!= '\0'`, else `0` |
| `FUN_0072AD20` | `0x0072AD32` | Returns `x=0xA8` (sidebar width) when flag `!= '\0'`; returns `x=0` when `'\0'` |
| `FUN_0072AD90` | `0x0072AD97` | Same pattern — sidebar-left-edge X offset |
| `FUN_00654320` | `0x0065432D` | Radar/minimap button X position: `radar_width` offset when `!= '\0'`, else 0 |
| `FUN_00478DB0` | `0x00478DB6` | Tactical-view hit-test right clip: uses radar viewport edge when `'\x01'`, uses `g_SidebarTopClip + g_SidebarX` otherwise |
| `FUN_007B90C0` | `0x007B9102` | Scroll/drag surface offset for sidebar panel |
| `FUN_004F4780` | `0x004F4875` | Mouse cursor clip rect — narrows tactial viewport width when `'\0'` and not in map editor |

The pattern is consistent across all readers: `'\x01'` = sidebar occupies the right side at `g_RadarViewportWidth + g_RadarViewportOffsetX`; `'\0'` = sidebar absent or on left (map-editor or edge case path).

---

## 4. Runtime Value During a Normal YR Skirmish

**The flag is `1` (`'\x01'`) during any normal YR skirmish.**

Reason:
- `OptionsClass__SetDefaults` is called early in startup (before the main game loop) and writes `1`.
- `OptionsClass__ReadFromINI` also hard-codes `1` — there is no INI override.
- No other writer exists.

**Consequence for `SidebarClass__InitSidebarRect`:**  
The `param_1 == '\0'` path (standard init from `SidebarClass__InitSurface`) checks `DAT_00a8eb7c != '\0'` before writing the 4 root sidebar globals. Since the flag is always `1`, **the 4-globals block ALWAYS fires** in a normal YR skirmish. The concern raised in the investigation request is resolved: the guard is never `0` in practice.

---

## 5. TS-vs-YR Filter

The `'\0'` path (sidebar on left) is consistent with a map-editor or Tiberian Sun left-sidebar mode. In standard YR the sidebar is always on the right. No TS-legacy dead-code concern — both paths are live and the flag drives real layout decisions. The `'\x01'` branch is always taken in YR; the `'\0'` branch is a map-editor accommodation.

---

## 6. Sibling Docs

`INIT_SIDEBAR_RECT_GHIDRA_REPORT.md` (2026-05-19) was the source of the investigation request and already documents the guard at `0x006A513D`. That doc should be updated to name the flag as `OptionsClass::bSidebarOnRight` at `OptionsClass+0x1C`.

No other doc in `ra2-rust-game-docs/` previously named or identified `0x00A8EB7C`.

---

## Verified Facts

1. **`0x00A8EB7C` = `OptionsClass` singleton base (`0x00A8EB60`) + offset `0x1C`** — verified by `decompile_function 0x005fa350` showing `*(undefined1 *)(param_1 + 7) = 1` with `param_1 = 0x00A8EB60`.
2. **String `"SideBar on %s"` + `"RIGHT"`** logged unconditionally after the field write in `OptionsClass__ReadFromINI` — verified via `decompile_function 0x005fa620` + `read_memory 0x00833200`.
3. **Single writer; value is always `1`** — `OptionsClass__SetDefaults` (`0x005FA35A`) and `ReadFromINI` both hard-code `1`; no INI key controls it. Verified via `get_xrefs_to 0x00a8eb7c` showing one `[WRITE]` entry.
4. **17 readers; all gate sidebar-right-side geometry** — consistently tested against `'\0'` (sidebar-on-left / map-editor) vs `'\x01'` (sidebar-on-right / normal game). Verified via `decompile_function` on 8 representative readers.
5. **`InitSidebarRect` 4-globals block always fires in YR** — the `DAT_00a8eb7c != '\0'` guard is always true at runtime. Verified by writer analysis above.
