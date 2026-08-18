# Sidebar Cameo/Chrome ConvertClass Setup 0x0052BA60 - Ghidra Research Report

**Address(es):** `0x0052BA60` primary game init; direct sidebar follow-up `0x006A5840`; palette loader `0x0072F350`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Which palette/ConvertClass feeds `DAT_0087f6cc` / `DAT_0087f6d0`, and what `0x0052BA60` does or does not prove for in-game sidebar cameo/chrome palette setup.
**Non-Scope:** full sidebar paint path, full observer-sidebar data semantics, side MIX mounting, DIALOG loading-screen palettes, and cameo flash behavior.
**Confidence:** High for setup/source palettes; Medium for Rust delta labels where Rust render surfaces were scanned but not exhaustively runtime-tested.
**Active in YR:** Yes. `0x0052BA60` is called from `Main_Game` at `0x0048CCCF`; `SidebarClass::LoadSHPs` is a live sidebar load function with direct xrefs and SHP/global writes.

## 1. Overview

`0x0052BA60` is broad game initialization. It constructs multiple global `ConvertClass` objects from palette files such as `ANIM.PAL`, `PALETTE.PAL`, `UNITSNO.PAL`, `CAMEO.PAL`, `MOUSEPAL.PAL`, and `GRFXTXT.PAL`, but it does **not** write `DAT_0087f6cc` or `DAT_0087f6d0`.

`DAT_0087f6cc` and `DAT_0087f6d0` are rebuilt in `SidebarClass::LoadSHPs` (`0x006A5840`). `DAT_0087f6cc` is built from a copy of raw palette buffer `DAT_00b0fbe4`, which `PaletteLoad` fills from `SIDEBAR.PAL`. `DAT_0087f6d0` is built from raw palette buffer `DAT_00b0fbfc`, which `PaletteLoad` fills from `OBSERVER.PAL`.

## 2. Key Globals / Palette Sources

| Global | Writer | Palette source | Active in YR | Evidence |
|---|---:|---|---|---|
| `DAT_0087f6cc` | `0x006A58AD` | raw `DAT_00b0fbe4` = `SIDEBAR.PAL` | Yes | `SidebarClass::LoadSHPs` decompile; assembly `0x006A584E`, `0x006A58A8`; `PaletteLoad` assembly `0x0072F3CA..0x0072F3DA`; string `0x0084542C` |
| `DAT_0087f6d0` | `0x006A5AA5` | raw `DAT_00b0fbfc` = `OBSERVER.PAL` | Yes, conditional consumer role | `SidebarClass::LoadSHPs` decompile; assembly `0x006A5A48`, `0x006A5AA0`; `PaletteLoad` assembly `0x0072F3DF..0x0072F3EF`; string `0x008453F4` |
| `DAT_0087f6b4` | `0x0052C075` | `CAMEO.PAL` loaded into stack buffer | Yes | `0x0052C089` loads string ptr `0x008204E0`; constructor at `0x0052C070` |
| `DAT_0087f6b0` | `0x0052C129` | `MOUSEPAL.PAL` loaded into stack buffer | Yes | `0x0052C13D` loads string ptr `0x00826084`; constructor at `0x0052C124` |
| `DAT_0087f6bc` | `0x0052C1FB` | alias/copy of `DAT_0087f6b4` | Yes | assembly `0x0052C1EA MOV EAX,[0x0087f6b4]`, `0x0052C1FB MOV [0x0087f6bc],EAX` |

## 3. Core Logic

### 3.1 `0x0052BA60` palette block

The relevant palette construction pattern in `0x0052BA60` is:

1. Load a named `.PAL` file into a 768-byte local stack buffer (`0x300` bytes).
2. Expand/copy 256 RGB triplets through the same signed-mask loop pattern: `idx = counter & 0x800000ff`; if negative, normalize back into byte range.
3. Allocate `0x188` bytes for `ConvertClass`.
4. Call `ConvertClass__Constructor(source_palette, destination_palette, DAT_00887308, arg4, arg5)`.
5. Write one of the `DAT_0087f6b0..c8` globals, or zero it on allocation failure.

Tiny details:

