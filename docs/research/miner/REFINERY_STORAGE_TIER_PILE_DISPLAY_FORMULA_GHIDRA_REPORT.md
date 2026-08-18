# Refinery Storage-Tier Ore-Pile Display Formula — Ghidra Research Report

**Address:** `0x004509D0` — `BuildingClass::UpdateAnimation` (Phase F, Refinery branch)
**Key address range:** `0x00450D96 – 0x00450F9E` (Phase F, gated on `Type+0x16BB`)
**Cited swarm sites:** `0x00450E0D` = `ClearAnimSlot(prior_tier_slot)` call; `0x00450F99` = `CreateAnimForSlot(new_tier_slot)` call
**Companion Phase:** Phase E (SiloDamage, `0x00450CB7 – 0x00450D96`) — slot-10 SpecialAnim retain logic
**Confidence:** HIGH — full disassembly + decompile of `0x004509D0` read directly this session
**Active in YR:** Yes — Phase F fires on every building with `Refinery=yes`; Phase E fires on every building with `SiloDamage=yes`

---

## 1. Overview

`BuildingClass::UpdateAnimation` (called unconditionally every tick from `BuildingClass::Update @ 0x0043FE22`) contains
two separate tier-display systems that both manipulate the slot-10 (`SpecialAnim`) and slots 3–6 (`ActiveAnim*`) anim
handles. This report covers:

- **Phase F (0x00450D96–0x00450F9E):** Refinery ore-pile display — swaps one of slots 3/4/5/6 based on
  `(stored × 4) / Storage` tier formula. Gate: `Type+0x16BB` (`Refinery=yes`).
- **Phase E (0x00450CB7–0x00450D96):** SiloDamage fill-level indicator — creates/clears slot-10 SpecialAnim
  and writes tier (1..3) into `anim+0xAC`. Gate: `Type+0x16A8` (`SiloDamage=yes`).
- **Slot-10 retain logic for refinery dock pulse:** `BuildingClass+0x584` holds the active slot-10 SpecialAnim
  pointer. `Mission_Deploy_Building` checks `building+0x584 == 0` before firing the per-bale SpecialAnim
  (SetAnimSlotImage slot 10). Because `SiloDamage=` is false on all stock refineries, `building+0x584` is always 0
  when a harvester arrives, so the dock-pulse SetAnimSlotImage(10) always fires unobstructed.

---

## 2. Tier Formula — Phase F (Refinery, slots 3–6)

### Exact decompile (from `0x004509D0`, verified via disassembly this session)

```
// Gate: Type+0x16BB != 0  (Refinery=yes)
// Address: 0x00450D96–0x00450F9E

StoredAmount = StorageClass::GetTotalAmount(&this->StorageClass);   // float → ftol → int
if (StoredAmount == 0) {
    new_tier = 0;
} else {
    StoredAmount2 = StorageClass::GetTotalAmount(&this->StorageClass);  // called TWICE
    iVar6 = (StoredAmount2 << 2) / *(int *)(this->Type + 0x800);       // (stored*4) / Storage
    // NO clamping here (cf Phase E which does clamp)
    new_tier = iVar6;   // raw result, can exceed 3 if overflow
}

cached_tier = *(int *)&this->field_0x6f0;      // BuildingClass+0x6F0

if (cached_tier != new_tier) {
    // Clear prior tier's slot:
    // switch(cached_tier): >=3→push 6, >=2→push 5, >=1→push 4, >=0→push 3, <0→skip
    // 0x00450E0D: CALL ClearAnimSlot(prior_slot)
    ClearAnimSlot(prior_tier_slot);
    
    // Recompute new_tier again (GetTotalAmount called a THIRD time):
    StoredAmount3 = GetTotalAmount();
    new_tier = (StoredAmount3 == 0) ? 0 : (StoredAmount3 << 2) / Type[+0x800];
    
    *(int *)&this->field_0x6f0 = new_tier;     // cache updated
    
    // Select and create new tier slot:
    // >=3 → slot 6, type offsets +0x10E4 (undamaged) / +0x10F4 (damaged)
    // ==2 → slot 5, type offsets +0x10A0 / +0x10B0
    // ==1 → slot 4, type offsets +0x105C / +0x106C
    // ==0 → slot 3, type offsets +0x1018 / +0x1028
    // <0  → LAB_00450F9E (skip, no create)
    
    // 0x00450F99: CALL CreateAnimForSlot(slot, anim_name, isDamaged, 0, 0)
    CreateAnimForSlot(slot=new_tier+3, anim_name, isDamaged=health<=ConditionYellow, 0, 0);
}
```

