# MPSSCRNL Duplicate Runtime Winner / Cache States - Ghidra Research Report

Date: 2026-05-27

## Working Notes

Target question: Prove which duplicate `MPSSCRNL.SHP` wins at runtime in relevant cache/load states for Soviet radar transition asset resolution.

Non-goals: Do not re-prove Soviet filename selection except where needed for liveness; do not investigate radar placement/draw composition; do not modify Rust, INI, or tracked docs outside `docs/research/`.

Evidence needed to mark COMPLETE: read-only Ghidra proof of `MPSSCRNL.SHP` call path, `LoadFileFromMIX` cache-first behavior, global MIX first-match order, `NTRLMD.MIX`/`NEUTRAL.MIX` mount order, cleanup/reload behavior, and retail duplicate membership.

Stop conditions: stop if Ghidra read-only access is unavailable; stop if runtime debugger state is required to prove normal stock cold-load order; stop after this report and shared claims update.

Investigation mode: exhaustive-slice for static stock YR loader/cache state transitions; coverage-map for live debugger-only cache contents.

## Summary

For stock YR cold load with neutral/right-panel archives mounted by `FUN_00534E50`, `MPSSCRNL.SHP` resolves from `NTRLMD.MIX`, not base `NEUTRAL.MIX`. The reason is mechanical: `FUN_00534E50` opens `NTRLMD.MIX` first and `NEUTRAL.MIX` second; `MixFileClass` appends new archives before the tail sentinel; `FUN_005B4430` scans the global list from first to last. Since the retail duplicate exists in both `ra2md.mix -> ntrlmd.mix` and `ra2.mix -> neutral.mix`, the YR duplicate in `NTRLMD.MIX` wins on a cache miss.

On any later load where `MPSSCRNL.SHP` is already in `LoadFileFromMIX`'s filename cache, the cache wins before any archive search. The radar-transition cleanup path clears the global pointer and destroys the file wrapper, but for a MIX-backed load it does not free the returned payload and does not clear the `LoadFileFromMIX` cache. Therefore normal cleanup/reload preserves the first winner.

## Verified Findings

### 1. Soviet non-640 transition path actively requests exactly `MPSSCRNL.SHP`

Active in YR: Yes.

`RadarTransitionMovie_SHPLoad @ 0x0072D830` branches on side and width. For side `1` and `g_ScreenWidth != 0x280`, assembly loads filename pointer `[0x00844CAC]` then calls `CDFileClass__Constructor @ 0x004A38D0` and stores the result to `g_MinimapMovie_SHP @ 0x00B0FB1C`.

Evidence:

- `0x0072D86C`: `CMP ECX,0x1`.
- `0x0072D871`: `CMP EAX,0x280`.
- `0x0072D87B`: non-640 jumps to `0x0072D88E`.
- `0x0072D88E`: `MOV ECX,dword ptr [0x00844CAC]`.
- `0x0072D894`: `CALL 0x004A38D0`.
- `0x0072D899`: `MOV [0x00B0FB1C],EAX`.
- `read_memory(0x00844CAC,4) -> 0x008451E4`; `read_memory(0x008451E4,16) -> "MPSSCRNL.SHP\0"`.

Liveness evidence:

- `FUN_0072D730 @ 0x0072D730` calls `RadarTransitionMovie_SHPLoad` only when `DAT_00B0FBB8 == 0`, then sets `DAT_00B0FBB8 = 1`.
- Caller `FUN_005C9720 @ 0x005C9720` reads `ScenarioClass+0x34B8` into `ECX`, calls `0x0072D730`, then calls `FUN_0072EAD0`, the direct minimap movie draw path.

### 2. `CDFileClass__Constructor @ 0x004A38D0` uses `LoadFileFromMIX` first and leaves MIX-backed cleanup flag false

Active in YR: Yes.

The transition selector calls `0x004A38D0` with `ECX = filename` and `EDX = 0x00B0FC7D`. The constructor first writes `*param_2 = 0`, then calls `LoadFileFromMIX`. If `LoadFileFromMIX` returns a payload pointer, the constructor returns immediately and leaves `DAT_00B0FC7D == 0`. Only the fallback loose/direct file path sets `*param_2 = !bVar4`.

