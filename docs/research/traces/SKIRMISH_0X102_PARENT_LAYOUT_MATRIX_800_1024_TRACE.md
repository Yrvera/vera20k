# Skirmish 0x102 Parent Layout Matrix 800/1024 Trace

**Scenario:** Fresh standard offline YR Skirmish parent dialog `0x102`, compare current Rust `SkirmishShellLayout` positions at `800x600` and `1024x768`.
**Scope:** Parent/child/control rectangles only: no paint internals, modal `0x6B`, dropdown popups, text colors, preview decode, or start markers.
**Verdict:** FAIL at `1024x768` for ordinary/fallback controls because the active Rust app path globally translates the fixed `800x600` layout by `(112,84)`. gamemd keeps those ordinary child HWND rects at their resource/fixup coordinates and only moves the allowlisted right-panel/status/button controls.

## Evidence Used

- gamemd active path is standard YR, not TS legacy: `FUN_006AE2C0` creates/pumps dialog `0x102`; `FUN_00622B50`/`FUN_0060C4A0` resize parent and enumerate children through `ResizeShellChildControl_0060C0C0`. Source: `docs/research/skirmish-ui/SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md:8`.
- gamemd `0x102` matrix is verified from RT_DIALOG extraction plus resize/helper/fixup code; the child matrix is not globally scaled. Source: `docs/research/skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md:7`, `:19`, `:49`.
- gamemd one-pixel fixups are verified: `0x694 y+1`, `0x50C y-1`, `0x54E/0x693/0x696/0x69A x-1`, `0x6A0 x+1,w+1`. Source: `docs/research/skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md:34`, `:156`.
- Active Rust path: app input/layout and skirmish renderer call `compute_fixed_800_layout`. Source: `src/app.rs:484`, `src/app_skirmish_shell_render.rs:390`.
- Current Rust helper behavior: `compute_fixed_800_layout(1024,768)` translates `compute_layout(800,600)` by `(112,84)`. Source: `src/ui/skirmish_shell/layout.rs:314`, `:316`; focused test confirms current expected values at `:895..905`.
- Verification run: `cargo test -q fixed_800_layout_centers_native_shell_without_rescaling` passed.

## Pipeline

1. Trigger: Skirmish shell active in main-menu state.
2. Rust layout construction: `App::skirmish_shell_layout` and renderer call `compute_fixed_800_layout(render_width, render_height)`.
3. Rust transform: `compute_fixed_800_layout` computes centered fixed-shell offset and calls `translate_layout`, which translates every represented child field.
4. gamemd layout construction: dialog `0x102` parent is full-screen; child HWNDs are enumerated by `ResizeShellChildControl_0060C0C0`.
5. gamemd transform: Start/Choose/Back/right-panel/status move by specific helpers; ordinary children preserve resource/fixup rects.
6. Screen result: at `800x600`, Rust offset is `(0,0)` and represented rects match. At `1024x768`, many ordinary visible controls are drawn and hit-tested 112 px right and 84 px down.

## Stage Verdicts

| Stage | Rust output | gamemd output | Verdict |
|---|---:|---:|---|
| Active path selection at `1024x768` | fixed-shell `layout.screen=(112,84,800,600)` | parent window resized to full screen; child policy is per-control | FAIL |
| `800x600` right-panel buttons/preview | Start `(644,241,156,42)`, Choose `(644,283,156,42)`, Preview `(644,37,144,112)`, Back `(644,535,156,42)` | same matrix values | PASS |
| `800x600` right-panel text/status | title `(635,3,162,16)`, game type `(649,167,135,16)`, map label `(649,189,135,33)`, status `(10,579,615,20)` | same matrix values | PASS |
| `800x600` player-name/fixup controls | `0x6A0=(58,59,151,23)`, `0x50C=(404,340,128,21)`, first four checkbox x=`71`, BuildOffAlly x=`302` | same matrix values | PASS |
| `800x600` row/flag/color/start/team controls | resource/fixup positions, no shell-wide offset | same matrix values | PASS |
| `800x600` trackbar labels/checkboxes | labels `(302,286/314/341,90,16)`, checkbox rects per matrix | same matrix values | PASS |
| `1024x768` right-panel/chrome anchor set | right-panel top `(744,84,168,199)`, title `(747,87,162,16)`, game type `(761,251,135,16)`, map label `(761,273,135,33)` | same matrix values | PASS |
| `1024x768` Start/Choose/Preview/Back | Start `(756,325,156,42)`, Choose `(756,367,156,42)`, Preview `(756,121,144,112)`, Back `(756,619,156,42)` | same matrix values | PASS |
| `1024x768` status/help `0x695` | `(122,663,615,20)` | `(122,663,615,20)` | PASS |
| `1024x768` ordinary/fallback controls | translated by `(112,84)` | preserved at resource/fixup positions | FAIL |

