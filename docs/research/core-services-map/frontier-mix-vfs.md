# Core Service Profile — `frontier-mix-vfs`

**Service:** MIX virtual file system + asset loaders
**Slug:** `frontier-mix-vfs`
**Layer:** `asset` (lowest engine layer — the from-scratch file/asset substrate everything mounts on)
**Status:** PROMOTED from catalog stub H1 (`_frontier.md`). Profile, not full decode.
**Plug point:** LOAD-TIME (boot + map/scenario load + lazy SHP/palette reloads). NOT a per-tick spine rung — out-of-sim; upstream of every studied service. No `LogicClass::PerTickUpdate` rung; no render/audio loop rung.

---

## 0. Session verification note (READ FIRST)

The Ghidra MCP bridge was **not connected this session** (no running instance:
`list_instances` → `{"instances": []}`; `check_tools` → `0/3 callable`;
`connect_instance gamemd` → connection refused). Per authority order (binary → Ghidra →
docs), live re-decompilation was unavailable, so representative-address verification falls
back to the **prior `[ghidra/verified]` research corpus**, where the load-bearing addresses
of this service are independently FULL-decompiled across **three** separate reports dated
within the last ~5 weeks:

- `docs/research/bridges/01-assets-map-load-overlay/ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md`
  §12 — function inventory, every address "decompiled this round."
- `docs/research/MPSSCRNL_DUPLICATE_RUNTIME_WINNER_CACHE_STATES_GHIDRA_REPORT.md`
  (2026-05-27) — cache + global-list + sentinel internals, COMPLETE.
- `docs/research/LOADFILEFROMMIX_SIDEBAR_SIDE_RESOLVER_ORDER_GHIDRA_REPORT.md`
  (2026-05-27) — resolver precedence + cache-first, COMPLETE.

Per `feedback_doc_family_confidence`: doc-only confidence (even cross-corroborated) is
weaker than a live Ghidra read. Addresses below are marked **CORROBORATED** (read out of
≥2 independent verified docs this session) vs **UNVERIFIED** (could not confirm at all).
Anything load-bearing for the Rust port should get one fresh live decompile when a Ghidra
instance is up — flagged in §9.

---

## 1. Purpose

`frontier-mix-vfs` is the engine's file substrate: the `.mix` archive virtual file system
plus the from-scratch asset parsers (SHP / VXL / HVA / PAL / TMP / AUD / CSF) layered on the
`FileClass` hierarchy. Every other service reads its data through this layer. It owns:

- **Mount order** of `.mix` archives into one global doubly-linked list (head/tail
  sentinels), including the neutral / right-panel archives and the per-side archives.
- **File resolution**: a filename-CRC cache checked *before* any archive search, then a
  first-match scan of the global MIX list, with a loose-file (raw filesystem) probe taking
  priority inside the file-object open path.
- **Asset parsing**: turning raw archive bytes into in-memory SHP frames, voxel/HVA geometry,
  palettes, theater tiles, and audio samples.

The Rust counterpart is the `assets/` + `util/` layer (`src/assets/mix_archive.rs`,
`asset_manager.rs`, `shp_file.rs`, `vxl_file.rs`, `pal_file.rs`, `tmp_file.rs`,
`src/util/lcw.rs`, `src/util/lzo.rs`).

---

## 2. What it owns (globals / structs)