Evidence:

- `CDFileClass__Constructor @ 0x004A38D0` decompile: `*param_2 = 0; pvVar2 = LoadFileFromMIX(); if (pvVar2 != 0) return pvVar2; ... *param_2 = !bVar4;`.
- Selector assembly sets `EDX = 0xB0FC7D` before the filename branch and call.
- Cleanup `FUN_0072D780` frees `g_MinimapMovie_SHP` only if `DAT_00B0FC7D != 0`; for stock MIX loads the flag remains zero.

Why it matters: the normal stock `MPSSCRNL.SHP` payload is not freed by the transition cleanup. The global pointer is cleared, but the cached MIX payload remains eligible for future cache hits.

### 3. `LoadFileFromMIX @ 0x005B40B0` checks filename cache before archive search

Active in YR: Yes.

`LoadFileFromMIX` copies and uppercases the filename, CRCs it, then walks cache tree `DAT_00ABF00C`. If a node has matching CRC and nonzero payload, it returns that payload before constructing `CCFileClass` or calling the MIX resolver. Only cache misses call `FUN_00473C50`, then insert a new node with the CRC and returned payload.

Evidence:

- `LoadFileFromMIX @ 0x005B40B0` decompile: cache root `puVar3 = DAT_00ABF00C`; matching `puVar3[2] == iVar2 && puVar3[3] != 0` returns `puVar3[3]`.
- Cache miss path constructs `CCFileClass`, calls `FUN_00473C50(0)`, allocates a 0x10 node, stores `puVar3[2] = iVar2`, loads file payload, then stores `puVar3[3] = uVar4`.
- Insertion helper `FUN_005B3FF0` only links the cache node by CRC; it does not encode archive source.

Cache-state consequence:

- Cold/no cache: archive order decides.
- Cached from `NTRLMD.MIX`: `NTRLMD` payload wins even if archives are later remounted.
- Cached from `NEUTRAL.MIX` in a nonstandard earlier state: `NEUTRAL` payload wins until process/cache reset, even if `NTRLMD.MIX` is later mounted earlier.

### 4. Cold stock right-panel/neutral archive mount order is `NTRLMD.MIX` before `NEUTRAL.MIX`

Active in YR: Yes.

`FUN_00534E50` releases old neutral archive objects, then opens `NTRLMD.MIX` into `DAT_00884E58`. If that succeeds, it opens `NEUTRAL.MIX` into `DAT_00884E5C`. The right-panel init paths call this helper before loading right-panel resources.

Evidence:

- `FUN_00534E50 @ 0x00534E50` decompile:
  - allocate object at `0x00534F13`;
  - `CDFileClass__Constructor(s_NTRLMD_MIX_00827DA0, &DAT_00886980)` at `0x00534F21..0x00534F34`;
  - if `DAT_00884E58 == 0`, return `0`;
  - allocate second object at `0x00534F53`;
  - `CDFileClass__Constructor(s_NEUTRAL_MIX_00827D80, &DAT_00886980)` at `0x00534F61..0x00534F74`.
- Callers include `RightPanel__Draw @ 0x0072E450`, `FUN_0072DFB0`, `FUN_0072AA40`, and `SidebarSurface__Init @ 0x0072DDB0`; each lazy-init path calls `FUN_00534E50` before right-panel SHP loads when `DAT_00B0FBE0 == 0`.
- Prior retail asset dump proves `MPSSCRNL.SHP` exists in both `ra2md.mix -> ntrlmd.mix` and `ra2.mix -> neutral.mix`; both have canvas `632x568`, frame `0`, zero offset, but the YR duplicate is format `2` and larger (`360144` bytes vs base `359008` bytes).

### 5. Global MIX list order preserves that mount order, and cleanup removes archives from the list without clearing the file cache

Active in YR: Yes.

