# Sidebar Top Strip 0x53A8 Liveness - Ghidra Report

Status: COMPLETE
Date: 2026-05-27
Target: `SidebarClass + 0x53A8`, the byte checked by `SidebarClass::BlitToScreen @ 0x006A70E0` before the top-strip-only `168x16` copy path.
Investigation Mode: exhaustive-slice for direct static liveness of this one field in standard active Yuri's Revenge.

## Summary

`SidebarClass + 0x53A8` is a real byte flag consumed by `SidebarClass::BlitToScreen`, but this pass found no standard active YR setter that arms it. The static executable contains only five byte-sized `+0x53A8` sidebar references: constructor clear, two `BlitToScreen` compares, and two `BlitToScreen` clears after top-strip copies. The other direct `+0x53A8` references in `.text` are 4-byte fields on other classes, mostly `HouseClass+0x53A8` power/defense accounting paths, not the sidebar byte.

Conclusion for implementation: keep the top-strip-only copy branch as a documented native branch in the dirty-rect model, but treat the arming flag as not live in stock standard YR unless runtime evidence later proves an indirect writer. Do not invent a gameplay trigger for it.

## Target and Non-Scope

In scope:

- Determine whether standard active YR directly sets `SidebarClass + 0x53A8`.
- Identify constructor/init value, read/check sites, clear sites, field width, and copy rectangle behavior.
- Distinguish direct sidebar byte references from unrelated `+0x53A8` offsets on other classes.
- Provide Rust-facing acceptance tests for modeling or intentionally not arming the branch.

Non-scope:

- General sidebar layout, SHP selectors, palette routing, cameo ordering, radar content, or `0x53A6/0x53A7` behavior except as contrast.
- Whole save/load serialization proof for every possible object-memory mutation.
- Runtime debugger watchpoint capture of `SidebarClass + 0x53A8` across a live match.

## Verified Binary Findings

### 1. The field is a 1-byte sidebar flag, not a 4-byte field, in the only sidebar consumers.

Evidence: `SidebarClass::Constructor @ 0x006A4EC0` decompile writes `*(undefined1 *)(param_1 + 0x14ea) = 0`, which is byte offset `0x53A8`. Local Capstone disassembly confirms the instruction sequence:

- `0x006A4E77: xor ebx, ebx`
- `0x006A4EC0: mov byte ptr [edi + 0x53A5], bl`
- `0x006A4EC6: mov byte ptr [edi + 0x53A6], 1`
- `0x006A4ECD: mov byte ptr [edi + 0x53A8], bl`

Active in YR: Yes for constructor initialization. `BL` is zero from `xor ebx, ebx`, so new `SidebarClass` instances start with `0x53A8 = 0`.

### 2. `SidebarClass::Draw` does not read or write `+0x53A8`.

Evidence: `SidebarClass::Draw @ 0x006A6C30` decompile reads `+0x53A5`, `+0x53A6`, and `+0x53A7`; it passes a derived force/copy argument into `SidebarClass::BlitToScreen(this, cVar4)`; after the blit it clears `+0x53A6` and `+0x53A7`. No `+0x53A8` read or write appears in the decompile.

Active in YR: Yes for the draw path; negative for `0x53A8` participation in draw invalidation.

### 3. `BlitToScreen` has exactly two `+0x53A8` check sites and both compare against zero.

Evidence: `SidebarClass::BlitToScreen @ 0x006A70E0` decompile and local disassembly:

- Fast no-dirty/current-rects-match path: `0x006A71AB: cmp byte ptr [edi + 0x53A8], bl`; if zero, it returns after clearing `DAT_00B0B518`; if nonzero, it copies only the top strip.
- Split dirty-copy path: `0x006A72A8: cmp byte ptr [edi + 0x53A8], bl`; if nonzero, it copies the top strip before lower-body copy.

`BL` is zero in this function from `0x006A70E9: xor ebx, ebx`.

Active in YR: The branches are live code inside an active YR function. The flag being nonzero is not proven live.

### 4. `BlitToScreen` clears `+0x53A8` after each top-strip copy and has no setter.

Evidence: local disassembly:

- Fast-path top-strip copy tail: `0x006A722F: mov byte ptr [edi + 0x53A8], bl`
- Split-copy top-strip tail: `0x006A7315: mov byte ptr [edi + 0x53A8], bl`

In both cases `BL == 0`. The function sets `DAT_00B0B519 = 1` after copied paths (`0x006A7481`) and clears `DAT_00B0B518` on exit (`0x006A748A`), but it never writes `1` to `+0x53A8`.

Active in YR: Yes for clearing after a hypothetical armed copy; no setter in this function.

### 5. The top-strip copy rectangle is fixed and sidebar-local.

