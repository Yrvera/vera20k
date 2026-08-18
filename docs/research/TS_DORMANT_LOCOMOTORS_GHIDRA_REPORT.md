# TS-Dormant Locomotors — Mech, DropPod, Tunnel — Ghidra Stub Report

**Date:** 2026-05-17
**Status:** All three classes are **DEFERRED-TS**. They exist as fully-compiled COM classes in `gamemd.exe` with constructors, vtables, NullCoord sentinel globals, and class factories registered at boot — but **no INI content in standard YR (or RA2) ever sets `Locomotor=` to their CLSIDs in an uncommented form**, and no engine code path other than the COM class-factory machinery references them. They are inherited from Tiberian Sun, kept compiled for binary compatibility, and never instantiated in standard YR skirmish.

**This doc is a stub.** Its purpose is to (a) document the evidence of dormancy so future investigators don't re-research them, and (b) record the addresses for the rare case where a mod activates one of these classes. **Do NOT implement these locomotors in the Rust engine** — they have no observable effect in standard play.

Per the parity bar in `CLAUDE.md`: the goal is indistinguishability from gamemd.exe **in observable behavior**. A locomotor that is never instantiated produces zero observable behavior, so omitting it from the Rust port is parity-correct.

---

## 1. Why these three were investigated

Index `INDEX_PATHFINDING_LOCOMOTION.md` listed each as TODO. Per `CLAUDE.md`'s reverse-engineering rules: "Always ask: is this code actually used in a standard YR skirmish?" This pass answers that question with evidence for each.

The investigation method for each class:
1. Find the CLSID in the binary (`search_byte_patterns` with the GUID bytes)
2. Find INI references via `Grep` on the CLSID prefix
3. Find constructor xrefs via `get_xrefs_to`
4. Find the class factory xref pattern (DATA xref from `WinMain` = boot-time COM registration only)
5. List vtable methods via `read_memory` on the ILocomotion vtable
6. Note that Ghidra has only auto-analyzed the *constructors* of these classes (not other methods) — because no analyzed code path reaches the other methods, so Ghidra never followed function-start markers for them

---

## 2. MechLocomotionClass — DEFERRED-TS

### 2.1 Identity

