# Bridge Parity — Two Architecture Decisions (pre-fix)

**Date:** 2026-05-29
**Purpose:** lock two policies BEFORE the fix phase so the ~25 in-place symptom fixes from
`BRIDGE_PARITY_IMPLEMENTATION_CONTRACT.md` are consistent applications of a rule, not 25
independent re-litigations. Bridge subsystem only — neither decision touches a non-bridge system.

---

## Decision A — `overlay_byte` is the single source of truth

**Rule.** `BridgeRuntimeCell.overlay_byte` (mirror of gamemd `Cell+0x44`) is authoritative for
per-cell bridge damage/render state. `DamageState` and `bridgehead_anchor_class` become **derived
views** computed from the byte (+ tile class), never written independently of it. On any conflict,
the byte wins.

**What changes.** Every state writer updates the byte and lets the enum follow:
- body-SM collapse writes `overlay_byte = 0xFF` (−1) — fixes BR-09 (collapsed cell stays walkable/intact).
- `update_ramp_perpendicular` writes the `SetOverlayAndPropagate`/`ToggleBridgePavement` byte — fixes BR-15.
- bridgehead absorb/collapse writes the slot byte (+2 absorb, +3 collapse) — fixes BR-12/20.
- `effective_render_state` derives from the byte with no enum fallback that can disagree.

**Invariant (enforced by test/debug-assert).** After every mutation,
`DamageState::from_overlay(cell.overlay_byte)` agrees with the cell's reported state — or the enum
is computed on read so it cannot drift.

**What this is NOT.** We do not delete the enum (it stays a typed read-through) and we do not remove
`BridgeCellRole` (it stays an internal index). The change is: the byte wins, and role/enum may not
*gate* behavior that gamemd gates on the byte/flags (fixes BR-11 walker role-skip, BR-10 rim predicate).

---

## Decision B — the bridge dispatcher is draw-faithful

**Rule.** Bridge damage / debris / repair code consumes RNG draws in gamemd's exact **count, order,
and stream**. The draw schedule is the spec; the spawn/outcome is downstream. No Rust short-circuit
(`break`, `is_empty()` early-return, `&&` gate, first-match-wins) may skip a draw the binary makes or
add one it doesn't.

**What changes.**
- model `ApplyDamageToCell`'s 4-block fall-through — each matching block rolls its own
  `R(1,BridgeStrength)`, no early-out (BR-01); the duplicate Block C/D overlay gate (BR-02).
- exact gate constants: outer `2_040_109_464`, metallic `0x3FFF_FFFF` (BR-03/04).
- metallic slot draw is unconditional once gate+alloc pass (BR-05).
- jitter = `round((draw·scale−0.5)·50)` (BR-21).

**Guard (the gate that keeps it fixed).** A `world_hash` regression test pins the draw schedule for a
scripted collapse + repair scenario. Any future edit that shifts a bridge draw fails the test. This
is the single most important artifact of the whole fix phase.

**What this is NOT.** We do not modify `SimRng` or any non-bridge RNG. The separate-stream question
(BR-07: repair variant possibly drawing from `g_MapGenRng`) stays **deferred** until a live
`disassemble_function 0x00598030` confirms the stream — implementing it on a guess could pollute the
main stream.

---

## Scope guards
- Both decisions are bridge-subsystem-local. No cross-system refactor, no ECS change, no RNG-core change.
- Decisions do not, by themselves, change behavior — they constrain HOW the contract's fixes are written.

## Sequencing implication
- **Unblocked now (no Ghidra needed):** the debris RNG holes (BR-03/04/05/21/22) are exact + self-contained;
  the dispatcher fall-through *structure* (BR-01/02) is correct independent of routing.
- **Blocked on Ghidra reconnect / BSS constants:** the routing predicate (BR-19, decides *which* blocks match),
  the Z-window lepton bounds (BR-17), and repair RNG bits/stream (BR-06/07). Full draw-faithfulness of BR-01/02
  reaches bit-exact only once BR-19's matcher is correct — until then the structure is right but the *matched set*
  still uses the `deck_level>=4` proxy.