| Global / struct | Address | Role | Source |
|---|---|---|---|
| MIX global-list head sentinel | `0x00ABEFDC` | head node of the doubly-linked archive list | LOADFILEFROMMIX §5 (CORROBORATED) |
| MIX global-list tail sentinel | `0x00ABEFE8` | tail node | LOADFILEFROMMIX §5 (CORROBORATED) |
| `DAT_00ABEFE0` | `0x00ABEFE0` | `head.next` — scan start for the resolver | LOADFILEFROMMIX §4–5, MPSSCRNL §5 (CORROBORATED) |
| `DAT_00ABEFF0` | `0x00ABEFF0` | `tail.prev` — new archives insert after this, before tail (append) | LOADFILEFROMMIX §5, MPSSCRNL §5 (CORROBORATED) |
| `DAT_00ABF00C` | `0x00ABF00C` | root of the `LoadFileFromMIX` filename-CRC **cache tree** (checked first) | LOADFILEFROMMIX §3, MPSSCRNL §3 (CORROBORATED) |
| `_DAT_00ABEFF8`..`_DAT_00ABF004` | `0x00ABEFF8`+ | cleared by `MixFileSystem_Reset` (does NOT clear cache root) | MPSSCRNL §5 (CORROBORATED) |
| `DAT_00884E58` | `0x00884E58` | mounted `NTRLMD.MIX` object | MPSSCRNL §4 (CORROBORATED) |
| `DAT_00884E5C` | `0x00884E5C` | mounted `NEUTRAL.MIX` object | MPSSCRNL §4 (CORROBORATED) |
| `DAT_00884E70` | `0x00884E70` | side archive `SIDEC%02dMD.MIX` | SIDE_MIXFILE §4, LOADFILEFROMMIX §2 (CORROBORATED) |
| `DAT_00884E74` | `0x00884E74` | side archive `SIDEC%02d.MIX` (MANDATORY) | SIDE_MIXFILE §4 (CORROBORATED) |
| `DAT_00884E78` | `0x00884E78` | side archive `SIDENC%02d.MIX` | SIDE_MIXFILE §4 (CORROBORATED) |
| `DAT_00884E68` | `0x00884E68` | released by `InitSideMixFiles` but never written there; writer unknown | SIDE_MIXFILE Q11 (UNVERIFIED writer) |
| `DAT_0087E734` | `0x0087E734` | mounted `AUDIOMD.MIX`/`AUDIO.MIX` object | AUDIO_IDX_BAG §1 (CORROBORATED) |
| `DAT_0087E724` | `0x0087E724` | `AudioIndex` object (audio.idx/.bag handler) | AUDIO_IDX_BAG §1 (CORROBORATED) |

**Per-archive node layout (partial):** node `+0x04` = next-link followed by the resolver
scan; `[3]` (i.e. `+0x0C`) = filename field logged on release. Full `MixFileClass` struct
layout was not enumerated this session.

**Object/struct families this service defines:**
- `FileClass` → `RawFileClass` (loose files) → `BufferIOFileClass` / `CDFileClass`
  (`MixFileClass` ctor) → `CCFileClass` (the generic wrapper the loaders use).
- `ConvertClass` — palette→surface remap object (built in `PaletteLoad`).
- `AudioIndex` — the audio.idx/.bag random-access reader (0x124-byte object).

---

## 3. Key functions (re-verified status this session)

### 3.1 Representative function — `LoadFileFromMIX @ 0x005B40B0` — **CORROBORATED (FULL decompile in 3 docs)**
The stub's representative address. Confirmed correct. Behavior:
1. Copy + uppercase the requested filename (`FUN_007DCFC4`), CRC it (`CRCEngine__AddData`).
2. Walk cache tree `DAT_00ABF00C`; a node with matching CRC and nonzero payload returns
   that payload **immediately** — before any `CCFileClass` construction or archive search.
3. On miss: construct `CCFileClass`, call `FUN_00473C50(0)` (file open / availability),
   which on a loose-file miss falls through to the archive resolver `FUN_005B4430`.
4. Insert a new cache node (CRC → payload) via `FUN_005B3FF0` and return.

The filename cache is the determinism-relevant boundary: **first winner is sticky** across
later re-mounts and normal cleanup (cleanup unlinks archives but does NOT clear
`DAT_00ABF00C`). Evidence: LOADFILEFROMMIX §3 (`0x005B4129..0x005B41A2`), MPSSCRNL §3.

### 3.2 `FUN_005B4430` — archive resolver (global-list first-match scan) — CORROBORATED
Uppercases + hashes the name, loads `DAT_00ABEFE0` (head.next), binary-searches that
archive's sorted entry table, advances via node `+0x04` on miss. **First matching archive
wins.** Evidence: LOADFILEFROMMIX §4 (`0x005B44A1`, `0x005B44CB..0x005B450B`).

