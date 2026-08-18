# Skirmish >800 Parent Background Targeted Trace Reconciliation

Date: 2026-05-22

**Investigation Mode:** targeted verification / doc reconciliation  
**Claimed Scope:** standard offline YR Skirmish dialog `0x102`, fresh entry at screen widths greater than 800, parent-background SHP selection/draw behavior only.  
**Non-Scope:** full retail screenshot capture, complete high-res UI pixel comparison, online/WOL shells, right-panel/control placement beyond the parent-background decision.  
**Confidence:** High for static binary behavior and current Rust policy; Medium for full screenshot parity because no live retail capture was produced in this pass.  
**Active in YR:** Yes, conditional on high-resolution video mode.

## 1. Result

The previously-open "wide-screen `>800` shell background" item is resolved for the parent-background SHP decision in the normal fresh Skirmish lifecycle.

Fresh `>800` Skirmish does not draw `MnScrnLCoopGameSetup.shp` as a fallback parent background. The loader only populates `DAT_00B0FA18` at exact width `800`; cleanup clears that pointer after normal Skirmish exit; `Background_Overlay` selects the alternate pointer for non-640 widths; and `CC_Draw_Shape` treats a null SHP pointer as an early no-op.

The remaining screenshot trace should be scoped to full high-res composition parity, not to deciding whether Rust should reuse/stretch/draw the 800 parent background above 800. It should not block the specific Rust policy of skipping the parent-background sprite at `>800`.

## 2. Evidence Spot Checks

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| Skirmish alternate background SHP is loaded only at exact width 800. | `0x0072CF49` compares `g_ScreenWidth` with `0x320`; `0x0072CF53` skips the SHP load when not equal; `0x0072CF65` writes `DAT_00B0FA18` only on the exact-width path. | High | Yes |
| The palette/convert load still runs after the skipped `>800` SHP branch. | `0x0072CF6A..0x0072CF7F` loads `MnScrnLCoopGameSetup.PAL`/convert state and sets guard `DAT_00B0FCD9`. | High | Yes |
| Normal cleanup clears stale alternate background state. | `0x0072CF90..0x0072D001`; `0x0072CFCB` writes `DAT_00B0FA18 = 0`; `0x0072CFFA` clears the loader guard. | High | Yes |
| `Background_Overlay` selects the alternate pointer for every non-640 width. | `0x0072E7AD` compares width with `0x280`; the non-equal path pushes the alternate pointer and calls `CC_Draw_Shape` at `0x0072E815`. | High | Yes |
| A null SHP pointer is a no-op, not a crash or fallback image. | `CC_Draw_Shape @ 0x004AED70`; `0x004AED84..0x004AED8E` tests `EDI` and jumps to return when null; `0x004AEDAB..0x004AEDAD` repeats the lazy-load null gate. | High | Yes |

Primary prior report: `SKIRMISH_GT800_BACKGROUND_POINTER_LIFECYCLE_GHIDRA_REPORT.md`.  
Supporting prior report: `SKIRMISH_HIGH_RES_SHELL_HOSTING_AND_GT800_BACKGROUND_GHIDRA_REPORT.md`.

## 3. Current Rust Status

Current Rust already matches this specific parent-background policy:

| Rust surface | Observed behavior | Status |
|---|---|---|
| `src/app_skirmish_shell_render.rs::parent_background_role` | returns `Mnscrns640` at width 640, `CoopGameSetup800` at width 800, and `None` at width 1024/greater than 800. | matches |
| `src/app_skirmish_shell_render.rs::parent_background_role_uses_only_verified_widths` | asserts `compute_layout(1024, 768)` has no parent-background role. | matches |
| `src/render/skirmish_shell_chrome.rs` | loads both verified parent-background assets, but runtime selection is controlled by `parent_background_role`. | acceptable |

No Rust patch is required for the `>800` parent-background SHP decision.

## 4. Remaining Screenshot Trace

A retail screenshot trace can still be useful, but its question should be narrower:

| Remaining trace question | Status | What it would prove |
|---|---|---|
| Full 1024x768 visual composition | deferred | Confirms aggregate chrome/control/background pixels, including right-panel origin, clipping edges, and any non-background artifacts. |
| Abnormal stale `DAT_00B0FA18` history | deferred | Requires runtime debugger/watchpoint; static standard lifecycle clears the pointer. |

Do not treat either deferred item as a reason to draw or stretch `MnScrnLCoopGameSetup.shp` at widths above 800.

## 5. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Fresh `>800` Skirmish does not emit a parent-background SHP draw. | `0x0072CF49..0x0072CF65`, `0x0072CFCB`, `0x004AED84..0x004AED8E` | none observed | `src/app_skirmish_shell_render.rs::parent_background_role` | Keep returning `None` above 800. | 1024x768 dev Skirmish draw order contains no `ParentBackgroundMnscrns640` and no `ParentBackgroundCoopGameSetup800`. | Do not stretch, tile, or reuse the 800 background for convenience. |
| Exact 800 still draws `MnScrnLCoopGameSetup.shp`. | `0x0072CF49..0x0072CF65`; non-640 background path at `0x0072E7EA..0x0072E815` | none observed | `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render.rs` | Keep 800 as a separate exact-width case. | 800x600 dev Skirmish can emit `ParentBackgroundCoopGameSetup800`. | Do not collapse 800 and `>800` into one "large shell" branch. |
| Normal cleanup prevents 800 background pointer leakage into a later fresh `>800` entry. | `0x0072CF90..0x0072D001` | no extra Rust lifecycle state needed while role is derived from width | future shell lifecycle/cache surfaces | Any future cached gamemd-like background pointer must reset on shell exit. | Enter/exit 800 shell, then enter 1024 shell: no parent-background SHP role. | Do not let an asset cache become semantic state. |

## 6. Synthesis Update Recommendation

Replace broad unresolved wording with:

> `>800` parent-background SHP behavior is resolved for the normal fresh Skirmish lifecycle: no alternate background SHP is loaded, and a null alternate pointer makes `CC_Draw_Shape` no-op. Full high-res screenshot comparison remains deferred for aggregate composition parity, not for the parent-background draw decision.

## Sources

- `docs/research/skirmish-ui/SKIRMISH_GT800_BACKGROUND_POINTER_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_HIGH_RES_SHELL_HOSTING_AND_GT800_BACKGROUND_GHIDRA_REPORT.md`
- Ghidra disassembly spot checks: `0x0072CF40`, `0x0072CF90`, `0x0072E730`, `0x004AED70`
- Rust scan: `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`