| Property | Value |
|---|---|
| CLSID | `{55D141B8-DB94-11D1-AC98-006008055BB5}` |
| GUID stored at | `0x007E9AA0` |
| Constructor | `0x005AFEF0` |
| Virtual destructor / scalar deleting | `0x005B1B50` |
| IUnknown vtable | `0x007EDC38` |
| ILocomotion vtable | `0x007EDB6C` (40 slots, 160 bytes) |
| IPiggyback vtable | **NONE** (constructor does not set `param_1[6]`) |
| Instance size | **~52 bytes** (last init at `+0x30`, much smaller than Drive's 0x6C) |
| NullCoord sentinel | `0x00ABEE10` / `0x00ABEE14` / `0x00ABEE18` |
| Confidence | C=HIGH, I=HIGH, B=HIGH |

### 2.2 Evidence of dormancy

**INI grep for `55D141B8` in `ini/rules.ini` and `ini/rulesmd.ini`:**

| File | Line | Form | Live? |
|------|------|------|-------|
| `rules.ini` | 5799 | `Locomotor={4A582741-...};<-drive   mech->{55D141B8-...}` | **No** — Drive CLSID is primary; Mech CLSID is in a comment annotation after `;` |
| `rules.ini` | 6505 | `;origional - Locomotor={55D141B8-...}` | **No** — entire line is commented |
| `rules.ini` | 6599 | `; origional - Locomotor={55D141B8-...}` | **No** |
| `rulesmd.ini` | 6635 | `; origional - Locomotor={55D141B8-...}` | **No** |
| `rulesmd.ini` | 7605 | `Locomotor={4A582741-...};<-drive   mech->{55D141B8-...}` | **No** — same annotation pattern |
| `rulesmd.ini` | 7715 | `;origional - Locomotor={55D141B8-...}` | **No** |
| `rulesmd.ini` | 8310 | `;origional - Locomotor={55D141B8-...}` | **No** |
| `rulesmd.ini` | 8798 | `Locomotor={4A582741-...};<-drive   mech->{55D141B8-...}` | **No** |

**Pattern across all 8 references:** Mech CLSID appears only as an `<-drive mech->` comment annotation or in fully commented-out `;origional - Locomotor=` lines. **Zero stock units use Mech as their active locomotor.** The comments are historical breadcrumbs from when Westwood was choosing between Drive and Mech for vehicles like Rhino Tank / Grizzly Tank / Terror Drone / Chaos Drone — all of which ultimately use Drive.

**Binary xrefs to CLSID `0x007E9AA0`:**

```
get_xrefs_to 0x7E9AA0 → 
  From 006BD380 in WinMain [DATA]    ← boot-time class factory registration
  From 005B1970 [READ]               ← inside Mech's QueryInterface (returns CLSID)
```

Only 2 xrefs:
1. **`WinMain @ 0x6BD380`** — boot-time `CoRegisterClassObject` registration. Necessary for COM to know about the class, but does NOT instantiate it.
2. **`0x005B1970`** — inside an unanalyzed Mech method, almost certainly the QueryInterface comparing IID against its own CLSID.

**No production-pipeline references** (compare to Ship's CLSID which has a `BuildingClass::MissionRepairAndProduce` xref). **No call to `CoCreateInstance(MechCLSID, ...)` anywhere in analyzed code** — verified by the absence of additional xrefs.

**Constructor xrefs:**
```
get_xrefs_to 0x5AFEF0 → 
  From 006C4DCC [UNCONDITIONAL_CALL]   ← IClassFactory::CreateInstance for Mech
```

Single caller — Mech's class-factory CreateInstance. **The factory exists but is never invoked** because no `CoCreateInstance(MechCLSID, ...)` happens in stock content.

### 2.3 Vtable structure (40 slots, partial decode)

Verified by `read_memory 0x007EDB6C len 160`:

| Slot | Addr | Status | Notes |
|------|------|--------|-------|
| 0 | `0x4D0510` | Shared QueryInterface | **Same address as Tunnel slot 0** — a generic locomotor QI helper |
| 1 | `0x4D0520` | Shared AddRef | Same as Tunnel slot 1 |
| 2 | `0x4D0530` | Shared Release | Same as Tunnel slot 2 |
| 3 | `0x55A710` | `LocomotionClass::Link_To_Object` (base, shared with Drive/Ship) | |
| 4 | `0x5AFF70` | `Mech::Is_Moving` (UNANALYZED — Ghidra didn't auto-analyze) | |
| 5 | `0x5AFF80` | `Mech::Destination` (UNANALYZED) | |
| 6 | `0x5AFFE0` | `Mech::Move_To` (UNANALYZED) | |
| 7 | `0x55ABF0` | base stub | |
| 8 | `0x55ABE0` | base stub | |
| 9 | `0x55A730` | base Draw_Matrix (Drive/Ship have class-specific) — **Mech uses generic** | |
| 10 | `0x55A7D0` | base Shadow_Matrix | |
| 11 | `0x55ABD0` | base | |
| 12 | `0x55A8C0` | `LocomotionClass::Can_Enter_Cell` (base) | |
| 13 | `0x55ABC0` | base | |
| 14 | `0x55ABA0` | base | |
| 15 | `0x55ABB0` | base | |
| 16 | **`0x5B0060`** | **`Mech::Process` (UNANALYZED)** | Real method address — exists but not auto-analyzed |
| 17 | `0x5B0080` | `Mech::Set_Destination` (UNANALYZED) | |
| 18 | `0x5B0120` | `Mech::Stop_Moving` (UNANALYZED) | |
| 19 | `0x5B0170` | `Mech::Do_Turn` (UNANALYZED) | |
| 20 | `0x55AC20` | base | |
| 28 | `0x0055AC10` | base (LocomotionClass) — Mech does NOT override slot 28 | Re-verified 2026-05-19 via `read_memory 0x007EDBDC 16` showing `10 ac 55 00` at slot 28 (base+0x70). Earlier doc revisions had `0x5B19D0` here, but `0x5B19D0` actually lives at slot 29 (next row). |
| 29 | `0x005B19D0` | `Mech::<override>` (UNANALYZED — likely Force_Track or In_Which_Layer) | Verified via `read_memory 0x007EDBE0 4` showing `d0 19 5b 00`. Function identity not independently confirmed (Ghidra never analyzed — vtable not invoked since no Mech instance is ever created in YR). |
| 30 | `0x005B01A0` | `Mech::<override>` (UNANALYZED) | Verified via `read_memory 0x007EDBE4 4` showing `a0 01 5b 00`. Identity unconfirmed; this slot was missing from earlier doc revisions entirely. |
| 31 | `0x0055ACE0` | base (Mech does NOT override slot 31 — unlike Drive/Ship) | Verified via `read_memory 0x007EDBE8 4` showing `e0 ac 55 00`. |
| 32 | `0x5B19E0` | `Mech::Is_Moving_Now` (UNANALYZED) | |
| 36-38 | `0x4B4C60/70/80` | shared with Drive/Ship (Begin/End_Piggyback, Is_Surfacing) | |
| 39 | `0x5B1A50` | `Mech::Is_To_Have_Shadow_Override` (UNANALYZED) | |

**Subtle detail — Ghidra's autoanalysis behaviour:** Ghidra only analyzes functions whose addresses are reached by some path it can follow (calls, jumps, vtable references *that are followed*). For Mech, only the constructor at `0x5AFEF0` is reached (via the class-factory CreateInstance at `0x6C4DCC`). The vtable method addresses at `0x5AFF70 / 0x5AFF80 / 0x5B0060 / etc.` are listed in the vtable but never followed because **the vtable is never actually invoked** (no instance is ever created). So those addresses contain real code but Ghidra hasn't analyzed them.

A user could force-analyze them via `create_function 0x5AFF70` etc. — but per the parity bar (these methods never execute), there's no parity-relevant reason to.

### 2.4 Verdict

**MechLocomotionClass is TS legacy, dormant in YR.** It was inherited from Tiberian Sun (where Cyborg/Wolverine-class walking units used it). All YR ground vehicles use `DriveLocomotionClass` instead, including units that were earlier prototyped with Mech (per the `<-drive mech->` INI annotations).

**Do NOT implement in Rust.**

---

## 3. DropPodLocomotionClass — DEFERRED-TS

### 3.1 Identity

| Property | Value |
|---|---|
| CLSID | `{4A582745-9839-11D1-B709-00A024DDAFD1}` |
| GUID stored at | `0x007E9A70` |
| Constructor | `0x004B5AB0` |
| Virtual destructor | `0x004B5B00` |
| Scalar deleting destructor | `0x004B66F0` |
| IUnknown vtable | `0x007E8344` |
| ILocomotion vtable | `0x007E8278` (40 slots) |
| IPiggyback vtable | `0x007E8254` |
| Instance size | **~48 bytes** (last init at `+0x2C`) |
| NullCoord sentinel | `0x008A0820` / `0x008A0824` / `0x008A0828` |
| Confidence | C=HIGH, I=HIGH, B=HIGH |

### 3.2 Evidence of dormancy

**INI grep for `4A582745` in `ini/rules.ini` and `ini/rulesmd.ini`:** **ZERO matches.** No commented, no uncommented, no historical annotation — **not referenced at all** in stock INI content.

This is even stronger evidence of dormancy than Mech's "all-commented" pattern. DropPod has been forgotten so thoroughly that there isn't even a commented-out hint left in the INI files.

**Confirmed independently by `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` §0:**
> "DropPod (item #28) | Active in YR: **No** — TS-DEAD | class never instantiated in standard YR (zero INI refs to CLSID `4A582745`)"

### 3.3 Vtable highlights (40 slots, partial decode from `0x7E8278`)

> **2026-05-19 verification note:** A spot-check during the verify-doc-swarm audit
> compared slot indices in this table against `read_memory` of the live vtable and
> found inconsistencies — both the original partial decode AND the swarm's proposed
> "corrections" used different slot-numbering conventions, and neither matches the
> raw vtable layout cleanly. Until a full byte-by-byte walk of `0x007E8278` len 160
> is performed and reconciled with the slot indices below, treat the slot-number→address
> mapping in this table as APPROXIMATE — the addresses themselves are real (they appear
> somewhere in this vtable) but their slot indices may be off by 1-3 slots. Function
> identities for UNANALYZED entries are inferences and should not be relied upon for
> implementation decisions. The dormancy verdict is unaffected (no DropPod instance
> ever exists in YR), but TS-archaeology consumers should re-verify before citing.

| Slot | Addr | Notes |
|------|------|-------|
| 0-2 | `0x4B6740/50/60` | DropPod-specific QI/AddRef/Release |
| 3 | `0x55A710` | base Link_To_Object |
| 4 | `0x4B5B30` | DropPod::Is_Moving (UNANALYZED) |
| 5 | `0x4B5B40` | DropPod::Destination (UNANALYZED) |
| 6 | `0x55ACA0` | **base** — DropPod uses generic Move_To |
| 9 | `0x55A730` | base Draw_Matrix |
| 16 | **`0x4B5B70`** | **DropPod::Process (UNANALYZED)** |
| 17 | `0x4B6040` | DropPod::Set_Destination |
| 18 | `0x4B63A0` | DropPod::Stop_Moving |
| 29 | `0x4B64D0` | DropPod::In_Which_Layer |
| 31 | `0x4B6610` | (unanalyzed) |
| 32 | `0x4B6510` | DropPod::Is_Moving_Now |

DropPod **has an IPiggyback vtable** (slot pattern `param_1[6] = &IPiggyback`) — meaning if it WERE instantiated, it could be piggybacked under another locomotor (e.g., infantry parachute-mode). Since it's never instantiated, this is irrelevant.

### 3.4 Verdict

**DropPodLocomotionClass is TS legacy, dormant in YR.** Originally for the TS Drop Pod superweapon's per-unit landing locomotion. In YR, the Allied Paradrop superweapon uses `[ParaBomb]`-style anim spawning instead, and there is no DropPod locomotor instantiation.

**Do NOT implement in Rust.**

---

## 4. TunnelLocomotionClass — DEFERRED-TS

### 4.1 Identity

| Property | Value |
|---|---|
| CLSID | `{4A582743-9839-11D1-B709-00A024DDAFD1}` |
| GUID stored at | `0x007E9A50` |
| Constructor | `0x00728A00` |
| Virtual destructor / scalar deleting | not analyzed by Ghidra |
| IUnknown vtable | `0x007F5AF0` |
| ILocomotion vtable | `0x007F5A24` (40 slots) |
| IPiggyback vtable | **NONE** (constructor does not set `param_1[6]`) |
| Instance size | **~57 bytes** (last init at `+0x38`, larger than Mech/DropPod) |
| NullCoord sentinel | `0x00B0F910` / `0x00B0F914` / `0x00B0F918` |
| Additional data global | `0x00A8ED84` (likely a TS subterranean-related timer/threshold) |
| Confidence | C=HIGH, I=HIGH, B=HIGH |

### 4.2 Evidence of dormancy

**INI grep for `4A582743`:** **ZERO matches.** Like DropPod, not referenced at all in stock INI.

**Confirmed independently by `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` §0:**
> "Tunnel (item #30) | Active in YR: **No** — TS-DEAD | no INI refs to CLSID `4A582743`"

**Confirmed in MEMORY.md:** Per `[[feedback_no_tunnel_subterranean]]` — "Tunnel/subterranean is TS legacy; not in RA2 or YR — skip in audits and gap scans".

### 4.3 Vtable highlights (40 slots, partial decode from `0x7F5A24`)

> **2026-05-19 verification note:** Same caveat as §3.3 applies here. The slot indices
> in this table were not reconciled byte-for-byte against `read_memory` of the live
> vtable during the 2026-05-19 audit. The swarm's proposed "corrections" used a
> different (also-unverified) numbering convention. The addresses listed below exist
> in the vtable but their exact slot positions may be off by 1-3 slots. Dormancy
> verdict is unaffected (no Tunnel instance ever exists in YR). Re-verify with
> `read_memory 0x007F5A24 160` before citing for implementation purposes.

| Slot | Addr | Notes |
|------|------|-------|
| 0 | `0x4D0510` | **Shared with Mech slot 0** — generic locomotor QueryInterface |
| 1 | `0x4D0520` | Shared with Mech |
| 2 | `0x4D0530` | Shared with Mech |
| 3 | `0x55A710` | base Link_To_Object |
| 4 | `0x728A50` | Tunnel::Is_Moving (UNANALYZED) |
| 5 | `0x728A90` | Tunnel::Destination (UNANALYZED) |
| 6 | `0x55ACA0` | base — same as DropPod |
| 7 | `0x72A090` | **Tunnel::Stop_Moving (CLASS-SPECIFIC)** |
| 8 | `0x72A060` | Tunnel::Do_Turn |
| 9 | `0x729B40` | **Tunnel::Draw_Matrix (CLASS-SPECIFIC)** — likely the surface/submerge transition matrix |
| 13 | `0x72A020` | Tunnel::Is_To_Have_Shadow |
| 14 | `0x728E30` | (probably Mark_All_Occupation_Bits) |
| 15 | `0x728AF0` | |
| 16 | **`0x728C00`** | **Tunnel::Process (UNANALYZED)** |
| 17 | `0x728F30` | Tunnel::Set_Destination |
| 18 | `0x728FE0` | Tunnel::Stop_Moving_Full |
| 29 | `0x72A1A0` | Tunnel::In_Which_Layer (UNANALYZED) — likely returns **1 (Underground)** but unverified |
| 31 | `0x728A60` | (unanalyzed) |
| 32 | `0x72A1C0` | Tunnel::Is_Moving_Now |
| 36-37 | `0x4B4C60/70` | shared with Drive/Ship (Begin/End_Piggyback) |
| 38 | `0x72A1E0` | Tunnel::Is_Surfacing — possibly NOT shared with Drive/Ship; Tunnel may override (surface/submerge state) |
| 39 | `0x4B6620` | shared with DropPod |

**Subtle detail — slot 38 (`Is_Surfacing`):** Drive and Ship share `0x4B4C80`. Tunnel uses `0x72A1E0`. This makes sense — surfacing is a Tunnel-specific concept (underground unit emerging). Drive/Ship inherit a stub that returns false.

### 4.4 Verdict

**TunnelLocomotionClass is TS legacy, dormant in YR.** Tiberian Sun had subterranean units (Devil's Tongue, Subterranean APC, Burrower). YR/RA2 has NO subterranean units. The class is fully implemented in the binary but never instantiated.

**Do NOT implement in Rust.**

---

## 5. Summary table — all three at a glance

| Class | CLSID | Constructor | INI refs | Status |
|---|---|---|---|---|
| Mech | `{55D141B8-DB94-11D1-AC98-006008055BB5}` | `0x5AFEF0` | 8 historical-comment refs (all commented or annotated) | DEFERRED-TS |
| DropPod | `{4A582745-9839-11D1-B709-00A024DDAFD1}` | `0x4B5AB0` | **0 refs** | DEFERRED-TS |
| Tunnel | `{4A582743-9839-11D1-B709-00A024DDAFD1}` | `0x728A00` | **0 refs** | DEFERRED-TS |

All three:
- Have full vtables with class-specific method addresses populated
- Have only their constructors analyzed by Ghidra (everything else is dead code)
- Have class factories registered in `WinMain` but no `CoCreateInstance` callers
- Have NullCoord sentinel globals (all zero at runtime, BSS-initialised)

---

## 6. Implications for Rust port

**Do NOT implement these locomotors.** Per the parity bar (`CLAUDE.md`):
- Parity is on observable behaviour.
- These classes produce zero observable behaviour in standard YR skirmish (they're never instantiated).
- Skipping them is parity-correct.

**If a mod activates one of them:** The Rust port would need to add the corresponding locomotor implementation. But this is a mod-support concern, not a parity concern. The current scope of this engine targets standard YR content.

**Index entry:** All three classes are marked `DEFERRED-TS` in `INDEX_PATHFINDING_LOCOMOTION.md`, pointing here.

---

## 7. Sources

**Ghidra functions decompiled / disassembled:**
- `MechLocomotionClass::Constructor` @ `0x005AFEF0` (full asm + decomp)
- `MechLocomotionClass::Constructor` @ `0x005B1B50` (destructor variant — full decomp)
- `DropPodLocomotionClass::Constructor` @ `0x004B5AB0` (full asm + decomp)
- `TunnelLocomotionClass::Constructor` @ `0x00728A00` (full asm + decomp)

**Memory reads (vtable contents):**
- `0x007EDB6C` len 160 (Mech ILocomotion vtable)
- `0x007E8278` len 160 (DropPod ILocomotion vtable)
- `0x007F5A24` len 160 (Tunnel ILocomotion vtable)
- `0x005B1960` len 48 (Mech vtable thunk region for QueryInterface verification)

**Xref tables:**
- `get_xrefs_to 0x7E9AA0` (Mech CLSID) → 2 (WinMain + Mech QI internal)
- `get_xrefs_to 0x5AFEF0` (Mech constructor) → 1 (class-factory CreateInstance @ `0x6C4DCC`)
- `search_byte_patterns` confirmed CLSID byte locations: Mech @ `0x7E9AA0`, DropPod @ `0x7E9A70`, Tunnel @ `0x7E9A50`

**INI files cross-referenced:**
- `ini/rules.ini` (8 Mech-CLSID lines, all commented or annotated; 0 DropPod, 0 Tunnel)
- `ini/rulesmd.ini` (same pattern)

**Companion docs:**
- `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` §0 (confirms DropPod/Tunnel TS-dead)
- `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` (for the live-class comparison pattern)
- `LOCOMOTION_MATH_AND_CONSTANTS.md` (CLSID GUID list)

**Memory references:**
- [[feedback_no_tunnel_subterranean]] — user policy: Tunnel is TS legacy, skip
- [[feedback_research_confidence_axes]] — 3-axis confidence applied
- [[feedback_caller_trace_before_finding]] — caller traces via `get_xrefs_to` and `get_function_callers`

---

*End of report. All three classes confirmed dormant. No further investigation warranted unless a mod activates one of them.*