Evidence: `BlitToScreen @ 0x006A70E0` decompile plus local disassembly around `0x006A71B7..0x006A722F` and `0x006A72B0..0x006A7315`.

The source rectangle is:

- source x `0`
- source y `0`
- width `g_SidebarTopClip` (`168` from prior layout reports)
- height `0x10`

The destination rectangle uses client-window origin plus the right-sidebar viewport offset when `DAT_00A8EB7C != 0`. In the split-copy path, the lower body copy still starts at source y `g_SidebarWidth` (`158` from prior layout reports), so the top-strip branch is specifically the y `0..15` strip, not the whole `SIDE1` body or radar region.

Active in YR: Conditional. The copy code is reachable if `+0x53A8` is nonzero, but no standard setter was found.

### 6. Whole `.text` direct-displacement scan found no direct sidebar setter to nonzero.

Evidence: read-only local Capstone pass over retail `gamemd.exe` `.text` with skipdata enabled found 27 real instructions containing memory displacement `0x53A8`. Only five are byte-sized `SidebarClass`-band references:

| Address | Instruction | Access | Size | Meaning |
|---|---|---:|---:|---|
| `0x006A4ECD` | `mov byte ptr [edi + 0x53A8], bl` | write | 1 | constructor clear |
| `0x006A71AB` | `cmp byte ptr [edi + 0x53A8], bl` | read | 1 | fast-path check |
| `0x006A722F` | `mov byte ptr [edi + 0x53A8], bl` | write | 1 | fast-path clear |
| `0x006A72A8` | `cmp byte ptr [edi + 0x53A8], bl` | read | 1 | split-path check |
| `0x006A7315` | `mov byte ptr [edi + 0x53A8], bl` | write | 1 | split-path clear |

All other direct `+0x53A8` references are 4-byte reads/writes at non-sidebar addresses such as `0x004F59A4`, `0x00508C3F`, and `0x00640460`. Those match known `HouseClass+0x53A8`/power accounting reports and are not this sidebar byte.

Active in YR: Negative for a direct static setter. No `mov byte ptr [reg + 0x53A8], 1`, no byte write from `AL`, and no byte write from a nonzero register was found for the sidebar field in the whole executable scan.

## Active in Standard YR?

Verdict: No standard active YR setter was found. The branch exists and should remain documented as a native conditional copy path, but `SidebarClass + 0x53A8` appears unarmed in stock standard YR from direct static evidence.

Proof chain:

- Constructor initializes `+0x53A8` to zero.
- `SidebarClass::Draw` does not set it.
- `SidebarClass::BlitToScreen` only compares it against zero and clears it after hypothetical top-strip copies.
- Whole `.text` direct-displacement disassembly found no byte setter to `1` or nonzero for this offset.
- The only nonzero writer pattern among direct `+0x53A8` references belongs to unrelated 4-byte class fields, not the sidebar byte.

Caveat: This is not a runtime watchpoint proof against every possible indirect memory write, raw object load, debugger mutation, or corrupted save state. It is a strong static proof that standard executable code has no direct setter for this field.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `SidebarClass + 0x53A8` starts zero and has no proven standard direct setter. | Constructor `0x006A4ECD`; whole `.text` direct scan found no byte setter. | Missing native dirty model; branch should not be actively armed by invented Rust events. | `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs`, future sidebar dirty/cache model. | Initialize top-strip-only pending flag to false; do not set it from tab/production/radar events unless a later report proves a setter. | `test_sidebar_top_strip_flag_starts_clear_and_no_standard_event_arms_it` | Do not map `0x53A7` tab repaint or credits text changes onto `0x53A8`. |
| If externally/nonstandard armed, `BlitToScreen` copies top strip `{0,0,g_SidebarTopClip,0x10}` and clears the flag. | `0x006A71AB..0x006A722F`, `0x006A72A8..0x006A7315`. | Unchecked; current Rust lacks retained surface dirty-copy modes. | Future sidebar surface cache/blit abstraction; `src/sidebar/mod.rs` constants split. | Preserve a dormant code path/test fixture for forced top-strip branch: source y `0`, height `16`, width `168`, then flag clears. | `test_sidebar_forced_top_strip_copy_uses_168_by_16_and_clears_flag` | Do not copy from y `158`, do not use the full sidebar height, and do not leave the flag set after copy. |
| `DAT_00B0B519` is set only after copied paths; no-top-strip/no-dirty fast path leaves it clear. | `0x006A71B1` zero branch returns to exit without `0x006A7481`; copied paths reach `0x006A7481`. | Missing native post-blit display flag. | `src/app_render/draw_passes.rs`, render scheduling. | When `0x53A8 == 0` and no dirty rect changed, no sidebar display notification occurs; when a forced top-strip test arms it, exactly one notification occurs. | `test_sidebar_top_strip_copy_sets_post_blit_once` | Do not use `DAT_00B0B519` as the arming flag; it is post-copy notification. |
| Direct `+0x53A8` references outside the sidebar byte are 4-byte class fields and must not be folded into this flag. | Capstone whole `.text` scan: non-`0x006A` references are size 4. | Documentation risk more than code risk. | Research docs and future implementation contracts. | Static verification artifact distinguishes sidebar byte from `HouseClass+0x53A8` power/drain fields. | `test_research_sidebar_53a8_not_house_power_drain_alias` | Do not grep `0x53A8` and assume every hit is the sidebar field. |

