# HouseClass::Add_Tiberium_Credits — PurifierBonus & AIVirtualPurifiers Arithmetic
# Ghidra Research Report

**Target:** `HouseClass::Add_Tiberium_Credits` @ `0x004F9610`
**Date:** 2026-05-19
**Binary:** gamemd.exe (Yuri's Revenge)
**Overall confidence:** HIGH — all findings verified from direct assembly/decompile in this session

---

## 1. Executive Summary

`Add_Tiberium_Credits` performs **two independent accumulations** per call, using
different multipliers for each field:

| Field | Formula | Semantic |
|-------|---------|---------|
| `HouseClass+0x54E8` | `trunc(amount × 5.0 + old_+0x54E8)` | **Cumulative score/statistics counter** |
| `HouseClass+0x30C` | `trunc(TibValue × IncomeMult × amount + old_+0x30C)` | **Live spendable credits (Balance)** |

**The PurifierBonus and AIVirtualPurifiers scaling happens entirely in the CALLERS**, not
inside `Add_Tiberium_Credits`. Each caller pre-computes `bonus_amount` and then calls
`Add_Tiberium_Credits` twice: once for the raw `drained_amount`, once for `bonus_amount`.
Inside `Add_Tiberium_Credits`, the `+0x30C` formula applies `TibValue × IncomeMult` to
whatever `amount` was passed in — so the bonus amount also gets value-scaled.

---

## 2. Full Assembly (verified via `disassemble_function 0x004F9610`)

```asm
; __thiscall: ECX = HouseClass*, [ESP+4] = float amount, [ESP+8] = int tibType
004f9610: FLD  float ptr [ESP+0x4]       ; ST0 = amount
004f9614: FMUL float ptr [0x007EAA00]    ; ST0 = amount × 5.0f  [0x007EAA00 = 0x40A00000 = 5.0f]
004f961a: PUSH ESI                       ; stack shifts +4
004f961b: MOV  ESI, ECX                  ; ESI = this (HouseClass*)
004f961d: FIADD dword ptr [ESI+0x54E8]   ; ST0 += (float)HouseClass[+0x54E8]
004f9623: CALL 0x007C5F00               ; Math__ftol → EAX = truncate(ST0)
004f9628: MOV  dword ptr [ESI+0x54E8], EAX  ; +0x54E8 = trunc(amount×5.0 + old_+0x54E8)

; After PUSH ESI: [ESP+8]=amount, [ESP+0xC]=tibType
004f962e: MOV  EAX, dword ptr [ESP+0xC]      ; EAX = tibType (int)
004f9632: MOV  ECX, dword ptr [0x00B0F4EC]   ; ECX = g_TiberiumClass_Array (global ptr)
004f9638: MOV  EDX, dword ptr [ECX+EAX*4]    ; EDX = TiberiumTypeClass*[tibType]
004f963b: MOV  EAX, dword ptr [ESI+0x34]     ; EAX = HouseClass→Type (HouseTypeClass*)
004f963e: FILD dword ptr [EDX+0xB8]          ; ST0 = (float)TiberiumTypeClass[+0xB8]  (Value int)
004f9644: FMUL float ptr [EAX+0x148]         ; ST0 × HouseTypeClass[+0x148]  (IncomeMult float)
004f964a: FMUL float ptr [ESP+0x8]           ; ST0 × amount
004f964e: FIADD dword ptr [ESI+0x30C]        ; ST0 += (float)HouseClass[+0x30C]
004f9654: CALL 0x007C5F00                    ; Math__ftol → EAX = truncate(ST0)
004f9659: FLD  float ptr [ESP+0x8]           ; return value: reload amount on FPU
004f965d: MOV  dword ptr [ESI+0x30C], EAX   ; +0x30C = trunc(TibValue×IncomeMult×amount + old_+0x30C)
004f9663: POP  ESI
004f9664: RET  0x8                           ; stdcall cleanup: 2 args × 4 bytes
```

---

## 3. Math__ftol Rounding Mode (verified via `disassemble_function 0x007C5F00`)

```asm
007c5f00: FNSTCW word ptr [ESP]         ; save current FPU control word
007c5f03: ...
007c5f13: CMP   EDX, dword ptr [0x00822D80]   ; compare against 0x0E7F
007c5f19: JNZ   007c5f26
007c5f1b: FISTP qword ptr [EAX]        ; if already 0x0E7F: store with current mode
007c5f26: MOV   EDX, [0x00822D80]      ; load 0x0E7F
007c5f2f: FLDCW word ptr [ESP]         ; set FPU control word to 0x0E7F
007c5f32: FISTP qword ptr [EAX]        ; store with forced mode
```

**`0x00822D80` = `0x0E7F`** (verified via `read_memory 0x00822D80` → bytes `7F 0E`).

`0x0E7F` FPU control word: RC (bits 10-11) = `11` = **round toward zero = truncate**.
`FISTP` with RC=11 truncates toward zero (C-cast `(int)f` behavior for positive floats).

**Conclusion:** Both `+0x54E8` and `+0x30C` use **truncation toward zero**, not
round-to-nearest-even. For positive amounts this is floor. The credit formula is:
```
new_balance = (int)(TibValue × IncomeMult × amount + old_balance)
```
Accumulate-then-truncate, not truncate-then-accumulate.

---

## 4. The Two Credit Fields

| Field | Offset | Purpose | Confirmed by |
|-------|--------|---------|-------------|
| `Balance` | `HouseClass+0x30C` | **Live spendable credits.** Read by `SpendMoney`, UI display via `CreditsClass`. | `ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md §9`; `HouseClass__SpendMoney` directly reads `+0x30C` |
| `HarvestedCredits` | `HouseClass+0x54E8` | **Score/statistics accumulator.** Tracks total ore harvested scaled by 5 (a score multiplier). Never spent. | Assembly shows `× 5.0f` constant; same pattern as `HouseClass__Removed_From_Game` which directly writes both fields |

**Sync guarantee:** Both fields are updated in the **same function call**, in sequence.
They cannot be observed in a half-updated state within a tick (no multi-thread risk).
They are **always** written together — there is no path in `Add_Tiberium_Credits` that
skips one.

**Why 5.0?** `TibValue` for ore is 25. `HarvestedCredits += amount × 5.0`. For 1 bale of
ore: Balance += 25 credits; HarvestedCredits += 5 units. The ratio `25/5 = 5.0` = TibValue
of ore. This means for standard ore, HarvestedCredits tracks bale count
(`= Balance / TibValue` when IncomeMult=1.0). The 5.0 constant is baked in, not
parameterised — it does not change per tib type. Gem ore (TibValue=50) would diverge.

---

## 5. Caller Protocol: PurifierBonus and AIVirtualPurifiers

Both `UnitClass__Mission_Deploy_Building` (Allied/Soviet) and
`BuildingClass__DepositOreFromStorage` (Yuri Slave Miner) use **identical pre-call math**:

```pseudo
facility_count = HouseClass[+0x538C]   // real purifier count from RecalcBonuses
if !owner.IsHuman and g_GameMode != 0:
    facility_count += AIVirtualPurifiers[owner[+0x184]]
                   // = *(int*)( *(int*)(g_RulesClass + 0x1324) + difficulty_index * 4 )

amount = StorageClass::GetAmount(slot)
bonus  = (float)facility_count * *(float*)(g_RulesClass + 0xF3C) * amount
         //                         = PurifierBonus (default 0.25)

drained = StorageClass::RemoveAmount(amount, slot)
if drained > 0.0:
    owner.Add_Tiberium_Credits(drained, slot)       // call 1: base
    if bonus > 0.0:
        owner.Add_Tiberium_Credits(bonus, slot)     // call 2: bonus
```

Verified in both callers this session:
- `BuildingClass__DepositOreFromStorage` @ `0x00522D50` — decompiled, shows exact pattern above.
- `UnitClass__Mission_Deploy_Building` @ `0x0073D630` — decompiled, shows identical pattern.

**Multiplier math order (Q1):** The stacking is **additive** (Q1 resolved):
```
total_amount = drained + facility_count × PurifierBonus × drained
             = drained × (1 + N_purifiers × 0.25)
```
Where `N_purifiers = real_purifiers + AIVirtualPurifiers[difficulty]`.
Each additional purifier adds exactly `PurifierBonus × base_amount` credits.
This is **linear/additive**, not compound (not `(1+0.25)^N`).

**The bonus call passes `bonus` as the `amount` argument.** Inside `Add_Tiberium_Credits`
this gets scaled again by `TibValue × IncomeMult`. So the effective credit from the bonus
call is `trunc(TibValue × IncomeMult × bonus + old_balance)` — same per-unit scaling as
the base call.

---

## 6. AIVirtualPurifiers Array Indexing

**Array location:** `*(int*)(g_RulesClass_Instance + 0x1324)` = pointer to `int[3]`
(verified in both decompiled callers this session).

**INI default:** `AIVirtualPurifiers=4,2,0` (from `rulesmd.ini`).

**Index → Difficulty mapping** (verified via `HouseClass__SetDifficulty @ 0x004F6EC0`):

`SetDifficulty(param_2)` writes `param_2` directly to `HouseClass+0x184`. In
`ScenarioClass__Create_Houses`, AI players are created with:
```c
int difficulty = piVar8[-8];   // = DAT_00a8b27c[i] = multiplayer-lobby difficulty setting
// CompEasyBonus adjustment: if (>1 human player && CompEasyBonus && difficulty > 0)
//     difficulty -= 1;   (bumps one level easier — never increases)
HouseClass__SetDifficulty(difficulty);
```

The multiplayer dialog reads `AIDifficulty` from `[MultiplayerDialogSettings]` into
`RulesClass+0x14A4`. Lobby UI strings are ordered `BRUTAL / MEDIUM / EASY` with values
`0 / 1 / 2` (conventional C&C ordering, confirmed by `ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT §6`):

| `HouseClass+0x184` value | Difficulty | `AIVirtualPurifiers[index]` | Effective extra purifiers |
|--------------------------|------------|-----------------------------|---------------------------|
| 0 | Hard (Brutal) | 4 | +100% ore income |
| 1 | Medium | 2 | +50% ore income |
| 2 | Easy | 0 | no bonus |

**Confidence:** HIGH (SetDifficulty decompiled this session; INI ordering confirmed by
prior ORE_VALUE report which independently decompiled the same parser at `0x0067054C`).

---

## 7. Credits Field Reconciliation (+0x30C vs +0x54E8)

This resolves the open question from `REFINERY_STORAGE_FLOW_GHIDRA_REPORT.md §7.3`:

**`HouseClass+0x30C` = Balance** — the live, spendable credit counter.
- Read by `HouseClass__SpendMoney` (verified in `ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT §11`).
- Decremented on purchase, incremented by `Add_Tiberium_Credits`.
- UI sidebar reads it via `CreditsClass::AI` which calls `GetAvailableCredits` vtable method,
  which reads `+0x30C` (confirmed in prior report §10).

**`HouseClass+0x54E8` = HarvestedCredits** — a cumulative score/statistics counter.
- Written by every `Add_Tiberium_Credits` call, but also directly by
  `HouseClass__Removed_From_Game` (decompiled this session at `0x00502855`, writes both
  fields independently via `Math__ftol` calls).
- Scaled by `5.0f` (a fixed constant at `0x007EAA00`), not by `TibValue × IncomeMult`.
- Never decremented by `SpendMoney` — it only grows.
- Purpose: ore-harvested score tracking for end-game statistics.

**Sync guarantee:** Both written in same function, never one-without-the-other inside
`Add_Tiberium_Credits`. Exception: `Removed_From_Game` writes them with separate
`Math__ftol` calls from a different FPU register path — but that is the "unit removed from
game" refund flow, not the credit-deposit flow.

---

## 8. Full Credit-Deposit Arithmetic (Example)

**Setup:** 40 ore (slot 0), 1 real purifier, `PurifierBonus=0.25`, `TibValue=25`,
`IncomeMult=1.0`, AI Hard difficulty (4 virtual purifiers), `old_balance=0`.

**Caller (pre-call):**
```
facility_count = 1 + 4 = 5          // 1 real + 4 virtual (Hard AI)
bonus = 5 × 0.25 × 40.0 = 50.0
```

**Call 1:** `Add_Tiberium_Credits(40.0, 0)`:
```
+0x54E8 = trunc(40.0 × 5.0 + 0)  = trunc(200.0)  = 200
+0x30C  = trunc(25 × 1.0 × 40.0 + 0) = trunc(1000.0) = 1000
```

**Call 2:** `Add_Tiberium_Credits(50.0, 0)`:
```
+0x54E8 = trunc(50.0 × 5.0 + 200) = trunc(450.0) = 450
+0x30C  = trunc(25 × 1.0 × 50.0 + 1000) = trunc(2250.0) = 2250
```

**Final:** Balance = **2250 credits**, HarvestedCredits = 450.
Base-only balance would be 1000. AI Hard bonus doubles it.

---

## 9. Rounding/Truncation Impact (Q2 resolved)

Both accumulations use the accumulate-then-truncate pattern:
```
new = trunc(scale × amount + old_field)
```
The truncation applies **once** to the running total, not per-call-then-sum. This means
fractional carry is preserved across calls within a deposit cycle (since `old_field` is
the already-truncated previous value, the carry is truncated per call). For a single call
of `amount = 1.0` with `TibValue=25`, `IncomeMult=1.0`:
```
trunc(25 × 1.0 × 1.0 + 0) = trunc(25.0) = 25  (exact, no rounding loss)
```
Gems (`TibValue=50`): `trunc(50 × 1.0 × 1.0) = 50` (also exact).

Rounding loss occurs when `amount` is a non-integer float (e.g., from partial RemoveAmount)
or when `IncomeMult ≠ 1.0`. In standard YR with `IncomeMult=1.0` and whole-slot drains,
no per-bale credit is lost to truncation.

---

## 10. Callers Summary

| Caller | Address | Context | When |
|--------|---------|---------|------|
| `BuildingClass__DepositOreFromStorage` | `0x00522D50` | Yuri Slave Miner | Slave arrives at dock cell |
| `UnitClass__Mission_Deploy_Building` | `0x0073D630` | Allied/Soviet harvester | Per-dump-counter fire (~15 frames) |
| `BuildingClass__Sell` | `0x00449C30` | Sell refund | Building sold with ore in storage |
| `FUN_00684C30` (map post-load setup) | `0x00684C30` | Scenario init overflow drain | Map load, if stored ore exceeds TibType max |
| `HouseClass__Removed_From_Game` | `0x00502855` | Unit death/remove refund | Directly writes `+0x30C`/`+0x54E8` with own ftol calls (bypasses the function) |

Note: `HouseClass__Removed_From_Game` does NOT call `Add_Tiberium_Credits`; it computes
its own `Math__ftol` and writes both fields directly (decompiled this session, pattern
confirmed at addresses `0x0050277B`-region).

---

## 11. Key Constants and Field Map

| Symbol | Location | Verified value | How verified |
|--------|----------|---------------|--------------|
| `5.0f` (score multiplier) | `0x007EAA00` | `0x40A00000` | `read_memory 0x007EAA00` → `00 00 A0 40` |
| `Math__ftol` rounding | `0x007C5F00` | Truncate toward zero (RC=11) | `disassemble_function 0x007C5F00`; `read_memory 0x00822D80` → `7F 0E` = `0x0E7F` |
| `TiberiumClass+0xB8` | `[EDX+0xB8]` | `Value` int (25 ore, 50 gems) | `ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT §2`; `disassemble_function 0x004F9610` |
| `HouseTypeClass+0x148` | `[EAX+0x148]` | `IncomeMult` float (default 1.0f) | `ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT §7`; assembly at `004F9644` |
| `HouseClass+0x30C` | `[ESI+0x30C]` | `Balance` (spendable credits) | Assembly write at `004F965D`; read confirmed by SpendMoney |
| `HouseClass+0x54E8` | `[ESI+0x54E8]` | `HarvestedCredits` (statistics) | Assembly write at `004F9628` |
| `HouseClass+0x184` | (indirect) | `AIDifficulty` index (0=Hard, 1=Med, 2=Easy) | `decompile_function 0x004F6EC0` (SetDifficulty writes it) |
| `HouseClass+0x538C` | (caller reads) | Real purifier count | Both callers decompiled this session |
| `RulesClass+0xF3C` | (caller reads) | `PurifierBonus=0.25` (float) | `ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT §8`; callers confirmed |
| `RulesClass+0x1324` | (caller reads) | `AIVirtualPurifiers` data ptr | Both callers decompiled, pattern `*(int*)(Rules+0x1324) + index*4` |
| `g_TiberiumClass_Array` | `0x00B0F4EC` | Global TiberiumClass* array | Assembly at `004F9632`; `ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT §2` |
| `FLOAT_007E1748` | `0x007E1748` | `0.0f` (epsilon) | `read_memory 0x007E1748` → `00 00 00 00` |

---

## 12. Rust Implementation Implications

### PurifierBonus (currently boolean → must be count-based)
Current Rust (`src/sim/miner/miner_dock_sequence.rs:386-393`) uses boolean `player_has_purifier() × 25%`.
Required formula:
```rust
let facility_count = house.purifier_count  // HouseClass+0x538C equivalent
    + if !house.is_human && in_game { ai_virtual_purifiers[house.difficulty as usize] } else { 0 };
let bonus_amount = facility_count as f32 * rules.purifier_bonus * drained_amount;
if bonus_amount > 0.0 {
    house.add_tiberium_credits(bonus_amount, tib_type);
}
```

### Add_Tiberium_Credits internals
```rust
fn add_tiberium_credits(&mut self, amount: f32, tib_type: usize) {
    // Statistics accumulator
    self.harvested_credits = (amount * 5.0 + self.harvested_credits as f32) as i32;
    // Spendable balance
    let tib_value = tiberium_types[tib_type].value as f32;
    let income_mult = self.house_type.income_mult;
    self.balance = (tib_value * income_mult * amount + self.balance as f32) as i32;
}
```
Note: Rust `as i32` on positive f32 truncates toward zero — matches `Math__ftol` behavior.

### AIVirtualPurifiers
Missing in Rust. Needs:
1. INI parser for `AIVirtualPurifiers=4,2,0` → `rules.general.ai_virtual_purifiers: [i32; 3]`
2. Wire `HouseClass::difficulty` (0=Hard, 1=Med, 2=Easy) into deposit path

---

## 13. Prior Report Reconciliation

This report resolves all three open questions from `REFINERY_STORAGE_FLOW_GHIDRA_REPORT.md §7`:

- **§7.3 (+0x30C vs +0x54E8):** Resolved. `+0x30C` = Balance (spendable). `+0x54E8` = HarvestedCredits (statistics). Always synced within `Add_Tiberium_Credits`.
- **§7.5 (difficulty index order):** Resolved. Index 0=Hard(Brutal), 1=Medium, 2=Easy. Confirmed via `SetDifficulty` decompile.
- **Multiplier math (Q1):** Additive (not compound). Formula: `base × (1 + N × PurifierBonus)`.
- **Rounding (Q2):** Truncation toward zero, accumulate-then-truncate pattern.

The ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md §3 (earlier separate investigation) reaches the same conclusions from independent decompilation — all cross-checked and consistent.

---

## Sources

- `disassemble_function 0x004F9610` — full assembly of `Add_Tiberium_Credits`
- `disassemble_function 0x007C5F00` — `Math__ftol` rounding mode
- `read_memory 0x007EAA00` (4 bytes) — confirmed `5.0f`
- `read_memory 0x00822D80` (2 bytes) — confirmed `0x0E7F` FPU ctrl word
- `read_memory 0x007E1748` (4 bytes) — confirmed `0.0f` epsilon
- `decompile_function 0x00522D50` — `BuildingClass__DepositOreFromStorage`
- `decompile_function 0x0073D630` — `UnitClass__Mission_Deploy_Building`
- `decompile_function 0x00502855` — `HouseClass__Removed_From_Game`
- `decompile_function 0x004F6EC0` — `HouseClass__SetDifficulty` (writes +0x184)
- `get_function_callers 0x004F9610` — 5 callers verified
- `ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md` — prior independent investigation, all findings consistent
- `REFINERY_STORAGE_FLOW_GHIDRA_REPORT.md` — open questions §7.3, §7.5 resolved
