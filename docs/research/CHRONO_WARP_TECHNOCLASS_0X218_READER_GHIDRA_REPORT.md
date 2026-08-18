# TechnoClass+0x218 (GhostCell) Reader Identification -- Ghidra Research Report

## Scope

Prior report `CHRONO_WARP_VISUAL_RENDERING.md` (Section 1) established TechnoClass+0x218
("GhostCell") is a CellClass pointer, zero-initialized in the constructor and written by a
trivial setter `TechnoClass__SetGhostCell` (0x0070C610) from 60+ call sites across ~25
functions -- but could not find any function that *reads* it, so its rendering role was
UNVERIFIED. This report locates the reader(s) and determines whether any of them drive
chrono-warp/temporal-erasure rendering (alpha, phase, draw-flag).

**Verdict: NEGATIVE RESULT, proven.** TechnoClass+0x218 has exactly one confirmed reader
site family, inside `BuildingClass__ExitObject_Main` (0x00443C60), and it is a gameplay
dispatch consumer (unit-exit / occupation bookkeeping), not a renderer. Every actual
rendering/visual-phase function in the TechnoClass draw pipeline was decompiled this
session and confirmed to NOT reference offset 0x218.

---

## 1. Confirmed reader: BuildingClass__ExitObject_Main (0x00443C60-0x004456A4)