## Negative Facts / Do Not Do

- Do not claim `+0x53A8` is live in standard YR gameplay. The only proven sidebar writes clear it.
- Do not use `+0x53A8` as a synonym for `+0x53A6` or `+0x53A7`. `Draw @ 0x006A6C30` consumes and clears `0x53A6/0x53A7`; `BlitToScreen @ 0x006A70E0` only consumes/clears `0x53A8`.
- Do not treat non-sidebar `HouseClass+0x53A8` hits as evidence for this flag. Those are 4-byte reads/writes in different functions.
- Do not remove the top-strip branch from the parity model. It is real native code, even if unarmed by standard static paths.
- Do not invent a Rust event such as “tab changed,” “credits changed,” or “radar changed” that sets the top-strip-only flag without binary evidence.

## Remaining Uncertainty

- No live debugger watchpoint was run on `SidebarClass + 0x53A8`; a watchpoint across new skirmish, tab switching, radar open/close, resolution modes, save/load, and observer mode would be the runtime proof.
- This report does not exhaust every possible indirect write through raw object serialization, `memcpy`, or save/load restore. No direct setter exists in `.text`; indirect memory writes would require a separate serialization-focused investigation.
- If a nonstandard save or external patch creates `+0x53A8 = 1`, native `BlitToScreen` behavior is proven: one top-strip copy and then clear.

## Stale-Doc Replacement Wording

- `docs/research/SIDEBAR_BLIT_TO_SCREEN_DIRTY_RECTS_GHIDRA_REPORT.md`: replace “What specifically sets `this+0x53A8` is outside this scope” with: `Follow-up liveness pass found no standard active YR direct setter for SidebarClass+0x53A8. The field is initialized to zero in SidebarClass::Constructor and only checked/cleared by SidebarClass::BlitToScreen in direct static evidence. Keep the top-strip-only copy branch documented, but treat the arming flag as unproven/inactive in stock standard YR unless runtime watchpoint or serialization evidence later proves otherwise.`
- `docs/research/SIDEBAR_DIRTY_RECTS_REDRAW_FLICKER_PIXEL_CADENCE_GHIDRA_REPORT.md`: replace “a whole-program dataflow pass would be needed to prove the top-strip-only branch unreachable” with: `A whole-.text direct-displacement pass found no direct byte setter to nonzero for SidebarClass+0x53A8. This strongly narrows the remaining uncertainty to indirect writes such as serialization or runtime mutation; standard code should not be modeled as arming it.`
- `docs/research/SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md`: replace `0x53A8 | TopBarDirty | Top bar needs refresh` with: `0x53A8 | dormant top-strip-copy flag | initialized zero; checked/cleared only by BlitToScreen in direct static evidence; no standard setter found.`
- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md`: replace `+0x53A8 | NeedsTabRedraw | Blit pending flag` with: `+0x53A8 | 1-byte top-strip-copy pending flag | real BlitToScreen check/clear field; no standard direct setter found; do not conflate with +0x53A6/+0x53A7 tab/strip invalidation.`

## Status

COMPLETE for direct static liveness of `SidebarClass + 0x53A8` in stock `gamemd.exe`.

Sources:

- Ghidra MCP read-only decompile: `SidebarClass::Constructor @ 0x006A4EC0`, `SidebarClass::Draw @ 0x006A6C30`, `SidebarClass::BlitToScreen @ 0x006A70E0`.
- Ghidra MCP byte-pattern search: `A8 53 00 00`, returning 27 operand occurrences in `.text`.
- Local read-only Capstone disassembly of `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe` `.text`, filtering real memory operands with displacement `0x53A8`.
- Prior related reports: `SIDEBAR_BLIT_TO_SCREEN_DIRTY_RECTS_GHIDRA_REPORT.md`, `SIDEBAR_DIRTY_RECTS_REDRAW_FLICKER_PIXEL_CADENCE_GHIDRA_REPORT.md`, `SIDEBAR_INIT_LAYOUT_GLOBALS_EXACT_RECHECK_GHIDRA_REPORT.md`.
