# Anchor Overlay 0x18 vs 0x19 — Axis Resolution

**Investigator:** /re-swarm slot 4 (read-only Ghidra MCP)
**Date:** 2026-05-18
**Parent docs:** `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §10 Q7 / §12.13, `BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`, `BRIDGE_SYSTEM.md`.
**Status:** COMPLETE.

---

## 0. TL;DR

**`0x18` is the N-S high-bridge anchor; `0x19` is the E-W high-bridge anchor.**
Both codes route to the SAME high-bridge dispatcher (`ProcessBridgeDamageStateMachine_High`) — `+0x44` differentiates only "high vs low anchor"; the **axis is carried by `cell.Flags & 0x800`** (set for NS, clear for EW) and by **`cell.field_0x11E`** (0 for NS, 9 for EW). These are written by `SetBridgeDirection_NESW(direction, 1)` at map-load, with `direction=0` for `0x18` and `direction=6` for `0x19`.

Cross-evidence from 4 independent sites is unanimous (writer at map-load, walker step in the unified destruction entry, ramp-update calls per state in the damage state machine, and frame-index pick in `DrawOverlay_Body`). **Correction to `BRIDGE_SYSTEM.md` line 37:** the inferred polarity "bit 0x800 → 0=N-S, 1=E-W" is inverted — bit SET means NS, bit CLEAR means EW.

---

## 1. Writer sites for `0x18` (NS anchor)

| Site | Address | Active in YR | Notes |
|---|---|---|---|
| `OverlayClass::Mark` dispatch for overlay-id 0x18 → `SetBridgeDirection_NESW(direction=0, state=1)` | `0x005FC5FE` (`PUSH 0x1; PUSH 0` ; `CALL 0x47E040`) | **Yes** — runs during `ReadMapOverlayPacks` `[OverlayPack]` first pass at every map load | The actual byte stored at `cell+0x44` (`OverlayTypeIndex`) is `0x18`; the call to `SetBridgeDirection_NESW` then *derives* the structural flags and the frame byte. |
| `CellClass::SetBridgeDirection_NESW(direction=0, state=1)` → writes `cell.field_0x11E = 0` and `cell.Flags |= 0x80 | 0x100 | 0x200 | 0x800 | 0x1000 | 0x10000` on anchor cell, stamps N/N/N/S slot pattern | `0x0047E040` (entry); `+0x80` write at `0x0047E0E7` via `(param_3 & 1) << 7`; `+0x800` write via `(param_2 == 0) << 0xb` | Yes — invoked by `OverlayClass::Mark` (map-load), `MapClass::Resize` (re-stamp), and damage-walker rebuild callers. |

The byte 0x18 itself is written into `cell+0x44` by `[OverlayPack]` parsing (`ReadMapOverlayPacks @ 0x005FD2E0`) **before** `OverlayClass::Mark` is called. The stamping helper does not rewrite `+0x44`; it stamps the surrounding flags + `+0x11E` (anchor frame index 0).

## 2. Writer sites for `0x19` (EW anchor)

| Site | Address | Active in YR | Notes |
|---|---|---|---|
| `OverlayClass::Mark` dispatch for overlay-id 0x19 → `SetBridgeDirection_NESW(direction=6, state=1)` | `0x005FC60A` (`PUSH 0x1; PUSH 0x6; CALL 0x47E040`) | **Yes** — same map-load path as 0x18 | Stores `0x19` into `cell+0x44`; calls SetBridgeDirection with direction=6. |
| `CellClass::SetBridgeDirection_NESW(direction=6, state=1)` → writes `cell.field_0x11E = 9`, sets `Flags |= 0x80 | 0x100 | 0x200 | 0x1000 | 0x10000`; bit `0x800` is **cleared** because `(param_2 == 0)` is false; stamps anchor + W/W/W/E/E + extra dir-6 cell | `0x0047E040` (same function); extra-cell branch at `0x0047E3FF` (`CMP param_2, 6`); `0x0047E406-0x0047E452` | Yes. |

## 3. Reader sites (consumers of `0x18` / `0x19`)

| Reader | Address | What it checks | What it does with axis |
|---|---|---|---|
| `ApplyDamageToCell` | `0x00587180` (test at ~`0x005872E0`: `*(int*)(puVar3+0x44) != 0x18 && != 0x19`) | Whether the cell pointed to by `+0x2C` (anchor partner) has overlay id 0x18 or 0x19 | If yes → dispatch to `ProcessBridgeDamageStateMachine_High`. **Both ids route identically — no axis split at this site.** |
| `ProcessBridgeDamageStateMachine_High` | `0x00576BA0` | Switches on `puVar9[0x11E]` (anchor's frame byte, 0 or 9 + state offset) | States 0-8 → `UpdateRamp_NS_DamageA/B_High` + `UpdateRamp_NS_CollapseA/B_High` then `SetBridgeDirection_NESW(0, 0)` (destroy with `direction=0`). States 9-17 → `UpdateRamp_EW_*_High` + `SetBridgeDirection_NESW(6, 0)` (destroy with `direction=6`). |
| `ProcessBridgeDestruction_High` (unified entry; map-init + engineer-repair) | `0x00573540` | Walker advance step uses `flags & 0x800` directly: `MapCoord_Add(&local_34, &g_DirectionOffsets + (-(uint)((uVar2 & 0x800) != 0) & 6) * 2)` at `LAB_005739da` | `flags & 0x800` SET → advance direction 6 (West) = walks along E-W axis... **wait — see §5**. The corresponding reverse-walk earlier (when `flags & 0x100` clear and `0x80` clear, near `puVar7 + 0x140 & 0x800`) sets `local_34 = (((flags & 0x800)?2:0) + 2)` — SET → 4 (South), CLEAR → 2 (East). These two walks operate on different sub-paths; see §5. |
| `CellClass::DrawOverlay_Body` (anchor-cell render branch when `Flags & 0x80` set) | `0x0047F6A0` (anchor branch ~`0x0047F740`) | Reads `cell.field_0x11E`; if equal to 0 or 9, adds `g_OverlayVarietyLatinSquare[((y&3)<<2) | (x&3)]` for variety | Frame `0` → NS-anchor SHP frame; frame `9` → EW-anchor SHP frame. No code-vs-code (0x18 vs 0x19) branching — render is driven entirely by `+0x11E`. |

## 4. Discriminator at each site

| Site | Discriminator |
|---|---|
| `OverlayClass::Mark` writers | **Constant operand.** `0x18` ⇒ `PUSH 0` (direction=0); `0x19` ⇒ `PUSH 6` (direction=6). Hard-coded per overlay id. |
| `SetBridgeDirection_NESW` flag/frame writes | **Param-driven.** `field_0x11E` ← `0 if param_2==0 else 9`. `Flags |= 0x800` only when `param_2 == 0`. |
| `ProcessBridgeDamageStateMachine_High` state dispatch | **`cell.field_0x11E`-driven** (the byte the writer set above). States 0-8 are the NS branch; 9-17 are the EW branch. |
| `ProcessBridgeDestruction_High` axis walker | **Flag-bit-driven** (`flags & 0x800`). When the function picks an anchor and needs to traverse, the advance direction is computed from bit 0x800. |
| `DrawOverlay_Body` anchor branch | **`+0x11E`-driven.** No overlay-code branching at render time. |

## 5. Reconciliation — `0x18 = NS`, `0x19 = EW`

The map-load chain locks the axis assignment to the destroy-state-machine's NS/EW labels via two independent constants:

1. **Stamp pattern from `BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md` §"Slot positions"** (verified in decomp of `SetBridgeDirection_NESW`):
   - `direction=0` → stamps `anchor, N×3, S×1` — a 5-cell column along the **Y axis** = an **N-S bridge**.
   - `direction=6` → stamps `anchor, W×3, E×1` + extra E cell — a 6-cell row along the **X axis** = an **E-W bridge**.
2. **State-machine ramp-update call set** in `ProcessBridgeDamageStateMachine_High` (decompiled directly): the `field_0x11E ∈ [0..8]` branch exclusively calls the `UpdateRamp_NS_*_High` family; the `[9..17]` branch exclusively calls `UpdateRamp_EW_*_High`. The writer sets `+0x11E = 0` for `direction=0` and `+0x11E = 9` for `direction=6`. Therefore `direction=0 ↔ NS ↔ 0x18` and `direction=6 ↔ EW ↔ 0x19` is internally consistent across writer, state machine, and ramp-update naming (the ramp-update names are *not* swapped — see HIGH_BRIDGE_DAMAGE §7's note about *walker* labels being swapped, which does NOT apply to the `UpdateRamp_*_High` family).

The bit-0x800 polarity rule (writer in NESW: `(param_2 == 0) << 0xb`) is consistent with the `ProcessBridgeDestruction_High` walker. The walker reads `flags & 0x800`:

- **Anchor-reverse step** (when `flags & 0x100` clear): `local_34 = (((flags & 0x800)?2:0) + 2)`. SET → 4 (S), CLEAR → 2 (E). This walks **opposite** the bridge body axis to find the destroyed tail end — for an NS bridge (bit set) you walk S; for an EW bridge (bit clear) you walk E. Both are "advance to the opposite ramp end".
- **Forward-advance step** at `LAB_005739da`: `MapCoord_Add(..., &g_DirectionOffsets + (-(uint)((flags & 0x800)!=0) & 6) * 2)`. SET → direction 6 (W), CLEAR → direction 0 (N). This is the **next-anchor-cell step**: for an NS bridge (bit set) advance W (perpendicular hop to neighbouring parallel bridge cell); for an EW bridge advance N. Both are valid axis-perpendicular hops for the **5×5 damaged-cell scan** that this function performs at entry. The walks make sense only with the polarity `bit 0x800 SET = NS bridge`.

**Resulting verdict:**

| Overlay id | Axis | `+0x11E` after stamp | `Flags & 0x800` after stamp | State-machine half |
|---|---|---|---|---|
| **`0x18`** | **N-S** | `0` | **SET** | states 0-8 (NS ramp updates) |
| **`0x19`** | **E-W** | `9` | **CLEAR** | states 9-17 (EW ramp updates) |

The dispatcher `ApplyDamageToCell` treats them identically (both are "high-bridge anchor, send to high state machine"). The axis differentiation happens entirely through the **stamped derived state** (`+0x11E` + bit 0x800), not through ongoing reads of `+0x44`.

**Correction to `BRIDGE_SYSTEM.md` line 37** ("bit 11 0x0800 Bridge orientation (0=N-S, 1=E-W)") — the polarity is inverted. Bit-value 1 (SET) corresponds to N-S, not E-W. The doc's claim was inferred; this report disproves it via the explicit constant in the writer (`(param_2 == 0) << 0xb` runs only for the NS-stamped direction-0 call).

## 6. Active in YR — verdict per site

| Site | Active in YR | Reason |
|---|---|---|
| `OverlayClass::Mark` writers @ `0x005FC5FE` / `0x005FC60A` | **Yes** | `ReadMapOverlayPacks` is the standard map-load path. Every retail YR skirmish hits it. |
| `SetBridgeDirection_NESW @ 0x0047E040` | **Yes** | Called by `OverlayClass::Mark` (always), `MapClass::Resize`, and the damage state machine collapse cases (`SetBridgeDirection_NESW(0, 0)` / `(6, 0)`). |
| `ApplyDamageToCell @ 0x00587180` reader | **Yes** | Called from area-damage / direct-damage paths on every weapon impact onto a bridge cell. |
| `ProcessBridgeDamageStateMachine_High @ 0x00576BA0` | **Yes** — gated on `SpecialFlags & 0x8000` (`DestroyableBridges`, defaults `yes` in YR skirmish per HIGH_BRIDGE_DAMAGE §11.10). |
| `ProcessBridgeDestruction_High @ 0x00573540` walker | **Yes** — engineer-repair path (always) and map-init building-death path (always). |
| `DrawOverlay_Body @ 0x0047F6A0` anchor branch | **Yes** — runs every render frame for any visible anchor cell. |

No TS-only path was found in this chain.

## 7. Open Questions

None for the axis question itself. Adjacent loose ends (out of this slot's scope):

- The `g_OverlayVarietyLatinSquare` table at `+ ((y&3)<<2 | x&3) * 4` adds frame variety to anchor frames 0 and 9 only. The actual entries are runtime-initialized; mention is for record (not in this report's scope).
- The `+0x11E = 0 or 9` write happens on the anchor + forward-1 + forward-2 + opposite slots in the same NESW call (per BRIDGE_SETBRIDGEDIRECTION_STAMPING §"Per-slot behavior"). The frame index 0/9 thus propagates beyond the anchor cell to the entire bridgehead set; downstream rendering uses each cell's own `+0x11E` value.

## 8. Sources

**Decompiled in this session (live Ghidra MCP, read-only):**

- `OverlayClass::Mark` @ `0x005FC570` — confirmed writer dispatch.
- `CellClass::SetBridgeDirection_NESW` @ `0x0047E040` — confirmed `(param_2==0)<<0xb` bit-0x800 polarity and `field_0x11E = 0 if param_2==0 else 9`.
- `ApplyDamageToCell` @ `0x00587180` — confirmed reader at `0x44 == 0x18 || == 0x19`.
- `ProcessBridgeDamageStateMachine_High` @ `0x00576BA0` — confirmed NS vs EW ramp-update split on `+0x11E` and destroy calls `SetBridgeDirection_NESW(0/6, 0)`.
- `ProcessBridgeDestruction_High` @ `0x00573540` — confirmed walker advance uses `flags & 0x800`.
- `CellClass::DrawOverlay_Body` @ `0x0047F6A0` — confirmed render uses `+0x11E` only; no overlay-code split.
- `g_DirectionOffsets` @ `0x0089F688` read (verified BSS/zero in static image; runtime-initialised to compass offsets per HIGH_BRIDGE_DAMAGE §11.7: N=0, E=2, S=4, W=6).

**Cross-referenced docs:**

- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §10 Q7 (now resolved), §11.5, §11.7, §11.2, §12.1, §12.13.
- `BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md` §"Map-load flow", §"Intact-state stamp table".
- `BRIDGE_SYSTEM.md` line 37 — **flagged for correction**: inverted polarity claim on bit 0x800.
- `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` §3.4 — bit-write map confirmation.

**Confidence (3 axes):**

- *Content*: HIGH. Five independent live decompilations cross-confirm the writer/reader/render chain.
- *Identity*: HIGH for `OverlayClass::Mark` (vtable-anchored), `SetBridgeDirection_NESW` (called with constant operands from Mark), `ProcessBridgeDamageStateMachine_High` (caller-traced via ApplyDamageToCell), `DrawOverlay_Body` (vtable slot per `BRIDGE_RENDERING_GHIDRA_REPORT.md`). MEDIUM for `ProcessBridgeDestruction_High` identity since identification rests on `MapClass::RepairBridge_High` call (parent doc's claim).
- *Binding*: HIGH for the axis-assignment claim: it is locked by the two `PUSH 0` / `PUSH 6` constants at the writer dispatch and is invariant across all readers.