Verified via `decompile_function 0x00443c60` (full-function decompile, this session).
`param_1` is the exiting `BuildingClass*` (`this`); `param_1[0x86]` is `param_1 + 0x86*4`
= `param_1 + 0x218` -- the same struct offset as TechnoClass's GhostCell field, since
BuildingClass inherits TechnoClass at offset 0 (no additional base-class insertion before
this field; the sibling reads `param_1[0x87]` = HouseClass owner-pointer-carrying field
and `param_1[0x148]` = TechnoTypeClass pointer are consistent with the rest of this report
family's verified TechnoClass field map).

Three distinct read sites of `param_1[0x86]` inside this one function:

1. **AI economy dispatch branch (mission-state 2 / HouseClass AI harvester exit path):**
   ```
   iVar5 = param_1[0x86];
   if (iVar5 == 0) { ... derive a fallback cell/coord via FUN_00703590 or MapClass__Get_CellClass ... }
   else {
     piStack_150 = (int *)0x1;
     uStack_154._0_2_ = (short)iVar5;
     uStack_154._2_2_ = (short)((uint)iVar5 >> 0x10);
     (**(code **)(*piVar7 + 0x480))();
     (**(code **)(*piVar7 + 0x1f0))(2);
   }
   ```
   The stored value is split into two 16-bit halves and passed (with flag `1`) to a virtual
   call at vtable+0x480 on the exiting unit, immediately followed by a vtable+0x1f0 call
   with action code `2`.

2. **Cache-before-overwrite pattern (normal player exit path), two occurrences:**
   ```
   uStack_144 = (int **)param_1[0x86];
   TechnoClass__SetGhostCell();
   ```
   The old GhostCell value is read into a local before `SetGhostCell` is called again to
   overwrite it -- a save/restore idiom, not a render consumption.

3. **End-of-function occupation dispatch:**
   ```
   if (param_1[0x86] != 0) {
     uStack_154._0_2_ = 1; uStack_154._2_2_ = 0;
     (**(code **)(*param_2 + 0x480))(param_1[0x86],1);
     (**(code **)(*param_2 + 0x1e8))(2,0);
   }
   ```
   Again: GhostCell value forwarded to vtable+0x480 with flag `1`, then a vtable+0x1e8 call
   with action code `(2,0)`.

**Pattern conclusion:** every read of +0x218 in this function feeds a virtual dispatch call
at offset **+0x480** (paired immediately with a mission/action-code call at +0x1e8 or
+0x1f0, both of which take small integer codes like `2` elsewhere in this same function --
consistent with `MissionClass`-style state/action dispatch used throughout
`ExitObject_Main`). No screen coordinates, draw-flag constants, or blitter/CC_Draw_Shape
calls are involved at any of these three sites.

**Confidence: HIGH** that these are the only reads inside this function and that none feed
rendering (full-function decompile inspected end-to-end). **UNVERIFIED** the exact identity
of the vtable+0x480 target function this session -- `get_xrefs_to 0x006f6ca0`
(TechnoClass::Unlimbo) shows it is installed via vtable data at 0x007f4a38, but that data
address was not cross-checked against the specific vtable+0x480 slot index this session, so
the "Unlimbo-like" identity is a plausible-but-unconfirmed inference from the call
signature (packed coord + int flag), not a proven binding.

---

## 2. False-positive trap: raw byte-pattern search collides with RulesClass+0x218

A brute-force `search_byte_patterns "8B 81 18 02 00 00"` (mov eax,[ecx+0x218], ECX-based
thiscall read) returned exactly two hits: 0x007196E3 and 0x00719A44. Both resolved via
`get_function_by_address` to `TeleportLocomotionClass__InitiateWarp` (0x00719400) and
`TeleportLocomotionClass__StateMachineTick` (0x007192F0) respectively. Decompiling both
(`decompile_function 0x00719400`, `decompile_function 0x007192f0`) showed these are
`*(int *)(g_RulesClass_Instance + 0x218)` -- **RulesClass+0x218, the ChronoInSound global
fallback** already documented in the prior report's Section 6 ("Rules+0x218 global
fallback"), reached by first loading the `g_RulesClass_Instance` pointer into ECX and then
applying `+0x218`. This is NOT a TechnoClass+0x218 read. It is flagged here explicitly per
the label-adversarial mandate: **the same displacement literal (0x218) collides across two
unrelated structs (TechnoClass and RulesClass) whenever the base pointer is loaded from a
global into a register first** -- raw displacement search cannot distinguish them; only
tracing the base-register's origin (decompile, not disassembly alone) resolves it.

---

## 3. Rendering pipeline swept clean -- no reader found in any draw/visual-phase function

Every function plausibly in the chrono-warp/temporal-erasure visual path was decompiled
this session and checked line-by-line for any `param_1[0x86]` / `+0x218` access. None found:

| Function | Address | Verified via | 0x218 read? |
|----------|---------|---------------|-------------|
| `TechnoClass__Draw` | 0x00706640 | `decompile_function` | No |
| `TechnoClass__Render` | 0x00706ED0 | `decompile_function` | No |
| `TechnoClass_DrawSHP` | 0x00705E00 | `decompile_function` | No |
| `TechnoClass__DrawExtras` | 0x006F5190 | `decompile_function` | No |
| `TechnoClass_GetVisualState` | 0x00703860 | `decompile_function` | No |
| `FootClass__GetVisualState` | 0x004DA4E0 | `decompile_function` | No |
| `TechnoClass__ModifyCloakDrawFlags` | 0x0070ED80 | `decompile_function` | No |
| `TechnoClass__ScaleByTemporalVisualPhase` | 0x0070E380 | `decompile_function` (newly decompiled this session; not in prior report) | No -- reads +0x198/+0x1A0/+0x1A4 only |
| `TechnoClass__ScaleByWarpInVisualPhase` | 0x0070E4B0 | prior report + field map cross-check | No -- reads +0x1B4/+0x1BC/+0x1C0 only |
| `TechnoClass__UpdateGapVisual` | 0x0070E920 | `decompile_function` | No -- reads/writes `param_1[0x6d..0x70]` = +0x1B4..+0x1C0, confirms it is the writer feeding `ScaleByWarpInVisualPhase` |

`TechnoClass__ScaleByTemporalVisualPhase` (0x0070E380) is a previously-undocumented
sibling of `ScaleByWarpInVisualPhase` -- same phase-scaling shape (`switch` on a phase
field producing a 0-2000 alpha-like scale), reading +0x198 (StartFrame) / +0x1A0
(Duration) / +0x1A4 (Phase), i.e. the same fields `UpdateTemporalVisual` (0x0070E5A0)
drives. Both `TechnoClass__Draw` and `TechnoClass_DrawSHP` call
`TechnoClass__ScaleByTemporalVisualPhase` immediately followed by
`TechnoClass__ScaleByWarpInVisualPhase` to compute the final alpha/scale value passed into
`Blitter_selector`/`CC_Draw_Shape` -- confirming (again) that the two verified visual-phase
field groups (+0x198/+0x19C/+0x1A0/+0x1A4 and +0x1B4/+0x1BC/+0x1C0) are the actual
rendering-facing state, and +0x218 is absent from that chain entirely.

---

## 4. No dedicated accessor exists

`search_functions name_pattern="TechnoClass"` (full listing, 130 results) shows
`TechnoClass__SetGhostCell` but no `GetGhostCell`/`IsGhostCell`-style counterpart anywhere
in the TechnoClass method family. This confirms reads of +0x218 only ever happen via
inline pointer dereference (`param_1[0x86]`), and the only such dereference site located
in the binary this session is inside `BuildingClass__ExitObject_Main`.

---

## 5. Active in YR

**Yes**, `BuildingClass__ExitObject_Main` fires on every unit that exits/spawns from a
building in a standard YR skirmish (war factory tank production, barracks infantry, refinery
harvester rally, etc.) -- routine, high-frequency, not gated behind any TS-legacy or
SpecialFlags check found in this function body.

**But this activity is unrelated to chrono-warp/temporal-erasure rendering specifically.**
GhostCell is a general building-exit bookkeeping field (rally/dispatch cell cache) that
happens to also get cleared (`SetGhostCell(0)`) inside
`TeleportLocomotionClass__StateMachineTick` phase 7 (already documented in the prior
report) as routine "clear stale exit-preview state" cleanup when a chrono warp completes --
that write is not a read, and no code path was found that reads the field back for warp
visual purposes.

---

## Implementation Handoff

**Does not feed rendering -- proof, not a positive handoff.** TechnoClass+0x218 (GhostCell)
should NOT be modeled as any part of the Rust chrono-warp/temporal-erasure visual pipeline
(alpha, phase, draw-flag, blit selection). Its only verified behavioral role is as a
building-exit dispatch cache: read back inside `BuildingClass::ExitObject_Main`-equivalent
logic to (a) restore/pass a previously-computed exit/rally cell into an occupation or
movement dispatch call, and (b) save/restore its old value around a `SetGhostCell`
overwrite. If/when the Rust engine implements the building-exit ghost-cell mechanism, model
it as part of that dispatch logic, not the render layer -- but that implementation is out of
this report's scope (only the reader identity + rendering relevance was requested).

## Negative Facts / Do Not Do

- Do NOT wire TechnoClass+0x218 into any Rust rendering code path (alpha blend, warp-phase
  scale, draw-flag selection). Proven absent from `Draw`, `Render`, `DrawSHP`, `DrawExtras`,
  `GetVisualState` (both TechnoClass and FootClass overrides), `ModifyCloakDrawFlags`,
  `ScaleByTemporalVisualPhase`, and `ScaleByWarpInVisualPhase`.
- Do NOT reuse the byte pattern `8B 81 18 02 00 00` (or any bare "18 02 00 00" search) as
  evidence of a TechnoClass+0x218 access without decompiling the containing function first
  -- it collides with RulesClass+0x218 (see Section 2).
- Do NOT treat "UpdateTemporalVisual" (0x0070E5A0) or "ScaleByWarpInVisualPhase"
  (0x0070E4B0) as related to GhostCell -- they operate on entirely separate field groups
  (+0x198/+0x19C/+0x1A0/+0x1A4 and +0x1B4/+0x1BC/+0x1C0 respectively), reconfirmed this
  session.
- Do NOT assume the old doc's UNVERIFIED "deploy-preview ghost rendering" narrative (Section
  1, point 4 of the prior report) is correct -- it is now refuted for the *rendering* half;
  the "occupation call" half of that old guess is directionally supported (vtable+0x480
  receives the cell/coord), but as gameplay dispatch, not a drawn ghost overlay.