`MixFileSystem_InitSentinels @ 0x005B3AC0` initializes head/tail sentinel links. `MixFileClass` construction at `0x005B3C20` inserts the new node after `DAT_00ABEFF0` (tail.prev) and before the tail sentinel. `FUN_005B4430` scans from `DAT_00ABEFE0` and follows `+0x04` links, so first matching archive wins. The destructor at `0x005B4630` frees the archive's directory/body allocations and unlinks the node, but it does not touch `DAT_00ABF00C`.

Evidence:

- `MixFileSystem_InitSentinels`: `DAT_00ABEFE0 = &tail`, `DAT_00ABEFF0 = &head`.
- Constructor `0x005B3DE2..0x005B3E00`: inserts after previous tail-prev and updates links.
- Resolver `FUN_005B4430`: starts with `iVar6 = DAT_00ABEFE0`, binary-searches current archive, then advances through node `+0x04`.
- Destructor `0x005B4630`: unlinks `param_1[1]`/`param_1[2]`, frees archive allocations, but contains no write to `DAT_00ABF00C`.
- `MixFileSystem_Reset @ 0x005B3AA0` only clears `_DAT_00ABEFF8`..`_DAT_00ABF004`, not the filename cache root.

Runtime winner matrix:

| State | Winner | Active in YR | Evidence |
|---|---|---|---|
| Stock cold cache, `FUN_00534E50` succeeds | `ra2md.mix -> ntrlmd.mix -> MPSSCRNL.SHP` | Yes | `NTRLMD` opened before `NEUTRAL`, append preserves order, first-match scan |
| Later reload after `FUN_0072D780` transition cleanup | previous cached payload, normally `NTRLMD` | Yes | cleanup clears globals but MIX-backed flag remains zero; `LoadFileFromMIX` cache remains |
| Side switch / side-MIX reload only | previous cached `MPSSCRNL` payload if requested again | Conditional | side MIX setup does not alter neutral archive order or clear filename cache; Soviet branch must be reached again |
| Nonstandard cache already seeded from `NEUTRAL.MIX` before `NTRLMD.MIX` | cached `NEUTRAL` payload | Conditional / not standard stock path proven here | cache-first return beats archive order |
| `NTRLMD.MIX` missing/fails in `FUN_00534E50` | no stock cold-load neutral fallback from this helper | No for stock retail; Conditional for broken install/mod | helper returns `0` immediately after `DAT_00884E58 == 0` |

## Implementation Handoff

1. Verified behavior: stock cold `MPSSCRNL.SHP` resolves from `NTRLMD.MIX`, not `NEUTRAL.MIX`. Rust delta: current `src/render/sidebar_chrome.rs` and `src/render/radar_anim.rs` do not model the Soviet `MPSSCRN*` path or duplicate archive precedence. Acceptance scenario: with duplicate `MPSSCRNL.SHP` entries in `ntrlmd` and `neutral`, Soviet non-640 transition picks the YR/ntrlmd bytes. Proposed test: `test_soviet_mpsscrnl_cold_load_prefers_ntrlmd_duplicate`. Risk: HIGH pixel parity risk because the two files have different SHP encoding/bytes.

2. Verified behavior: filename cache wins before archive search and persists across normal transition cleanup. Rust delta: asset resolver/cache for parity-sensitive sidebar/radar assets must model cache boundaries or explicitly prove a cold-only path. Acceptance scenario: load `MPSSCRNL.SHP`, cleanup transition globals, then reload; bytes/source are the same cached payload and no second archive search can swap winner. Proposed test: `test_mpsscrnl_reload_uses_cached_first_winner_after_transition_cleanup`. Risk: MEDIUM-HIGH for shell/side-switch/reinit parity.

3. Verified behavior: `DAT_00B0FC7D` distinguishes fallback loose/direct allocation cleanup from MIX-backed payloads; stock MIX loads leave it false. Rust delta: do not free/drop the transition movie payload merely because `g_MinimapMovie_SHP` is cleared if modeling native cache lifetime. Acceptance scenario: MIX-backed `MPSSCRNL` cleanup nulls the global but cache lookup still returns the original payload. Proposed test: `test_mpsscrnl_mix_backed_cleanup_clears_global_not_cache_payload`. Risk: MEDIUM for reload timing and cache-order drift.

## Negative Facts / Do Not Do