| Detail | Evidence | Active in YR |
|---|---|---|
| `ConvertClass` allocation size is `0x188` bytes. | `operator_new(0x188)` before each constructor; examples `0x0052BEDE`, `0x0052C046`, `0x0052C0FA`, `0x006A587B`, `0x006A5A73` | Yes |
| Most `0x0052BA60` stack-palette ConvertClass constructors use `arg4=0x35`, `arg5=0`. | Constructor push sequence before `0x0052BF08`, `0x0052BFBC`, `0x0052C070`, `0x0052C124` | Yes |
| The final `GRFXTXT.PAL` ConvertClass uses `arg4=1`, `arg5=0`, not `0x35`. | `0x0052C1C5 PUSH EBX`, `0x0052C1C6 PUSH 0x1`, constructor at `0x0052C1D8` | Yes |
| `DAT_0087f6bc` is not separately constructed; it aliases `DAT_0087f6b4`. | `0x0052C1EA` read from `DAT_0087f6b4`; `0x0052C1FB` write to `DAT_0087f6bc` | Yes |
| `0x0052BA60` has no xrefs/writes to `0x0087f6cc` or `0x0087f6d0`. | `get_bulk_xrefs` for both globals: writers are `0x006A58AD/0x006A5AA5` and cleanup, not `0x0052BA60` | Yes |

### 3.2 `DAT_0087f6cc` construction

`SidebarClass::LoadSHPs` starts by calling `FUN_0072f4a0`, whose decompile is exactly `return DAT_00b0fbe4;`. It copies `0xC0` dwords (`768` bytes) from that raw palette buffer to a local stack buffer, destroys any old `DAT_0087f6cc`, allocates `0x188`, and constructs:

`ConvertClass(local_600, local_600, DAT_00887308, 1, 0) -> DAT_0087f6cc`.

`PaletteLoad` fills `DAT_00b0fbe4` by loading the filename pointer at `0x00844BF0`; that pointer resolves to string `SIDEBAR.PAL` at `0x0084542C`.

Active in YR: Yes. `SidebarClass::LoadSHPs` assigns `DAT_0087f6cc` into repair/sell/tab/gclock-related gadget fields and `StripClass::Draw` directly reads it for normal build-cameo chrome overlays.

### 3.3 `DAT_0087f6d0` construction

Later in `SidebarClass::LoadSHPs`, it calls `FUN_0072f4e0`, whose assembly is `MOV EAX,[0x00b0fbfc]; RET`. It copies another `0xC0` dwords into a local stack buffer, destroys any old `DAT_0087f6d0`, allocates `0x188`, and constructs:

`ConvertClass(auStack_300, auStack_300, DAT_00887308, 1, 0) -> DAT_0087f6d0`.

`PaletteLoad` fills `DAT_00b0fbfc` by loading the filename pointer at `0x00844C04`; that pointer resolves to string `OBSERVER.PAL` at `0x008453F4`.

Active in YR: Yes, but the proved direct sidebar consumer is the observer/spectator branch inside `StripClass::Draw`, not ordinary player build cameos. `StripClass::Draw` reads `DAT_0087f6d0` at `0x006AA144` and `0x006AA2BA` immediately before `CC_Draw_Shape` calls for observer-side icon/SHP draws.

## 4. INI Keys

No INI keys are read in this slice. Source palettes are hardwired filename strings/tables in binary code and loaded from MIX content:

| Asset | Purpose in this slice | Evidence |
|---|---|---|
| `SIDEBAR.PAL` | Source for `DAT_00b0fbe4`, then `DAT_0087f6cc` | `0x0084542C`, `0x0072F3CA..0x0072F3DA`, `0x006A584E..0x006A58AD` |
| `OBSERVER.PAL` | Source for `DAT_00b0fbfc`, then `DAT_0087f6d0` | `0x008453F4`, `0x0072F3DF..0x0072F3EF`, `0x006A5A48..0x006A5AA5` |
| `CAMEO.PAL` | Source for `DAT_0087f6b4` in `0x0052BA60`; not direct proof for `DAT_0087f6cc/d0` | `0x008204E0`, `0x0052C089..0x0052C075` |

## 5. Integration Points

