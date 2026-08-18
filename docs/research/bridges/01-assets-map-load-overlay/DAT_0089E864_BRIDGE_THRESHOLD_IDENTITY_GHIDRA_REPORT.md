# DAT_0089E864 Bridge Layer Threshold — Identity Investigation

**Target:** `DAT_0089E864` — object-layer bridge-selection threshold in `Apply_area_damage`  
**Primary function:** `Apply_area_damage @ 0x00489280`  
**Investigation date:** 2026-05-18  
**Confidence:** HIGH for all identity and derivation claims; verified directly from Ghidra decompilation and byte-level disassembly.  
**Active in YR:** Yes. Used every time a splash warhead detonates near a bridge cell in a standard YR skirmish.

---

## 1. One-Line Answer

`DAT_0089E864` is **`2 × BridgeHeight`** — a theater-init-derived constant, not an INI-read value. It is not `Rules.BridgeHeight` and is not an independent INI key. It equals `2 × DAT_0089E870` (where `DAT_0089E870` is the bridge height in leptons, nominally 104).

---

## 2. Verified Usage in `Apply_area_damage`

From the live Ghidra decompilation of `Apply_area_damage @ 0x00489280`:

### 2.1 Object-layer selector (addresses 0x0048957A–0x0048958D)

```c
if ((local_c8[0x50] & 0x100U) != 0) {   // cell has bridge flag
    iVar10 = CellClass__GetGroundHeight();
    if (iVar10 + DAT_0089e864 / 2 < param_1[2]) {   // impact_z > ground_z + BridgeHeight
        // select bridge deck layer (CellClass+0xE8)
    }
}
```

Because `DAT_0089E864 = 2 × BridgeHeight`, the integer division `DAT_0089E864 / 2 = BridgeHeight`.
Net effect: **select the bridge deck layer when `impact_z > ground_z + BridgeHeight`**.

### 2.2 Bridge tile Z-gate (addresses 0x00489F90, 0x0048A114)

Both high-bridge and low-bridge state-machine paths gate the state-machine call on:

```c
if ((this->Level + 1) * DAT_0089e870 + DAT_0089e864 < local_c8[2] ||
    local_c8[2] <= (this->Level - 2) * DAT_0089e870 + DAT_0089e864) {
    goto skip;   // explosion too far above or below — skip state machine
}
```

Here `DAT_0089E864` is the **base Z at Level=0**. The formula `Level × BridgeHeight + CellBaseZ` gives the world Z for a given terrain level. `DAT_0089E864` is the intercept (Z when Level=0). With `DAT_0089E864 = 2 × BridgeHeight = 208` and `DAT_0089E870 = 104`, a level-0 bridge cell has base Z = 208, level-1 bridge has Z = 312, etc.

Evidence: confirmed at addresses `0x00489F90`, `0x00489FAB`, `0x0048A114`, `0x0048A127` in the live decompile.

---

## 3. The Single Writer — Function at 0x00489100

`get_xrefs_to(0x0089E864)` returns exactly one WRITE: **`0x00489120`**, inside an unnamed stub function at `0x00489100`.

Direct byte disassembly of `0x00489100`:

```
51                         PUSH ECX
a1 70 e8 89 00             MOV  EAX, [0x0089E870]          ; EAX = BridgeHeight (int)
8d 0c 85 00 00 00 00       LEA  ECX, [EAX*4 + 0]           ; ECX = BridgeHeight * 4
89 4c 24 00                MOV  [ESP+0], ECX               ; push to stack
db 44 24 00                FILD DWORD PTR [ESP+0]           ; FPU = float(BridgeHeight * 4)
dc 05 38 17 7e 00          FMUL QWORD PTR [0x007E1738]     ; × 0.5  =>  BridgeHeight * 2.0
e8 e0 cd 33 00             CALL 0x007C5EE5                  ; Math__ftol → EAX = BridgeHeight * 2
a3 64 e8 89 00             MOV  [0x0089E864], EAX          ; WRITE DAT_0089E864
59                         POP ECX
c3                         RET
```

The constant at `0x007E1738` is `0x3FE0000000000000` (IEEE 754 double) = **0.5**. Verified by `read_memory(0x007E1738, 8)` = `00 00 00 00 00 00 E0 3F`.

**Derivation formula:** `DAT_0089E864 = ftol(DAT_0089E870 × 4 × 0.5) = ftol(DAT_0089E870 × 2) = DAT_0089E870 × 2`

This function is referenced only through a function-pointer table at `0x00812A68` (DATA xref), which is part of the C++ static initializer / theater-geometry init sequence. There are no call-site xrefs — it is dispatched indirectly.

---

## 4. What DAT_0089E870 Is

`DAT_0089E870` is the **bridge height in leptons** (= `LevelHeight`). From `splash_cellspread.md`: value is nominally **104** leptons. It is used directly in `Apply_area_damage` at:
- `0x0048979E`, `0x0048982B` — object distance adjustment for buildings on bridges (`BridgeHeight * 2`)
- `0x00489F90`, `0x0048A114` — bridge tile Z-gate (alongside `DAT_0089E864`)
- `0x0048A541` — `Warhead__SelectExplosionAnim` (bridge explosion Z computation)

Its writer is at `0x0048908B` (via a similar nearby stub function). `DAT_0089E870` is itself a theater-geometry-derived constant computed from FPU arithmetic on camera/geometry parameters — not directly from `Rules.BridgeHeight` INI.

---

## 5. INI Cross-Check — Rules.BridgeHeight