### Verified disassembly landmarks (this session, from `disassemble_function 0x004509D0`):

| Address | Instruction | Meaning |
|---------|-------------|---------|
| `0x00450D96` | `MOV AL, byte ptr [EDX + 0x16BB]` | Phase F gate: check `Refinery=yes` |
| `0x00450DA4` | `JZ 0x00450F9E` | Skip if not a refinery |
| `0x00450DB0` | `MOV ECX, EDI; CALL 0x006C9650` | `StorageClass::GetTotalAmount` call #1 |
| `0x00450DB7` | `CALL 0x007C5F00` | `Math::ftol` — float→int |
| `0x00450DBC` | `TEST EAX, EAX; JZ 0x00450DDC` | if stored==0, skip multiplication |
| `0x00450DC0` | `MOV ECX, EDI; CALL 0x006C9650` | `StorageClass::GetTotalAmount` call #2 |
| `0x00450DC7` | `CALL 0x007C5F00` | ftol #2 |
| `0x00450DD2` | `SHL EAX, 0x2` | `stored * 4` (left-shift by 2) |
| `0x00450DD5` | `CDQ` | sign-extend for signed divide |
| `0x00450DD6` | `IDIV dword ptr [ECX + 0x800]` | `/ Storage` — signed integer division |
| `0x00450DDE` | `MOV ECX, dword ptr [ESI + 0x6F0]` | load cached_tier |
| `0x00450DE2` | `CMP ECX, EAX; JZ 0x00450F9E` | if tier unchanged, skip |
| `0x00450DEA` | `CMP ECX, 0x3; JL …` | switch: which prior slot to clear |
| `0x00450E0D` | `CALL 0x00451E40` | `ClearAnimSlot(prior_slot)` ← CITED ADDRESS |
| `0x00450E12` | `MOV ECX, EDI; CALL 0x006C9650` | `GetTotalAmount` call #3 |
| `0x00450E19` | `CALL 0x007C5F00` | ftol #3 |
| `0x00450E34` | `SHL EAX, 0x2; CDQ; IDIV [ECX+0x800]` | tier formula re-computed |
| `0x00450E41` | `MOV dword ptr [ESI + 0x6F0], EAX` | write new_tier to cache |
| `0x00450F99` | `CALL 0x00451890` | `CreateAnimForSlot(slot, …)` ← CITED ADDRESS |
| `0x00450F9E` | Phase G starts | end of Phase F |

---

## 3. Tier Formula Summary

**Formula:** `tier = floor((stored * 4) / Storage)` — signed integer arithmetic (CDQ/IDIV)

**Tier→Slot mapping:**

| Tier value | Slot | INI key (undamaged) | INI key (damaged) | GAREFN animation |
|------------|------|---------------------|-------------------|------------------|
| `< 0` | no slot (skip) | — | — | — |
| `0` (0%–<25%) | 3 | `ActiveAnim` (+0x1018) | `ActiveAnimDamaged` (+0x1028) | `GAREFNL1` |
| `1` (25%–<50%) | 4 | `ActiveAnimTwo` (+0x105C) | `ActiveAnimTwoDamaged` (+0x106C) | `GAREFNL2` |
| `2` (50%–<75%) | 5 | `ActiveAnimThree` (+0x10A0) | `ActiveAnimThreeDamaged` (+0x10B0) | `GAREFNL3` |
| `≥ 3` (75%–100%+) | 6 | `ActiveAnimFour` (+0x10E4) | `ActiveAnimFourDamaged` (+0x10F4) | `GAREFNL4` |

