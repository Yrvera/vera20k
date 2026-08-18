# RepairBridgeWalker `+0x11E` Reset — Follow-Up

**Trigger:** [REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md](REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md)
§7 Open Question 1 hypothesized the engineer-repair `+0x11E` Destroyed→Healthy
transition fires indirectly through `CellClass::RecalcAttributes @ 0x0047D2B0`
under `SlopeIndex != 0 && OverlayTypeClass[+0x2a9] != 0`.

**Verdict:** **Hypothesis refuted.** `OverlayTypeClass + 0x2a9` is the
`Tiberium=` INI flag, not a bridge-related flag. Bridge overlays do not set
`Tiberium=yes`, so neither of the two `field_0x11e = 0` sites in
`RecalcAttributes` fires on engineer repair.

## Evidence

`OverlayTypeClass::ReadINI @ 0x005FE7A0` (decompiled this session, read-only):

```
CCINIClass::ReadBool(iVar1, s_Tiberium_00817278, *(this + 0x2a9))
→ *(this + 0x2a9) = result
```

The string at `0x00817278` is `"Tiberium"`. The `+0x2a9` byte is set by
`Tiberium=yes` in the overlay's INI section. Standard YR bridge overlays
(`[0x4A..0x65]` low, `[0xCD..0xE8]` high) are not Tiberium.

`RecalcAttributes @ 0x0047D2B0` has **two** `field_0x11e = 0` writes:

1. The conditional at the top of the function:
   ```c
   if ((this->SlopeIndex != 0) && (*(char *)(iVar9 + 0x2a9) != '\0')) {
       this->OverlayTypeIndex = -1;
       this->field_0x11e = 0;
   }
   ```
   Gated on `Tiberium=yes`. Does not fire for bridge overlays.

2. The IsoTileType branch when overlay is set:
   ```c
   else {  // SlopeIndex >= 5
       iVar9 = FUN_00544be0(this->Height);
       this->LandType = iVar9;
       this->OverlayTypeIndex = -1;
       this->field_0x11e = 0;
   }
   ```
   Indirectly gated by the Tiberium-overlay path above (the outer block only
   runs when `iVar9 == CellClass::OverlayToTiberiumIndex()` returns a valid
   tiberium index). Does not fire for bridge overlays.

## Implication

In gamemd, **engineer repair does not reset `+0x11E` at all**. The walker
overwrites only `+0x44` (OverlayTypeIndex) from a damaged value (e.g.,
`0x4E..0x52`) to an intact value (`0x4A + RNG(0..3)`). The damage-state byte
`+0x11E` retains whatever value it had during destruction — typically a
non-zero ramp-walk state.

This is harmless in normal play because:

- The damage state machine (`ProcessBridgeDamageStateMachine_{High,Low}`) is
  only invoked from `ApplyDamageToCell @ 0x00587180` when the cell takes a
  new weapon impact. At that point, the cell is dispatched again via the
  overlay-byte → state-machine routing.
- The intact-overlay routing in the state machine selects fresh state-byte
  values from the damage RNG; the stale value is overwritten.
- Rendering reads `+0x11E` only for anchor cells (overlays 0x18 / 0x19 / 0xED
  / 0xEE) — these are not modified by the repair walker, so their state is
  untouched.

## Rust port implication

If the Rust port's engineer-repair logic resets the damage byte to 0, that is
a **parity divergence** (the engine doesn't do it). However, the divergence is
not observable in any test the user has reported — the visible effect is the
overlay returning to intact, which both engines do.

Recommended action: **do not** add a Rust `damage_state = 0` reset on
engineer repair. Match gamemd: only the overlay is rewritten. If a Rust
implementation already resets the damage byte, leave it — it's a benign
divergence and probably more readable.

The "engineer-repair regression" the user mentioned is therefore **not** in
the `+0x11E` path. Investigate the cell's overlay byte first, and the Rust
port's analog of `+0x44 → bridge_overlay` write path in `RepairBridgeWalker_*`.

## Open question (genuinely open)

`+0x11E` on a body cell after engineer repair is left in a damage-state value
(typically 1-8 for HIGH NS, 10-17 for HIGH EW). The damage state machine
ladder starts at 0 (intact) and walks up to higher values as damage
accumulates. If `+0x11E` is left at e.g. `5` (already half-damaged), and the
cell takes a fresh damage hit, will the state machine treat it as continuing
from state 5 or restart from state 0?

Per `ProcessBridgeDamageStateMachine_High` switch shape, it appears to
**continue from the existing value**. That would mean repaired-then-redamaged
cells fall faster than freshly-damaged cells. This is potentially
player-observable: a repaired bridge dies in fewer hits than a fresh one. The
behavior is consistent with the binary but worth a fidelity check against a
running gamemd.

## Sources

- `CellClass::RecalcAttributes @ 0x0047D2B0` (decompiled)
- `OverlayTypeClass::ReadINI @ 0x005FE7A0` (decompiled — `+0x2a9` ← `Tiberium=`)
- `MapClass::RepairBridge_Low @ 0x0057F200` (decompiled — confirms no `+0x11E` write in dispatcher either)
- String `"Tiberium"` @ `0x00817278`