| Function / address | Role | Active in YR | Evidence |
|---|---|---|---|
| `Main_Game` `0x0048CCCF` | Calls `0x0052BA60` during game init | Yes | `get_function_xrefs 0x0052BA60` |
| `0x0052BA60` | Constructs general palette ConvertClass globals, including CAMEO/MOUSE/GRFXTXT; not `cc/d0` | Yes | decompile and xrefs |
| `PaletteLoad` `0x0072F350` | Loads raw/ConvertClass palette slots including `SIDEBAR.PAL` and `OBSERVER.PAL` | Yes | decompile; assembly and strings |
| `FUN_0072f4a0` | Accessor returning `DAT_00b0fbe4` raw `SIDEBAR.PAL` buffer | Yes | decompile |
| `FUN_0072f4e0` | Accessor returning `DAT_00b0fbfc` raw `OBSERVER.PAL` buffer | Yes | assembly |
| `SidebarClass::LoadSHPs` `0x006A5840` | Rebuilds `DAT_0087f6cc/d0` from raw palette copies | Yes | decompile |
| `StripClass::Draw` | Reads `DAT_0087f6cc` for build-cameo chrome/overlays; reads `DAT_0087f6d0` for observer branch | Yes/Conditional | assembly xrefs around `0x006A9B2B`, `0x006AA144` |
| `FUN_006BE1C0` | Shutdown frees `DAT_0087f6b0..d0`, including `cc/d0` | Yes | decompile |

## 6. Current Rust Implementation Status