- Do NOT claim the vtable+0x480 target's exact name/address as verified -- it is an
  inference from call signature, not a confirmed binding this session.

## Remaining Uncertainty

- Exact identity/address of the function at vtable+0x480 (the actual consumer of the
  GhostCell value read in `ExitObject_Main`) is UNVERIFIED. Plausible candidate
  `TechnoClass::Unlimbo` (0x006F6CA0) was located but its vtable slot index was not
  confirmed against +0x480 this session.
- Whether any OTHER function elsewhere in the ~1300-function TechnoClass-adjacent surface
  reads +0x218 outside `BuildingClass__ExitObject_Main` was not exhaustively proven (a
  full-binary disassembly grep for every possible base-register encoding of the +0x218
  displacement was not computationally feasible in this session's scope) -- but every
  plausible *rendering* candidate function was checked exhaustively (Section 3), which was
  the task's actual question.

## Stale-doc replacement wording

In `CHRONO_WARP_VISUAL_RENDERING.md`, Section 1, point 4 and the "Conclusion" paragraph
currently read (in relevant part):

> OLD: "*(Not re-verified this session -- its source, "TacticalClass__DrawObjects,"
> does not exist in the binary... Treat this point as UNVERIFIED, not disproven, until the
> true reader of +0x218 is located.)*" ... "Its specific use as a "building deployment
> ghost rendering" field is UNVERIFIED as of 2026-07-12."

> NEW: "The true reader of +0x218 has been located (see
> `CHRONO_WARP_TECHNOCLASS_0X218_READER_GHIDRA_REPORT.md`): `BuildingClass__ExitObject_Main`
> (0x00443C60), which reads `param_1[0x86]` at three sites and forwards the value to a
> vtable+0x480 dispatch call (occupation/movement bookkeeping for the exiting unit), never
> to a draw/blit call. CONFIRMED: +0x218 does NOT feed rendering -- it is a building-exit
> dispatch cache field, unrelated to chrono-warp/temporal-erasure visuals."

---

**Status: COMPLETE** (negative result, proven for the rendering question; reader identity
proven for BuildingClass__ExitObject_Main; exact vtable+0x480 callee identity left
UNVERIFIED as noted above).
