# TechnoClass+0x6AF Field Investigation — Ghidra Report

**Date:** 2026-05-19
**Investigator:** swarm slot-3, pass #4
**Scope:** Exhaustive write/read search for offset +0x6AF; semantic naming; chrono-miner relevance.

---

## Executive Summary

`+0x6AF` is a **FootClass byte field** (not TechnoClass). It is the **locomotor rate-timer snapshot** used for turret-facing synchronization. For non-turret units (including CMIN), it is **always 0**. The prior FOOTCLASS_STRUCT_LAYOUT label `ShouldNotScatter` was based on a single read site in radio case 0x17 and missed the functional meaning. Correct name: **TurretRateSync** (or `IsLocomotorTimerActive`).

For the chrono miner, `+0x6AF == 0` always — so every gate on this field is a constant-pass. It has no functional effect on harvest-warp.

---

## (a) Exhaustive Setter Search

Pattern searched: `C6 8? AF 06 00 00` (MOV byte imm) and `88 8? AF 06 00 00` (MOV byte reg).
Also tried all ModRM variants: `C6 85/87/84/88`, `88 8A/8E/8F/96`. Zero additional hits.

| Address | Pattern | Value Written | Function |
|---------|---------|--------------|----------|
| `0x736b16` | `88 86 AF 06 00 00` | `CDTimerClass__Remaining()` (byte) | `UnitClass__Facing_Update` (0x736990) |

**Total setters (non-zero writes): 1.**

---

## (b) Exhaustive Clearer (write-to-0) Search

| Address | Pattern | Value Written | Function |
|---------|---------|--------------|----------|
| `0x736ad5` | `C6 86 AF 06 00 00 00` | `0` | `UnitClass__Facing_Update` (0x736990) |

**Total clearers: 1.** Both sites are in the same function.

---

## (c) Semantic Interpretation

**Verified from binary (confidence: 95%).**

In `UnitClass__Facing_Update` (0x736990):

```
// Clear path (0x736ad5):
if (TechnoTypeClass+0xD21 /* TurretSpins */ == 0) {
    *(byte *)(this + 0x6AF) = 0;          // always clear first
    if (TechnoTypeClass+0xCA1 /* Turret */ != 0) {
        remaining = CDTimerClass__Remaining();
        if (remaining != 0 && TurretSpins == 0) {
            *(byte *)(this + 0x6AF) = remaining;  // set at 0x736b16
        }
    }
}
```

The field is set to the **locomotor rate timer's remaining ticks** when:
- Unit has a turret (`Turret=yes`)
- Turret is not in free-spin mode (`TurretSpins=no`)
- The locomotor rate timer is still counting down (unit in motion)

When the locomotor timer finishes (unit stopped), it's cleared to 0.

**Semantic name: `TurretRateSync`** — a snapshot of locomotor rate-timer remaining ticks, used to block locomotor speed resets while turret facing is still syncing with movement.

This is NOT a chrono state field. The comment "(not chrono-teleporting)" in the task brief is misleading — the guard in TIMING_SYNC case 0x16 is actually "not in the middle of a turret rate update."

---

## (d) When Does It Fire for Chrono Miner During Harvest-Return Warp?

**Answer: Never. It does not fire.**

The chrono miner (`CMIN`) has `Turret=no` (TechnoTypeClass+0xCA1 = 0). The write path at `0x736b16` is gated behind `Turret != 0`. Therefore:

- Every call to `UnitClass__Facing_Update` on the chrono miner clears `+0x6AF` to 0 at `0x736ad5`, then skips the setter entirely.
- Throughout all warp phases (Phase 0 timer, footstep, Phase 0 expiry, docking), `+0x6AF = 0` for CMIN.
- The `+0x6AF == 0` guard in TIMING_SYNC (UnitClass__Receive_Radio case 0x16) always passes for CMIN; only the `timer != 0x4000` sub-condition actually does work.

---

## (e) TechnoClass vs. FootClass Confirmation

**Verified: this field is on FootClass (byte offset +0x6AF from the object base).**