## 1024x768 Drift Ledger

All entries below are active Rust `compute_fixed_800_layout(1024,768)` versus gamemd final matrix. Width/height match unless noted; every failing entry has `dx=+112`, `dy=+84`.

| Control(s) | Rust rect(s) | gamemd rect(s) | Verdict |
|---|---:|---:|---|
| parent/layout basis | `(112,84,800,600)` | full-screen parent and per-child policy for `(1024,768)` | FAIL |
| `0x6A0` player name | `(170,143,151,23)` | `(58,59,151,23)` | FAIL |
| column statics `0x796/0x791/0x792/0x793/0x794` | `(171,118)`, `(399,118)`, `(537,118)`, `(600,118)`, `(657,118)` | `(59,34)`, `(287,34)`, `(425,34)`, `(488,34)`, `(545,34)` | FAIL |
| AI type combos `0x50B/0x50E/0x516/0x51A/0x51B/0x51C/0x51D` | x=`171`, y=`169,195,221,247,273,299,325` | x=`59`, y=`85,111,137,163,189,215,241` | FAIL |
| side combos `0x6A1/0x510/0x513/0x51E/0x514/0x51F/0x520/0x521` | x=`399`, y=`143,169,195,221,247,273,299,325` | x=`287`, y=`59,85,111,137,163,189,215,241` | FAIL |
| color combos `0x6A2/0x522..0x528` | x=`535`, y=`143,169,195,221,247,273,299,325` | x=`423`, y=`59,85,111,137,163,189,215,241` | FAIL |
| flags `0x6DA..0x6E1` | x=`337`, y=`143,169,195,221,247,273,299,325` | x=`225`, y=`59,85,111,137,163,189,215,241` | FAIL |
| start-position combos `0x6A3..0x6AB` | x=`598`, y=`143,169,195,221,247,273,299,325` | x=`486`, y=`59,85,111,137,163,189,215,241` | FAIL |
| team combos `0x76D..0x774` | x=`658`, y=`143,169,195,221,247,273,299,325` | x=`546`, y=`59,85,111,137,163,189,215,241` | FAIL |
| trackbar labels `0x699/0x69B/0x69C` | `(414,370,90,16)`, `(414,398,90,16)`, `(414,425,90,16)` | `(302,286,90,16)`, `(302,314,90,16)`, `(302,341,90,16)` | FAIL |
| trackbars `0x529/0x511/0x50C` | `(516,370,128,21)`, `(516,398,128,21)`, `(516,424,128,21)` | `(404,286,128,21)`, `(404,314,128,21)`, `(404,340,128,21)` | FAIL |
| checkboxes `0x54E/0x693/0x696/0x69A/0x69D` | `(183,370,150,16)`, `(183,398,150,16)`, `(183,425,150,16)`, `(183,455,155,16)`, `(414,453,249,18)` | `(71,286,150,16)`, `(71,314,150,16)`, `(71,341,150,16)`, `(71,371,155,16)`, `(302,369,249,18)` | FAIL |

## One-Pixel Fixup Check

The known one-pixel fixups are correctly present in `compute_layout(800,600)`: player name `x+1,w+1`, unit count `y-1`, and the first four checkbox `x-1` fixups. They are also internally present before `compute_fixed_800_layout` translates the entire layout. The `1024x768` failures are therefore not missing 1-pixel fixups; they are larger screen-position drift caused by applying the fixed-shell translation to controls that gamemd leaves unshifted.

## Adjacent Findings

- `compute_layout(1024,768)` has tests that match the gamemd selective high-res matrix for several controls, but this helper is not the active app/render path for the skirmish shell. The active path uses `compute_fixed_800_layout`, so those tests can hide the real `1024x768` parity failure.
- `translate_layout` also shifts `layout.screen` to `(112,84,800,600)`. gamemd’s parent dialog is full-screen before child enumeration; the shell background policy may still need a separate visual trace, but this slot only traces child/control layout.

## Tally

PASS: 8 | FAIL: 10 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