### 3.3 `MixFileSystem_InitSentinels @ 0x005B3AC0` — CORROBORATED
Initializes head/tail sentinel links (`DAT_00ABEFE0 = &tail`-region, `DAT_00ABEFF0 =
&head`-region per the constructor's insert math). Evidence: MPSSCRNL §5
(`0x005B3AC0..0x005B3B06`).

### 3.4 `CDFileClass__Constructor @ 0x005B3C20` (= `MixFileClass` ctor) — CORROBORATED
The constructor every `.mix` open routes through (audio, neutral, side, map). Inserts the
new node **after `DAT_00ABEFF0`, before the tail sentinel** = APPEND. Evidence: MPSSCRNL §5
+ AUDIO_IDX_BAG §1 (`0x005B3DE2..0x005B3E00`). This is the single correction to make about
"prepend vs append": gamemd APPENDS; the Rust `AssetManager::load_nested` inserts at index 0
(prepend) — a known divergence (LOADFILEFROMMIX Negative Facts).

### 3.5 `MixFileClass` destructor `@ 0x005B4630` — CORROBORATED
Unlinks the node, frees the archive's directory/body allocations, but does NOT touch the
filename cache `DAT_00ABF00C`. `MixFileSystem_Reset @ 0x005B3AA0` clears
`_DAT_00ABEFF8..0x00ABF004` only — also not the cache root. Evidence: MPSSCRNL §5.

### 3.6 `CCFileClass__Constructor @ 0x004739F0` — CORROBORATED
Trivial wrapper: sets the CCFile vtable around a CDFile. The generic file object the
loaders construct. Its vtable `+0x14` is availability/open; `+0x1C` is read-open; in read
mode it probes the raw filesystem first (`FUN_00431F10(0)`), then MIX fallback. Evidence:
ASSET_PARSING §2.6, SKIRMISH_SELECTED_MAP §3.4.

### 3.7 `InitSideMixFiles @ 0x00534FA0` — CORROBORATED (FULL, dedicated report)
Loads up to three per-side archives for the active side (0=Allied, 1=Soviet, 2=Yuri, with
hard `if side==2: side=1`). Open order: `SIDEC%02dMD.MIX` (optional) → `SIDEC%02d.MIX`
(MANDATORY, missing = return 0) → `SIDENC%02d.MIX` (optional, gated on the mandatory one).
Format strings at `0x00827dd4` / `0x00827de4` / `0x00827e0c`. Releases 4 slots
(`DAT_00884E68/E74/E70/E78`) at entry on every call. Callers: `ScenarioClass__Full_Init`
(`0x00686B20`, twice in singleplayer) and save/load restore `FUN_0067E730`. Active in YR:
YES. Evidence: SIDE_MIXFILE_INIT report (full).

### 3.8 `FUN_00534E50` — neutral / right-panel archive mounter — CORROBORATED
**This is the actual NEUTRAL/NTRLMD mounter** (the stub's "MIX_LoadNeutral" intent).
Releases old neutral objects, opens `NTRLMD.MIX` into `DAT_00884E58`; on success opens
`NEUTRAL.MIX` into `DAT_00884E5C`. Mounts `NTRLMD.MIX` BEFORE `NEUTRAL.MIX`, so YR
duplicates win first-match. Lazy-init guarded by `DAT_00B0FBE0 == 0`; callers include
`RightPanel__Draw @ 0x0072E450`, `SidebarSurface__Init @ 0x0072DDB0`. Evidence: MPSSCRNL §4.

### 3.9 Asset parsers layered on the VFS — CORROBORATED (ASSET_PARSING §12 inventory)
| Parser | Address | Note |
|---|---|---|
| `SHP_Resolve` | `0x0069E580` | SHP header/frame resolution; frame getters `0x0069E740`, `0x0069E7E0` |
| `Standard_SHP_blitter` | `0x004373B0` | format 0–3 codec dispatch (format 3 = YR std); inner codec not byte-decoded |
| `Extended_SHP_blitter` | `0x00437A10` | extended blit path |
| `TMP_Loader` | `0x00547020` | theater tile (`.tmp`) loader |
| `PaletteLoad` | `0x0072F350` | `.pal` 6-bit→8-bit (`<<2`) into `ConvertClass`; teardown+reload left-panel SHPs |
| `MIX_LoadNeutral` | `0x0072FA10` | **NOT an archive mounter** — SHP-reload helper inside `PaletteLoad`/`LeftPanel__Draw` lazy-init (see §6 correction) |
| `LCW` decompressor | `0x00551C60` | Westwood LCW (used by IsoMapPack5, SHP) |
| IsoMapPack5 decoder | `0x0056BAC0` | map cell-pack |
| `VXL_Load_File` | `0x00755DB0` | voxel geometry; opened via `CCFileClass` |
| `HVA_Load_File` | `0x005BD5C0` | voxel per-frame matrices; via `CCFileClass` |
| `SampleTracker__LoadSample` | `0x00401C00` | AUD sample load |
| `AudioIndex__Constructor` | `0x004011C0` | audio.idx/.bag reader (binary-searched) |
| `IMA_ADPCM__DecodeSample` | `0x0040ACD0` | ADPCM audio decode |
| CSF parser | **UNVERIFIED** | UNRESOLVED in ASSET_PARSING §8 — string-table format not located |

---

## 4. Plug point (load-time, not the tick spine)

`frontier-mix-vfs` does **not** appear in `LogicClass::PerTickUpdate @ 0x0055AFB0` (the 28
A–AB rungs in `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`) and has no render-pass or audio-loop
rung. It runs at:

- **Boot:** audio MIX mount + `AudioIndex` build (`FUN_00406B10`), top-level archive mounts.
- **Scenario load:** `ScenarioClass__Full_Init @ 0x00686B20` → `InitSideMixFiles @ 0x00534FA0`
  (side archives) and theater/tile loads; map cell-packs decoded here.
- **Lazy reloads:** right-panel/neutral mount `FUN_00534E50` and `PaletteLoad`/left-panel
  reloads fire on first draw / side switch — still out-of-sim, render-thread-side, but
  data-producing not per-tick.

It is **upstream of every studied service** — they consume parsed data it produced, but it
ticks nothing. The only timing parity concern is *startup/side-switch asset selection*, not
per-frame ordering.

---

## 5. Edges

### OUTGOING (what this service depends on)
| → Service | Via symbol / evidence | Notes |
|---|---|---|
| `ini-parsing` | `CCINIClass` reads `UIMD.INI`, theater INI, `Rules` through the VFS file objects (`CCFileClass`). `InitSideMixFiles` opens `UIMD.INI` then `RulesClass__ReadCommandBar`. | Sits *beside* this service — INI parser reads files this VFS resolves. Lowest-layer peer. |
| `lookup-tables` | CRC engine (`CRCEngine__AddData`) + uppercase table (`FUN_007DCFC4`) for the cache key; PAL `<<2` expansion table. | Filename-hash + palette math. |

This service is otherwise the **lowest layer** — it depends on nothing in the studied 18 for
its core file-resolution behavior (the stub's "most depends on: nothing in the 18" is
correct).

### INCOMING (who depends on this service)
Effectively **every** service that touches an asset, at load time:

| ← Service | Via symbol / evidence |
|---|---|
| `rules-class` | reads `rules(md).ini` / `art(md).ini` through `CCFileClass`; theater + tile data via `TMP_Loader`/`Init_Theater` |
| `cell-map` | theater tile graphics (`TMP_Loader @ 0x00547020`), IsoMapPack5 decode `0x0056BAC0` at map load |
| `frontier-sidebar` | side-archive resolution (`InitSideMixFiles`) feeds cameo/sidebar SHPs; `FUN_006D02B0` button art |
| `frontier-radar` | neutral-archive `MPSSCRNL.SHP` / radar SHPs via `FUN_00534E50` mount order |
| `frontier-render-tactical` / `frontier-anim` / `frontier-voxelanim` | SHP frames (`SHP_Resolve`), voxel geometry (`VXL_Load_File`/`HVA_Load_File`), palettes (`PaletteLoad`) |
| `frontier-audio-voc` / `frontier-audio-eva` | AUD samples via `AudioIndex` (`audio.idx`/`audio.bag`), `SampleTracker__LoadSample`, ADPCM decode |
| `frontier-saveload` | save/load restore `FUN_0067E730` re-runs `InitSideMixFiles` on load |
| `frontier-input-command` / movie players | Bink/VQA movie file objects open through `RawFileClass`/`CCFileClass` then bypass the MIX cache (direct byte reads) |

The whole render/audio/sidebar/radar/rules/map stack consumes this layer; none of them feed
*into* it.

---

## 6. Stub corrections (this session)

1. **`MIX_LoadNeutral @ 0x0072FA10` is mislabeled in the stub.** The stub lists it as a
   neutral-archive mounter. It is actually a **SHP-reload helper** inside `PaletteLoad`
   (`0x0072F350`) and `LeftPanel__Draw`'s lazy-init — it reloads left-panel SHPs, it does
   **not** mount `NEUTRAL.MIX`. The real neutral/NTRLMD archive mounter is **`FUN_00534E50`**
   (mounts `NTRLMD.MIX` then `NEUTRAL.MIX`). Evidence: ALLIED_SIDEBAR_PALETTE §7 caller
   chain (`PaletteLoad → MIX_LoadNeutral // reload SHPs`); MPSSCRNL §4 (`FUN_00534E50` is the
   mounter).
2. **`MixFileSystem_InitSentinels @ 0x005B3AC0`** in the stub is correct, but the stub did
   not name the `.mix` constructor itself — it is **`CDFileClass__Constructor @ 0x005B3C20`**
   (= `MixFileClass` ctor), and the resolver scan is **`FUN_005B4430`**, the cache root is
   **`DAT_00ABF00C`**, and the cache insert is **`FUN_005B3FF0`**. These are the load-bearing
   internals the stub elided.
3. **Append, not prepend.** New archives append before the tail sentinel; the Rust port
   currently prepends (`load_nested` index 0) — a real divergence to fix (LOADFILEFROMMIX).

---

## 7. Active-in-YR vs TS legacy

| Behavior | Active in YR |
|---|---|
| MIX global list + first-match resolver + filename-CRC cache | YES — every asset load |
| Append-before-tail mount order | YES |
| `NTRLMD.MIX` before `NEUTRAL.MIX` (YR-duplicate-wins) | YES — stock cold load |
| Yuri→Soviet side-archive substitution (`if side==2: side=1`) | YES — unconditional |
| `InitSideMixFiles` double-call in singleplayer (`g_GameMode==0`) | YES — campaign |
| `-1` sentinel / CD disk-detection path in `InitSideMixFiles` | TS-LEGACY / DEAD — no caller passes -1 in YR skirmish (SIDE_MIXFILE §6) |
| `FUN_005B43F0` stub calls (return 1, inert) | called but functionally dead |
| `RawFileClass` loose-file-first probe (raw filesystem before MIX) | YES — loose files shadow archive entries (SKIRMISH_SELECTED_MAP §3.4) |

No subterranean/tunnel coupling. The only dead path inside this service is the legacy
CD disk-detection sentinel.

---

## 8. Lockstep / determinism note

The filename-CRC cache (`DAT_00ABF00C`, first-winner-sticky) and the deterministic
first-match archive scan together make **asset identity deterministic given mount order**.
This is not RNG-coupled and not per-tick, so it is not a lockstep desync surface in the
event-queue sense — but it IS a *content-parity* surface: if the Rust port resolves a
different duplicate (e.g. prepend vs append, or base `NEUTRAL` instead of `NTRLMD`), the
player sees different bytes (different SHP encoding/size for some duplicates per MPSSCRNL).
That is a pixel-parity bug, surfaced here, not triaged out.

---

## 9. Follow-ups / what needs a fresh live Ghidra read

- **Re-confirm `LoadFileFromMIX @ 0x005B40B0` body live** when an instance is up (3-doc
  corroboration is strong but doc-only per `feedback_doc_family_confidence`).
- **`DAT_00884E68` writer** — released by `InitSideMixFiles`, written elsewhere; unknown.
- **CSF parser** — UNRESOLVED (ASSET_PARSING §8); locate via CSF magic / string-table xrefs.
- **`FUN_007DCFC4`** filename normalize — uppercase-only vs path/extension canonicalization
  (affects whether Rust filename hashing is byte-identical).
- **SHP format-3 codec byte syntax** — not extracted (ASSET_PARSING §11.7).
- **Full `MixFileClass` struct layout** — only `+0x04` (next) and `[3]` (filename) confirmed.

---

## Sources

- `docs/research/core-services-map/_frontier.md` §H1 (seed stub).
- `docs/research/bridges/01-assets-map-load-overlay/ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md`
  §2.6, §12 (parser inventory, `LoadFileFromMIX` FULL).
- `docs/research/MPSSCRNL_DUPLICATE_RUNTIME_WINNER_CACHE_STATES_GHIDRA_REPORT.md`
  (cache/list/sentinel internals, `FUN_00534E50` neutral mounter).
- `docs/research/LOADFILEFROMMIX_SIDEBAR_SIDE_RESOLVER_ORDER_GHIDRA_REPORT.md`
  (resolver precedence, append-before-tail, cache-first).
- `docs/research/SIDE_MIXFILE_INIT_GHIDRA_REPORT.md` (`InitSideMixFiles` full).
- `docs/research/ALLIED_SIDEBAR_PALETTE_SELECTOR_GHIDRA_REPORT.md` (`MIX_LoadNeutral` is a
  SHP reload, not an archive mount — stub correction).
- `docs/research/AUDIO_IDX_BAG_GHIDRA_REPORT.md` (audio MIX mount + `AudioIndex`).
- `docs/research/VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md` (voxel loaders via `CCFileClass`).
- `docs/research/LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` (confirms NO tick rung — out-of-sim).
- Session Ghidra state: bridge NOT connected (`list_instances` empty; `connect_instance`
  refused) — addresses CORROBORATED from the above verified corpus, not freshly decompiled.
```