- Do not choose base `neutral.mix` for stock cold YR `MPSSCRNL.SHP`; evidence: `FUN_00534E50` opens `NTRLMD.MIX` before `NEUTRAL.MIX`, and first-match scan preserves that order.
- Do not treat duplicate SHP geometry equality as byte/source equivalence; evidence: prior retail dump shows same `632x568` canvas but different format/byte size for base vs YR duplicate.
- Do not ignore `LoadFileFromMIX` cache when reasoning about cleanup/reload or side switching; evidence: cache tree `DAT_00ABF00C` is checked before `FUN_00473C50`.
- Do not assume transition cleanup frees MIX-backed `g_MinimapMovie_SHP`; evidence: `0x004A38D0` leaves `DAT_00B0FC7D = 0` for `LoadFileFromMIX` hits, and `FUN_0072D780` frees only when that flag is nonzero.
- Do not model side MIX setup as affecting `MPSSCRNL.SHP`; evidence: the target duplicate is in neutral/ntrlmd archives and selected by `RadarTransitionMovie_SHPLoad`, not `InitSideMixFiles` generic side archive loading.

## Remaining Uncertainty

- Live process cache contents were not inspected because the debugger server was not running. Static evidence proves the cache rules; it does not list the current runtime cache in an already-running process.
- The exact top-level boot order of every archive before `FUN_00534E50` was not exhaustively re-enumerated. This does not affect the proved stock cold winner once `NTRLMD.MIX` and `NEUTRAL.MIX` are mounted by `FUN_00534E50`, because their relative order and first-match scan are proven.
- A nonstandard mod/broken-install path where `MPSSCRNL.SHP` is first cached from base `NEUTRAL.MIX` before `NTRLMD.MIX` is mounted remains a conditional cache-state possibility, not a stock YR path proven active here.
- Pixel decode/color differences between the two duplicate payloads were not compared; this report resolves source winner and cache lifetime only.

## Stale-doc replacement wording

For `LOADFILEFROMMIX_SIDEBAR_SIDE_RESOLVER_ORDER_GHIDRA_REPORT.md`, add:

> For Soviet non-640 radar transition `MPSSCRNL.SHP`, the relevant duplicate is not in the side MIX family. The stock cold path mounts `NTRLMD.MIX` before `NEUTRAL.MIX` via `FUN_00534E50`; because `MixFileClass` appends and `FUN_005B4430` scans first-to-last, `ra2md.mix -> ntrlmd.mix -> MPSSCRNL.SHP` wins on a cache miss. Later loads can be cache hits from `LoadFileFromMIX` and preserve the first winner.

For `RETAIL_SOVIET_SIDEBAR_SHP_DIMENSIONS_OFFSETS_GHIDRA_REPORT.md`, replace the deferred duplicate note with:

> Stock cold YR `MPSSCRNL.SHP` resolves from `ra2md.mix -> ntrlmd.mix`, not `ra2.mix -> neutral.mix`, after `FUN_00534E50` mounts `NTRLMD.MIX` before `NEUTRAL.MIX`. The base duplicate remains relevant only for nonstandard cache states or if the YR neutral MD archive is unavailable.

## Sources

- Ghidra read-only decompile/assembly: `RadarTransitionMovie_SHPLoad @ 0x0072D830`
- Ghidra read-only decompile/assembly: `FUN_0072D730`, `FUN_005C9720`
- Ghidra read-only decompile: `CDFileClass__Constructor @ 0x004A38D0`
- Ghidra read-only decompile: `LoadFileFromMIX @ 0x005B40B0`
- Ghidra read-only decompile: `FUN_005B4430`, `FUN_005B3FF0`
- Ghidra read-only decompile/assembly: `FUN_00534E50`
- Ghidra read-only decompile: `MixFileSystem_InitSentinels @ 0x005B3AC0`, `MixFileClass` constructor `0x005B3C20`, destructor `0x005B4630`, `MixFileSystem_Reset @ 0x005B3AA0`
- Existing retail asset dump: `RETAIL_SOVIET_SIDEBAR_SHP_DIMENSIONS_OFFSETS_GHIDRA_REPORT.md`

## Status

COMPLETE.