| Surface | Current status | Delta from binary |
|---|---|---|
| `src/render/sidebar_chrome.rs` | Soviet and Allied chrome both load `sidebar.pal`; Yuri loads `radaryuri.pal`. Repair/sell/tab/gclock use the theme palette. | Matches `DAT_0087f6cc` source for Allied/Soviet ordinary sidebar chrome, based on this slice. |
| `src/render/sidebar_cameo_atlas.rs` + `src/app_init_helpers.rs` | Cameo atlas loads `cameo.pal` first and applies it to build cameo SHPs. | This slice does not prove ordinary build cameo art should use `DAT_0087f6d0`; it only proves `DAT_0087f6d0` = `OBSERVER.PAL` for observer branch consumers. Keep current build-cameo palette separate until another trace proves otherwise. |
| `src/render/sidebar_text.rs` | Side highlight table is independent of `cc/d0`. | Out of scope for this report; no text-color conclusion from these globals. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x0052BA60` writes to palette globals | verified | decompile; assembly `0x0052BE20..0x0052C1FB`; xrefs | none for target |
| `DAT_0087f6cc` source | verified | `0x006A584E`, `0x006A58AD`, `0x0072F3CA..DA`, string `0x0084542C` | none |
| `DAT_0087f6d0` source | verified | `0x006A5A48`, `0x006A5AA5`, `0x0072F3DF..EF`, string `0x008453F4` | observer branch semantics deferred |
| Repair/sell/tab gadget `DAT_0087f6cc` assignment | verified | `SidebarClass::LoadSHPs` decompile assigns gadget fields to `DAT_0087f6cc` | full draw geometry out of scope |
| Normal build-cameo chrome reads of `DAT_0087f6cc` | verified | assembly xrefs `0x006A9B2B`, `0x006A9B9D`, `0x006A9E7B` before `CC_Draw_Shape` | full composition ordering out of scope |
| Observer branch reads of `DAT_0087f6d0` | touched-not-exhausted | assembly xrefs `0x006AA144`, `0x006AA2BA` before `CC_Draw_Shape` | full observer sidebar matrix |
| `0x0052BA60` relation to `DAT_0087f6cc/d0` | verified negative | no xrefs/writes from `0x0052BA60`; `get_bulk_xrefs` | none |

## 8. Open Questions - Final State

- `[RESOLVED] OQ1 - Does Ghidra MCP read-only access exist? -> Yes, read-only decompile/disassembly/xref tools succeeded; no mutating tools were used.` (evidence: `decompile_function 0x0052BA60`)
- `[RESOLVED] OQ2 - Does 0x0052BA60 write DAT_0087f6cc/d0? -> No.` (evidence: `get_bulk_xrefs 0x0087f6cc,0x0087f6d0`)
- `[RESOLVED] OQ3 - Which function writes DAT_0087f6cc? -> SidebarClass::LoadSHPs writes it at `0x006A58AD` after constructing from `FUN_0072f4a0` raw palette.` (evidence: `0x006A5840` decompile)
- `[RESOLVED] OQ4 - Which function writes DAT_0087f6d0? -> SidebarClass::LoadSHPs writes it at `0x006A5AA5` after constructing from `FUN_0072f4e0` raw palette.` (evidence: `0x006A5840` decompile)
- `[RESOLVED] OQ5 - Which file feeds FUN_0072f4a0/DAT_00b0fbe4? -> `SIDEBAR.PAL`.` (evidence: `0x0072F3CA..DA`, string `0x0084542C`)
- `[RESOLVED] OQ6 - Which file feeds FUN_0072f4e0/DAT_00b0fbfc? -> `OBSERVER.PAL`.` (evidence: `0x0072F3DF..EF`, string `0x008453F4`)
- `[RESOLVED] OQ7 - Is there a Soviet-specific source palette for DAT_0087f6cc? -> No in this slice; `PaletteLoad` uses `SIDEBAR.PAL` all sides for `DAT_00b0fbe4`, and `SidebarClass::LoadSHPs` does not branch by side.` (evidence: `0x0072F3CA..DA`, `0x006A5840`)
- `[RESOLVED] OQ8 - Does this prove ordinary build cameos use OBSERVER.PAL? -> No; the direct ordinary build-cameo chrome xrefs read `DAT_0087f6cc`, while `DAT_0087f6d0` xrefs are in the observer branch.` (evidence: `0x006A9B2B`, `0x006AA144`)
- `[RESOLVED] OQ9 - Are DIALOG palettes involved in this in-game sidebar `cc/d0` setup? -> No.` (evidence: no DIALOG string/xref in the `cc/d0` construction path; prior `ALLIED_SIDEBAR_PALETTE_SELECTOR_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ10 - What exact observer list assets use `DAT_0087f6d0`?` (category: out-of-scope; reason: this requires a full observer sidebar branch trace; next-step-if-pursued: investigate `StripClass::Draw` observer branch and `DAT_00b0b490..b4c8` loads)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `SidebarClass::LoadSHPs` `0x006A5840` | always during sidebar SHP load | repair/sell/tab/gclock SHPs loaded nearby | loader only | constructs `DAT_0087f6cc` from `SIDEBAR.PAL` raw copy | Yes | chrome/gadget ConvertClass |
| 2 | `StripClass::Draw` `0x006A9B2B`, `0x006A9B9D`, `0x006A9E7B` | normal player branch | `DAT_00b07bc0` / `DAT_00b0b484` cameo/clock overlays | strip cell positions | `DAT_0087f6cc` | Yes | build-cameo chrome/overlay |
| 3 | `StripClass::Draw` `0x006AA144`, `0x006AA2BA` | observer branch (`g_PlayerPtr == DAT_00ac1198`) | observer side/icon SHPs from `DAT_00b0b490..` | observer list rows | `DAT_0087f6d0` = `OBSERVER.PAL` | Conditional | observer sidebar content/icons |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `SIDEBAR.PAL` | yes | via ConvertClass | yes for ordinary sidebar chrome | no | yes | yes | no | no | `0x0072F3CA..DA`, `0x006A58AD`, `0x006A9B2B` |
| `OBSERVER.PAL` | yes | via ConvertClass | conditional observer branch | yes in observer list | no | no | no | not ordinary player strip | `0x0072F3DF..EF`, `0x006A5AA5`, `0x006AA144` |
| `CAMEO.PAL` | yes in `0x0052BA60` | not proven in `cc/d0` path | not proven by this report | possible separate cameo asset palette | no | no | no | inactive for `cc/d0` setup | `0x0052C089`, `0x0052C075` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary Allied/Soviet sidebar repair/sell/tab/cameo chrome uses `DAT_0087f6cc`, built from `SIDEBAR.PAL`, with no Soviet-specific palette branch. | `0x006A584E`, `0x006A58AD`, `0x0072F3CA..DA`, string `SIDEBAR.PAL` | none observed for `sidebar_chrome.rs` Soviet `sidebar.pal` choice | `src/render/sidebar_chrome.rs` | Keep Allied/Soviet chrome rendering on `sidebar.pal`; side-specific difference comes from side MIX art, not a Soviet palette. | `sidebar_chrome_soviet_uses_sidebar_pal_not_dialog_or_cameo` | Do not use `DIALOG.PAL` or `CAMEO.PAL` for repair/sell/tab chrome. |
| `DAT_0087f6d0` is `OBSERVER.PAL` and is directly read by observer branch draws, not proved as the ordinary build-cameo palette. | `0x006A5A48`, `0x006A5AA5`, `0x0072F3DF..EF`, `0x006AA144`, `0x006AA2BA` | observer sidebar rendering unchecked/missing | future observer sidebar render surface; currently separate from `src/render/sidebar_cameo_atlas.rs` | If observer sidebar gets implemented, use `observer.pal` for the `DAT_0087f6d0` icon/SHP branch. | `observer_sidebar_icons_use_observer_pal_convertclass` | Do not switch ordinary build cameos to `observer.pal` from this evidence. |
| `0x0052BA60` constructs `DAT_0087f6b4` from `CAMEO.PAL`, but that does not feed `DAT_0087f6cc/d0`. | `0x0052C089`, `0x0052C070`, `0x0052C1FB`; negative xrefs to `cc/d0` | current build cameo atlas uses `cameo.pal` first; not invalidated by this report | `src/app_init_helpers.rs`, `src/render/sidebar_cameo_atlas.rs` | Keep build-cameo palette decision separate from `cc/d0`; future proof should trace ordinary cameo SHP draw source, not infer from global name. | `build_cameo_palette_not_derived_from_sidebar_chrome_globals` | Do not cite `DAT_0087f6d0` as proof that normal build cameos use `OBSERVER.PAL`. |

### Negative Facts / Do Not Do

- Do not describe `0x0052BA60` as the writer of `DAT_0087f6cc` or `DAT_0087f6d0`; those writes are in `SidebarClass::LoadSHPs`.
- Do not use `DIALOG.PAL`, `DIALOGY.PAL`, or `DIALOGN.PAL` for in-game repair/sell/tab chrome.
- Do not invent a Soviet-only `DAT_0087f6cc` palette branch; Allied and Soviet share `SIDEBAR.PAL` for this global.
- Do not treat `OBSERVER.PAL` as ordinary build-cameo evidence; the direct `DAT_0087f6d0` consumers found here are observer-branch draws.
- Do not collapse `CAMEO.PAL` global `DAT_0087f6b4` and sidebar chrome global `DAT_0087f6cc`; they are separate ConvertClass objects.

### Remaining Uncertainty

- The exact observer sidebar asset matrix using `DAT_0087f6d0` is not fully traced. This does not affect the `DAT_0087f6d0` source-palette conclusion, but it limits claims about what every observer-row SHP represents.
- This report does not prove the full ordinary build cameo art palette path; it only proves the `DAT_0087f6cc/d0` globals and their direct consumers in the touched draw ranges.

### Stale Docs / Follow-up Docs

Replacement wording for stale or ambiguous sidebar docs:

- Replace "0x0052BA60 sets `DAT_0087f6cc` / `DAT_0087f6d0`" with: "`0x0052BA60` constructs general palette ConvertClass globals including `CAMEO.PAL`, but `DAT_0087f6cc` and `DAT_0087f6d0` are rebuilt later in `SidebarClass::LoadSHPs` from `PaletteLoad` raw buffers."
- Replace "DIALOG.PAL/side dialog palettes feed in-game sidebar chrome" with: "In-game repair/sell/tab/build-strip chrome uses `DAT_0087f6cc`, a `SIDEBAR.PAL` ConvertClass; DIALOG-family palettes are loading-screen palettes."
- Replace "OBSERVER.PAL is wrong for sidebar" with role-scoped wording: "`OBSERVER.PAL` is wrong for ordinary repair/sell/tab chrome (`DAT_0087f6cc`) but correct for `DAT_0087f6d0`, whose direct consumers found here are observer-branch sidebar draws."

## Sources

- Ghidra MCP read-only: `decompile_function 0x0052BA60`, `0x006A5849`, `0x0072F4A0`, `0x0072F350`, `0x006A9B2B`, `0x006AA144`, `0x006BE4C6`
- Ghidra MCP read-only assembly/xrefs: `get_bulk_xrefs 0x0087f6cc,0x0087f6d0,...`; `get_assembly_context` for `0x0052BE20..0x0052C1FB`, `0x006A584E..0x006A5AA5`, `0x006A9B2B`, `0x006AA144`, `0x0072F3CA..0x0072F3EF`
- Ghidra MCP read-only strings: `SIDEBAR.PAL` at `0x0084542C`, `OBSERVER.PAL` at `0x008453F4`, `CAMEO.PAL` at `0x008204E0`, `MOUSEPAL.PAL` at `0x00826084`, `GRFXTXT.PAL` at `0x00826078`
- Prior docs referenced: `docs/research/ALLIED_SIDEBAR_PALETTE_SELECTOR_GHIDRA_REPORT.md`, `docs/research/SIDEBAR_REPAIR_SELL_BUTTON_GHIDRA_REPORT.md`
- Rust files scanned: `src/render/sidebar_chrome.rs`, `src/render/sidebar_cameo_atlas.rs`, `src/app_init_helpers.rs`, `src/render/sidebar_text.rs`

Status: COMPLETE
