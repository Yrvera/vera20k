# SOVIET_SIDEBAR_TEXT_COLOR_READY_STATUS Ghidra Report

Status: FAILED

## Target Question

Verify `SetSidebarTextColor` / `FUN_0072F440` and the relevant `StripClass::Draw`
Ready/status text consumer path for Soviet in-game sidebar text color, Ready/status
color use, and whether current Rust side-highlight assumptions match stock
`gamemd.exe`.

## Non-goals

- Do not re-investigate full sidebar text rendering beyond color/use proof.
- Do not inspect or alter Rust implementation.
- Do not substitute raw binary disassembly for the required Ghidra MCP evidence.
- Do not modify Ghidra state.

## Evidence Needed To Mark COMPLETE

- Read-only Ghidra decompile and assembly/disassembly evidence for
  `FUN_0072F440` branch predicates and color literal/conversion inputs.
- Read-only Ghidra xref/caller evidence from the relevant `StripClass::Draw`
  Ready/status text path.
- A Rust-facing handoff that distinguishes stock text color from any fade/highlight
  or side chrome color usage.

## Stop Conditions

- Ghidra MCP read-only access is unavailable.
- The target expands into unrelated sidebar rendering or palette research.
- Any necessary evidence would require mutating Ghidra state.

## Verified Binary Findings

None. Ghidra MCP was unavailable in this session, so no target evidence was
collected.

## Implementation Handoff

None. The required evidence gate was not met.

## Negative Facts / Do Not Do

- Do not treat this report as proof that Rust `side_highlight_color` is either
  correct or incorrect; no fresh Ghidra evidence was collected.
- Do not replace the existing sidebar Ready/status color wording from this report;
  stale-doc wording remains unverified until Ghidra can be queried.
- Do not use raw local disassembly as a substitute for this slot; the dispatch
  explicitly required Ghidra MCP.

## Remaining Uncertainty

- Exact `FUN_0072F440` branch predicates remain unverified.
- Exact color byte/order and conversion inputs remain unverified.
- Ready/status text consumer address relationship remains unverified.
- Rust `side_highlight_color` semantic role remains unverified.

## Stale-doc Replacement Wording

None. No replacement wording is justified without Ghidra evidence.
