# SOVIET_RADAR_LEFT_PANEL_SHP_SELECTORS_GHIDRA_REPORT

Status: FAILED

## Target Question

Re-check the Soviet branches in `FUN_0072D460`, `FUN_0072D830`, `FUN_0072FA10`, and direct layout/draw consumers only as needed to prove filename selection for side `1` and `640` vs `800+` branches.

## Non-goals

- Do not inspect unrelated sidebar systems.
- Do not modify Rust, INI, or published research docs outside this report.
- Do not substitute raw binary disassembly, local docs, or inference when Ghidra MCP is unavailable.

## Evidence Needed To Mark COMPLETE

- Read-only Ghidra decompile plus assembly/disassembly evidence for each selector function.
- Filename string source plus selector address for every Soviet-relevant SHP/minimap branch.
- Caller/xref evidence proving whether each branch is active in standard Yuri's Revenge.
- A Rust-facing implementation handoff with concrete test-name proposals.

## Stop Conditions

- Stop immediately if Ghidra MCP read-only access is unavailable.
- Stop if function boundaries are missing and cannot be inspected read-only.
- Stop before any mutating Ghidra operation.

## Investigation Result

Ghidra MCP read-only tools were not available in this session. Tool discovery exposed GitHub, Node REPL, and Codegraph tools, but no Ghidra decompile, disassembly, xref, or function-inspection tools. Per the swarm prompt, this slot must return `FAILED` and must not substitute raw binary disassembly or existing research-doc claims.

## Verified Binary Findings

None. No binary findings were added because Ghidra MCP was unavailable.

## Active In YR

Unchecked. No material side-branch or filename-selection claim can be marked active or inactive without the required Ghidra evidence.

## Implementation Handoff

None. The required evidence threshold was not met.

## Negative Facts / Do Not Do

- Do not conclude whether Soviet uses `SS*`, `SY*`, or generic radar/left-panel assets from this failed slot; no Ghidra evidence was collected.
- Do not change `src/render/sidebar_chrome.rs` based on this report; it contains no verified selector facts.
- Do not downgrade the existing `SIDEBAR_RADAR_POSITIONING` filename table from this report; no contradiction was verified.

## Remaining Uncertainty

- Whether `FUN_0072D460`, `FUN_0072D830`, and `FUN_0072FA10` select `SS*`, `SY*`, or generic SHP names for side `1`.
- The exact `640` versus `800+` branch predicates and whether they are inclusive/exclusive.
- Which branches are active in standard YR for Soviet and Yuri live side values.
- Whether current Rust's generic `radar.shp` path represents a different sidebar layer from native explicit `SSCR*` / `MPSSCR*` assets.

## Stale Doc Wording

None. No stale-doc replacement wording can be proposed without the required Ghidra evidence.