**No upper clamp in Phase F.** Values `> 3` all route into the `>= 3` branch. The SiloDamage (Phase E) version
applies an explicit clamp to `[0, 3]` via `CMP EAX, 0x3 / MOV EDI, 0x3 / JG` — Phase F does not.

**Signed division.** `CDQ; IDIV [ECX+0x800]` — the stored amount is a float, converted via `ftol` to a
signed int before the shift+divide. A negative stored amount (shouldn't happen in valid play) would produce
a negative tier and hit the `< 0 → skip` path.

**Zero-stored special path:** The code tests `StoredAmount == 0` (JZ at `0x00450DBE`) and jumps to the
`new_tier = 0` path WITHOUT multiplying or dividing. This prevents a divide by zero if Storage=0 (though
in practice `Storage=200` for stock refineries). It also means `stored=0` maps directly to tier 0 without
going through the formula.

**Important: GetTotalAmount is called THREE times per tier-change tick.** Two calls compute whether the tier
changed; if it has, a third call re-derives the tier and commits it to `+0x6F0`. All three happen within the
same tick. No intermediate state.

---

## 4. Tier-0 Hidden State (slot 3 always shows, never clears)

**Tier 0 is NOT hidden.** When `stored == 0`, tier = 0 → slot 3 (`ActiveAnim` = `GAREFNL1`) is created.
There is no special "slot 3 is the empty state" clear path in Phase F — at tier 0, `CreateAnimForSlot(3, …)`
is called with the `ActiveAnim` name. The ore pile stays visible (showing the empty fill-level art) even when
the refinery holds zero ore.

This is **important for stock refineries**: in standard Allied/Soviet play the refinery's own `StorageClass`
is never written by the harvester path (harvesters drain directly to credits, not through the refinery's
StorageClass). So GAREFN/NAREFN always show the tier-0 pile (`GAREFNL1`). The tier only changes if the
refinery's own StorageClass gets filled — which only happens in the Yuri Slave Miner path.

---

## 5. Phase E — SiloDamage (slot 10, `Type+0x16A8`)

### INI key confirmed

From `BuildingTypeClass::ReadINI @ 0x00461170` (large function, decompiled and searched):

```c
uVar4 = CCINIClass__ReadBool(iVar15, s_SiloDamage_0081a780, *(unsigned1 *)(param_1 + 0x16a8));
*(unsigned1 *)(param_1 + 0x16a8) = uVar4;
```

**Verified:** `Type+0x16A8` = `SiloDamage=` (bool). String at `0x0081A780`. XRef at `0x00461170`.

### Phase E logic (0x00450CB7–0x00450D96)

```
// Gate: Type+0x16A8 != 0  (SiloDamage=yes)
// Address range: 0x00450CB7–0x00450D96

if (*(int *)(Type + 0x800) < 1) {
    iVar9 = 0;   // Storage capacity = 0 → tier forced to 0
} else {
    stored = GetTotalAmount(...);
    stored_int = ftol(stored);
    stored_int <<= 2;
    // Phase E stores intermediate in local:
    local_val = stored_int;
    // Second ftol call on the float result of the divide:
    tier_float = (float)local_val / (float)Type[+0x800];   // float divide
    tier_raw = ftol(tier_float + epsilon);
    if (tier_raw < 0) {
        iVar9 = 0;   // clamp to 0
    } else if (tier_raw > 3) {
        iVar9 = 3;   // clamp to 3
    } else {
        iVar9 = tier_raw;
    }
}

if (iVar9 == 0) {
    if (building+0x584 != 0) {
        ClearAnimSlot(10);   // clear slot 10 if it exists
    }
    // slot 10 remains NULL after clear
} else {
    // tier 1, 2, or 3
    if (building+0x584 == 0) {
        // slot 10 is empty — create the SpecialAnim
        anim_name = (health <= ConditionYellow) ? Type+0x1204 : Type+0x11F4;
        if (anim_name != null && *anim_name != '\0') {
            CreateAnimForSlot(10, anim_name, isDamaged, 0, 0);
        }
    }
    // Regardless of whether we just created it, write tier to anim+0xAC:
    *(int *)(building+0x584 + 0xAC) = iVar9;
}
```

### Disassembly landmarks for Phase E (verified this session):

| Address | Instruction | Meaning |
|---------|-------------|---------|
| `0x00450CBD` | `MOV AL, byte ptr [ECX + 0x16A8]` | `SiloDamage=` gate ← CITED ADDRESS |
| `0x00450CC5` | `JZ 0x00450D96` | skip if SiloDamage=no |
| `0x00450CCB` | `MOV EDX, [ECX + 0x800]; TEST EDX, EDX; JLE 0x00450D1B` | storage cap == 0 check |
| `0x00450D09` | `TEST EAX, EAX; JGE 0x00450D11; XOR EDI, EDI` | clamp negative → 0 |
| `0x00450D11` | `CMP EAX, 0x3; MOV EDI, 0x3; JG 0x00450D1D` | clamp > 3 → 3 |
| `0x00450D1D` | `MOV EAX, [ESI + 0x584]; TEST EDI, EDI; JZ 0x00450D89` | if tier==0, go clear path |
| `0x00450D27` | `TEST EAX, EAX; JNZ 0x00450D7B` | if slot-10 pointer non-null, skip create |
| `0x00450D71` | `PUSH 0xA; CALL 0x00451890` | `CreateAnimForSlot(slot=10, …)` |
| `0x00450D7B` | `MOV ECX, [ESI + 0x584]; MOV [ECX + 0xAC], EDI` | write tier (1..3) to anim+0xAC |
| `0x00450D89` | (tier==0 path) `TEST EAX, EAX; JZ 0x00450D96` | if slot-10 null, skip clear |
| `0x00450D8D` | `PUSH 0xA; CALL 0x00451E40` | `ClearAnimSlot(10)` |

**Critical: Phase E creates slot 10 only once (when `building+0x584 == 0`) and then writes tier each tick
to `anim+0xAC`.** This means for a SiloDamage building, slot 10 is persistent once created (as long as
tier > 0), and `building+0x584` stays non-null. The tier value is updated via `+0xAC` every tick,
not by recreating the anim.

---

## 6. Slot-10 Retain Semantics (`building+0x584`) and Dock-Pulse Gate

`BuildingClass+0x584` is the active **slot-10 AnimClass pointer** — set by `CreateAnimForSlot(10, …)`,
cleared by `ClearAnimSlot(10)`. The Ghidra decompile of Phase E confirms both writes this session.

**Retain semantics:**
- For buildings with `SiloDamage=yes` (ore silos): slot 10 is created on the first tick where tier > 0,
  then kept alive while tier > 0, with tier updated via `anim+0xAC` each tick. It is NOT cleared then
  recreated each tick — only cleared when tier drops to 0.
- For buildings with `Refinery=yes` and `SiloDamage=no` (all stock refineries — GAREFN, NAREFN): Phase E
  never fires. `building+0x584` starts null at build time and is only written by:
  - `Mission_Deploy_Building` per-bale call: `SetAnimSlotImage(10, …)` which calls `CreateAnimForSlot(10)`.
  - The anim's own destruction when it finishes playing (one-shot SpecialAnim completes → anim pointer cleared).

**Dock-pulse gate:**
`Mission_Deploy_Building @ 0x73D630` per-bale code:
```c
if (*(int *)(building + 0x584) == 0) {
    SetAnimSlotImage(10, isDamaged, 0, 0);  // fires GAREFNOR
}
```
For stock refineries (`SiloDamage=no`), Phase E never creates a slot-10 anim. After GAREFNOR finishes
playing (~4 seconds), the pointer is cleared back to 0. So the dock-pulse `SetAnimSlotImage(10)` fires
on every bale where the previous GAREFNOR has finished — OR on the first bale of a new dock
(slot is freshly null).

**Key dependency for swarm slot 1:** The slot-1 investigation into `Type+0x16A8` vs `Type+0x16BB` is
relevant here but resolved independently: `Type+0x16A8` = `SiloDamage=` (verified from ReadINI this
session, `0x00461170`). For all stock refineries (GAREFN, NAREFN, YAREFN), `SiloDamage=` is NOT set in
either `rulesmd.ini` or `rules.ini`. Therefore Phase E never fires on refineries, and `building+0x584`
for a refinery is 0 at the moment any harvester bale fires — **the dock-pulse is not gated by Phase E**.

---

## 7. Update Frequency

Phase F runs on **every tick** (no cadence gate). It calls `GetTotalAmount` up to three times but short-
circuits after the `cached_tier != new_tier` comparison — if the tier hasn't changed (the common case),
work stops at `JZ 0x00450F9E` (`0x00450DE4`). The three `GetTotalAmount` calls are only reached on
tier-change ticks.

`BuildingClass::Update` is the sole caller of `UpdateAnimation` (verified via Ghidra xref: only caller is
`0x0043FE22` in `BuildingClass::Update`). So the visual updates on every game tick, but the slot swap only
happens when tier changes.

---

## 8. INI Data — Stock Refinery Storage Values

| Section | Key | Value | Effect |
|---------|-----|-------|--------|
| `[GAREFN]` | `Refinery=yes` | yes | Enables Phase F |
| `[GAREFN]` | `Storage=200` | 200 | Denominator in tier formula |
| `[NAREFN]` | `Refinery=yes` | yes | Enables Phase F |
| `[NAREFN]` | `Storage=200` | 200 | Denominator |
| `[YAREFN]` | `Storage=200` | 200 | Denominator |
| `[YAREFN]` | `Refinery=` | absent | Phase F NOT enabled on YAREFN |
| (any) | `SiloDamage=` | absent on all refineries | Phase E never fires on GAREFN/NAREFN/YAREFN |

Verified from `ini/rulesmd.ini` lines 11722–11760 (GAREFN) and 13234–13259 (YAREFN).

---

## 9. Current Rust Implementation Status

The Rust renderer in `src/app_instances/shp.rs` (around line 530) renders ALL `ActiveAnim*` entries that
have `loop_count < 0` unconditionally (infinite-loop path). For refineries, this means all four
`GAREFNL1..L4` anims render stacked on top of each other every frame.

**gamemd behavior:** Exactly one of slots 3/4/5/6 is active at any time (the one matching the current
storage tier). The others are null (never created, or cleared when tier changed).

**Gap:** The tier-gate logic is not implemented. Our Rust renderer has no `refinery_display_tier` state
on buildings. All four ore-pile anims render simultaneously.

**What needs to change:**
- Per-refinery state: `display_tier: u8` (0..3), updated from a refinery's "displayed stored amount"
  per tick (or per bale event since standard harvester paths don't fill the refinery StorageClass).
- Render path: only draw the `ActiveAnim*` slot matching `display_tier` (slot 3 for tier 0, 4 for tier 1, etc.).
- Note from `REFINERY_STORAGE_FLOW_GHIDRA_REPORT.md` §3c: since Allied/Soviet harvesters drain directly
  to credits (not via refinery StorageClass), the refinery's `StorageClass` stays at 0. A "display
  counter" that increments on `BaleDepositEvent` and decrements as ore is "visually consumed" is the
  correct approach for the visual tier, decoupled from the actual credit flow.

---

## 10. Tiny Details

- **StorageClass::GetTotalAmount called three times on tier-change ticks.** Two to detect change, one
  to re-derive the new tier. All three read the same StorageClass embed at `BuildingClass+0x33C`. In
  a single-threaded game sim, all three return identical values in the same tick — no observable difference.

- **Phase F uses IDIV (signed divide) for the tier formula; Phase E uses float divide + ftol.** Different
  arithmetic paths for what should be the same formula. Phase F: `(int)(stored) << 2 / Storage`
  (integer shift then signed divide). Phase E: `(float)((int)stored << 2) / (float)Storage` then `ftol`
  with epsilon addition (`FADD double ptr [0x007E1738]`, where `0x007E1738 = 0.0`... actually the epsilon
  add is 0.0, so this is effectively the same as truncation).

- **Phase F has NO tier-0 clear path for slots 3–6.** When new_tier == 0, the code still calls
  `CreateAnimForSlot(slot=3, ActiveAnim, …)` — it does NOT short-circuit or clear slot 3. So tier-0 is
  the GAREFNL1 "empty tower" visual, not a blank space.

- **Phase F clears the PREVIOUS tier slot, then immediately creates the NEW tier slot.** The clear uses
  the cached value at `building+0x6F0` (the prior tier), then the create uses the freshly computed value.
  There is no gap tick where neither slot exists.

- **`building+0x6F0` (previous_tier cache) is written BEFORE the create call** (disasm: `MOV dword ptr
  [ESI + 0x6F0], EAX` at `0x00450E41`, then `CreateAnimForSlot` at `0x00450F99`). So if `CreateAnimForSlot`
  somehow failed (null anim name), the cache is already updated — the clear happened but no slot was
  created, leaving a blank tier until the next change.

- **The `< 0` tier path in Phase F (LAB_00450F9E):** If the computed tier is negative (StoredAmount
  negative, impossible in normal play), Phase F skips both clear and create and falls through to Phase G.
  The cache at `+0x6F0` is NOT updated in this case (the write happens after the `iVar6 < 3` branch but
  at `0x00450E41` before the inner branches — looking at the disasm: `MOV dword ptr [ESI + 0x6F0], EAX`
  is at `0x00450E41` which is reached only after the re-derive calls, i.e., only on tier-change ticks where
  the old tier is being cleared. The `< 0` JL at `0x00450F4B` jumps to `0x00450F9E` skipping the
  CreateAnimForSlot only).

- **`FADD double ptr [0x007E1738]`** in Phase E formula adds `0.0` (the value at that address per
  `REFINERY_STORAGE_FLOW_GHIDRA_REPORT.md` §globals). So there is no rounding bias — pure truncation.

- **Phase E writes `anim+0xAC = tier (1..3)` unconditionally after creation.** The write at `0x00450D81`
  happens even if `building+0x584` was already non-null and no new anim was created. This means every tick
  (while SiloDamage building has tier > 0), `anim+0xAC` is re-written. This is the "frame index" field
  of the AnimClass, used by the ore silo fill indicator to select which frame of the fill animation to
  display. It's a direct per-tick frame override, not a play-once.

---

## 11. Open Questions — Final State

- `[RESOLVED] OQ-1` — Tier formula: `floor((stored × 4) / Storage)`, signed int, no upper clamp in Phase F.
  (evidence: disasm `0x00450DD2–0x00450DD6`)
- `[RESOLVED] OQ-2` — Tier-0 hidden state: tier 0 creates slot-3 (`GAREFNL1`). Pile never disappears.
  (evidence: disasm `0x00450F49–0x00450F99` — `TEST EAX, EAX; JL skip; … CreateAnimForSlot(3, …)`)
- `[RESOLVED] OQ-3` — Update frequency: every tick, but only modifies slots on tier-change.
  (evidence: `JZ 0x00450F9E` at `0x00450DE4` — early-out when cached_tier == new_tier)
- `[RESOLVED] OQ-4` — `Type+0x16A8` INI key: `SiloDamage=` (bool). Verified from ReadINI at `0x00461170`.
- `[RESOLVED] OQ-5` — `building+0x584` semantics: slot-10 AnimClass pointer. Cleared by `ClearAnimSlot(10)`,
  set by `CreateAnimForSlot(10)`. For refineries (SiloDamage=no), stays null between GAREFNOR plays.
  (evidence: disasm Phase E `0x00450D1D`, `0x00450D7B`, `0x00450D89`)
- `[RESOLVED] OQ-6` — Dock-pulse gate for refineries: `building+0x584 == 0` check in
  `Mission_Deploy_Building`. For stock refineries with `SiloDamage=no`, Phase E never sets this pointer,
  so the dock-pulse fires whenever the previous GAREFNOR one-shot has completed (pointer auto-cleared on
  anim destruction). (evidence: REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §9.2 + Phase E disasm)
- `[RESOLVED] OQ-7` — Stock refinery StorageClass: Allied/Soviet harvesters drain directly to credits,
  not through the refinery StorageClass. GAREFN/NAREFN always have stored=0 → always tier-0.
  (evidence: REFINERY_STORAGE_FLOW_GHIDRA_REPORT.md §3a)
- `[DEFERRED] OQ-8` — YAREFN has `Storage=200` but no `Refinery=yes`. Does YAREFN's StorageClass
  ever have tier > 0? Per REFINERY_STORAGE_FLOW_GHIDRA_REPORT.md §3b, Slave Miner deposits go into the
  building StorageClass before draining — so YAREFN would briefly show tier > 0 on slave deposits, but
  Phase F is never triggered (no `Refinery=yes`). Slave visual handled separately.
  (category: out-of-scope; reason: Yuri faction not yet in scope)
- `[DEFERRED] OQ-9` — Exact clamping behavior when `Type.Storage = 0` for a Refinery building. The
  Phase F code tests `stored == 0` but does not separately test `Storage < 1` (unlike Phase E which has
  `JLE 0x00450D1B`). If Storage=0, the IDIV would fault. In practice Storage=200 for all stock
  refineries; this is a modding edge case.
  (category: bounded-cost-too-high; reason: requires runtime div-by-zero trace)

---

## 12. Sources

- **Ghidra MCP `decompile_function 0x004509D0`** — full decompile, this session
- **Ghidra MCP `disassemble_function 0x004509D0`** — full disassembly, verified every address cited
- **Ghidra MCP `get_xrefs_to 0x004509D0`** — sole caller confirmed as `0x0043FE22` (BuildingClass::Update)
- **Ghidra MCP `search_strings "SiloDamage"` → `0x0081A780`**; `get_xrefs_to 0x0081A780` → `0x00461170`
  (BuildingTypeClass::ReadINI); `decompile_function 0x00461170` + string search confirmed
  `*(param_1 + 0x16a8) = ReadBool("SiloDamage")`
- **`ini/rulesmd.ini` lines 11722–11760** — GAREFN: `Storage=200`, `Refinery=yes`
- **`ini/rulesmd.ini` lines 13234–13259** — YAREFN: `Storage=200`, no `Refinery=`
- **Prior docs cross-referenced (NOT re-derived, used for context):**
  - `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md` — Phase F walkthrough (confirmed correct)
  - `REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md` — slot-10 dock-pulse gate, `building+0x584`
  - `REFINERY_STORAGE_FLOW_GHIDRA_REPORT.md` — Allied/Soviet storage flow, tier formula pseudo
- **Rust implementation reviewed:** `src/app_building_anim.rs`, `src/app_instances/shp.rs`
  (confirmed: no tier-gate logic, all ActiveAnim* slots rendered unconditionally)