The INI key `Rules.BridgeHeight` does **not** appear in `ini/rulesmd.ini` or `ini/rules.ini`.  
Searching all research docs: `BridgeHeight` appears as a semantic label in several reports but is always used as a name for `DAT_0089E870` (the per-level Z step), not for an INI-parsed field at `0x0089E864`.

`RulesClass__ReadCombatDamage @ 0x66CD60` (verified in BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md §2) reads `BridgeStrength` into `Rules+0x1740` — it does NOT write to `0x0089E864` or `0x0089E870`.

No evidence of any `ReadInt`/`ReadBool`/`ReadString` writer for `0x0089E864`. Identity is **theater-geometry-derived, not INI-driven**.

---

## 6. Alternative Labels in Existing Docs — Reconciliation

| Source document | Label for `DAT_0089E864` | Accurate? |
|---|---|---|
| `WARHEAD_DETONATE_GHIDRA_REPORT.md` §8 | `CellHeight — base cell height in leptons` | Partially. It is a "base" height constant but the name CellHeight is ambiguous; value is `2 × BridgeHeight`. |
| `splash_cellspread.md` §14 | `bridge-Z offset` | Accurate as a descriptor; not a name. |
| `PHASE_F_BRIDGE_DAMAGE_DISPATCH_VERIFICATION.md` §1 | `DAT_0089e864` (unnamed) | Used correctly as base intercept in level formula. |
| This report | `g_CellBaseZ` / `2 × BridgeHeight` | **Recommended**: `g_DoubleLevelHeight` or `g_CellBaseZ_Derived` to distinguish it from the raw BridgeHeight. |

**Recommended canonical name:** `g_CellBaseZ` (the base Z world coordinate for a terrain level-0 cell, derived from the theater geometry). Value at runtime: `2 × DAT_0089E870 = 2 × LevelHeight`.

---

## 7. Rust Parity Impact

Current `src/sim/combat/combat_aoe.rs` uses the threshold documented in `BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md` §3.1:

```
threshold_z = ground_z + DAT_0089E864 / 2
```

Which simplifies to `ground_z + BridgeHeight`. This is correct. The Rust implementation only needs to use the raw `BridgeHeight` (= `LevelHeight`) constant for the threshold — the binary's `DAT_0089E864 / 2` detour produces the same result.

No code change needed if the Rust side already uses `ground_z + level_height` for the threshold. If it uses `ground_z + level_height / 2`, that is wrong — the correct threshold is `ground_z + level_height`, not `ground_z + level_height / 2`.

---

## 8. Open Questions (deferred, not in scope)

1. What exactly populates `DAT_0089E870` (BridgeHeight = 104)? The writer stub at `~0x00489080` computes it from theater camera geometry. The exact formula chain from `DAT_0089E7F8`, `DAT_0089E820`, `DAT_0089E818` to the final value is untraced.
2. Does the theater-init always produce 104, or does it vary by theater? If it varies, the Rust constant must be theater-resolved, not hardcoded.
3. The `BridgeHeight * 2` check at `Apply_area_damage` line `if (DAT_0089e870 * 2 < param_1[2] - *(int *)(iVar10 + 8))` — this is for building-on-bridge distance correction; its parity in Rust was not checked in this investigation.

---

## 9. Load-Bearing Verified Facts

1. `DAT_0089E864 = 2 × DAT_0089E870` — single writer at `0x00489120`, verified by direct byte read: `LEA ECX, [EAX*4]` then `FMUL 0.5`. (`read_memory(0x00489100, 48)`)
2. Object-layer threshold in `Apply_area_damage`: `ground_z + DAT_0089E864 / 2 < impact_z` — directly from Ghidra decompile of `0x00489280`, confirmed at the comparison construct around `0x0048957A–0x0048958D`.
3. `0x007E1738` = `0.5` (IEEE 754 double `3FE0000000000000`) — verified by `read_memory(0x007E1738, 8)`.
4. No INI reader writes `0x0089E864` — `RulesClass__ReadCombatDamage @ 0x66CD60` verified in BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md; only xref WRITE is the theater-init stub.
5. Bridge tile Z-gate formula: `(Level + 1) * DAT_0089E870 + DAT_0089E864` is the upper bound, `(Level - 2) * DAT_0089E870 + DAT_0089E864` is the lower bound — confirmed from Ghidra decompile of `Apply_area_damage @ 0x00489280` at lines using `this->Level`.

---

## 10. Sources

- Ghidra decompiled: `Apply_area_damage @ 0x00489280` (full function)
- Byte-level disassembly: `0x00489100–0x00489127` (writer stub for `DAT_0089E864`)
- `read_memory(0x007E1738, 8)` — constant 0.5 verification
- `get_xrefs_to(0x0089E864)` — single WRITE at `0x00489120`, four READs in `Apply_area_damage`
- `get_xrefs_to(0x0089E870)` — WRITE at `0x0048908B`, multiple READs in `Apply_area_damage` and `Warhead__SelectExplosionAnim`
- Docs referenced:
  - `BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md` (§3, §10 OQ-1)
  - `BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md` (§2 corrections, §13)
  - `WARHEAD_DETONATE_GHIDRA_REPORT.md` (§8 key globals table)
  - `PHASE_F_BRIDGE_DAMAGE_DISPATCH_VERIFICATION.md` (§1 finding 2)
  - `combat/systems/splash_cellspread.md` (§14 global table)
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini` — no `BridgeHeight` key found