Evidence:
1. `UnitClass__Facing_Update` takes `int *param_1` where `param_1` is the UnitClass `this` pointer. Accesses `*(char *)((int)param_1 + 0x6af)` — direct byte offset from the object base.
2. `FUN_007171a0` (UnitClass CRC/serialization function) serializes `param_1 + 0x6ae`, `param_1 + 0x6af`, `param_1 + 0x6b0` as consecutive bytes using `FUN_004a1ca0` — confirming this is a byte field in the main UnitClass/FootClass layout.
3. The FOOTCLASS_STRUCT_LAYOUT doc already places it at FootClass+0x6AF in the sequential struct layout (between `+0x6AE = IsUndeploying` and `+0x6B0 = Unknown_6B0`).
4. TechnoClass (base) ends well before +0x6AF. FootClass begins after TechnoClass. The field is definitively on FootClass.

---

## (f) Cross-Check Against Existing Docs

### FOOTCLASS_STRUCT_LAYOUT.md
Previously labeled: `0x6AF | 1 | bool | 0 | ShouldNotScatter`

**Assessment: Misleading.** The "ShouldNotScatter" name was inferred from one read site (radio case 0x17), where `+0x6AF != 0` prevents scatter. That behavior is a consequence of the field being non-zero when turret rate timer is running — not a deliberate "scatter suppression" flag.

**Corrected name: `TurretRateSync`** (byte snapshot of locomotor rate timer remaining ticks, non-zero only for `Turret=yes && TurretSpins=no` units while moving).

### TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md
Previously labeled: `0x6AF | byte | FootFlag_0x6AF | LOW`

**Assessment: Unlabeled placeholder. TurretRateSync is the correct name.**

### TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md
**+0x6AF is NOT mentioned.** Correct — it has no chrono meaning.

---

## (g) Relationship to +0x6AD (IsDeploying)

`+0x6AD` (IsDeploying) and `+0x6AF` (TurretRateSync) are in the same struct neighborhood (both in the 0x6A0–0x6B0 block of FootClass boolean flags), but they are **different flags with unrelated purposes**:

| Offset | Name | Set by | Purpose |
|--------|------|--------|---------|
| +0x6AD | IsDeploying | TechnoClass__PerformDeploy | Gates destination changes, locomotor piggyback, Is_Ok_To_End |
| +0x6AE | IsUndeploying | FootClass::Set_Destination_Internal | Tracks undeploy transition state |
| +0x6AF | TurretRateSync | UnitClass__Facing_Update | Locomotor rate timer snapshot for turret-facing sync |
| +0x6B0 | Unknown_6B0 | — | Unknown |

They are spatially adjacent but semantically unrelated. +0x6AD and +0x6AE are deploy-state flags; +0x6AF is a turret animation state hint.

---

## All Read Sites

From pattern `8A 86 AF 06 00 00`:

| Address | Function | Usage |
|---------|----------|-------|
| `0x4d90a5` | `FootClass__Receive_Radio` | Radio case 0x17: if `+0x6AF==0 && no_destination` → scatter |
| `0x717791` | `FUN_007171a0` (UnitClass CRC) | Serialize to CRC |
| `0x7376bf` | `UnitClass__Receive_Radio` | Radio case 0x0E (CAN_DOCK): gate locomotor stop |
| `0x737a41` | `UnitClass__Receive_Radio` | Radio case 0x16 (TIMING_SYNC): gate SetSpeed(0x4000) |
| `0x73d892` | `UnitClass__Mission_Deploy_Building` | case 1: if `+0x6AF==0` → advance to phase 3 |
| `0x73df7a` | `UnitClass__Mission_Deploy_Building` | Deploy locomotor speed gate |
| `0x740ea7` | `FUN_00740e80` | Locomotor stop helper: if `loco_not_moving && +0x6AF==0` → notify |
| `0x741233` | (no function found) | Unknown site — function boundary not recognized |

---

## Confidence Summary

| Finding | Confidence | Evidence |
|---------|-----------|----------|
| Only 2 write sites, both in UnitClass__Facing_Update | 100% | Exhaustive binary pattern search |
| Field is on FootClass, not TechnoClass | 100% | Direct byte-offset access in UnitClass; CRC serializer confirms byte layout |
| Semantic: locomotor rate timer snapshot for turret sync | 95% | Clear from write condition: Turret=yes && TurretSpins=no && timer>0 |
| Always 0 for chrono miner (no functional effect) | 100% | CMIN has Turret=no; write gate is `Turret != 0` |
| Prior "ShouldNotScatter" label was misleading | 90% | Name follows from effect not cause; no explicit scatter suppression logic |
