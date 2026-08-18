# Gate D2 — Damage Clamps Resolution (MaxDamage cap + minimum-damage clamp + MinDamage key)

**Status:** CLOSED. Live-verified this run against gamemd.exe in Ghidra (read-only).
**Date:** 2026-06-04
**Scope:** Resolve every clamp in the damage receive/apply path: the MaxDamage upper cap, the minimum-damage ("damage >= 1") clamps, and the `MinDamage` INI key. Companion to `DAMAGE_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (this gate pins down D6 / D7 / D14 / D17 and adds the dead `MinDamage` finding).
**Rule:** Rust-native structure, gamemd-native semantics. Default verdict for any unproven equivalence is DRIFT.

> **2026-07-13 verified correction:** 1000 is the constructor/missing-key
> fallback, not the standard stock runtime value. Both repository stock rule
> layers set `MaxDamage=10000`, and the verified reader stores that value over
> the fallback. See `DAMAGE_KERNEL_CONSTANTS_REVERIFICATION_2026-07-13.md`.

---

## Verdict

**CLOSED.** There are **four** clamps in the receive path and **one INI key that is dead**:

1. **MaxDamage upper cap** — in the warhead kernel, on kernel output, per target. Source: `[CombatDamage] MaxDamage` → `Rules+0x16C8`, constructor fallback **1000**, standard stock runtime **10000**. Signed `>=` compare.
2. **Min-1 positive floor (defender pre-pipeline)** — in `TechnoClass::ReceiveDamage`, AFTER country-armor divide and AFTER vet/elite armor divide, BEFORE immunity gates and BEFORE the Verses kernel. `if (dmg < 1) dmg = 1`.
3. **Building min-1 floor** — in `ObjectClass::ReceiveDamage`, only for `WhatAmI()==6` (building) lacking the `+0x1577` flag, applied to the post-Verses value. `if (dmg < 1) dmg = 1`.
4. **Overkill clamp** — in `ObjectClass::ReceiveDamage`, after the Verses kernel and state classify setup: `if (dmg >= currentHealth) dmg = currentHealth`. Damage never exceeds remaining HP.
5. **`MinDamage` INI key — EXISTS but is DEAD.** Parsed from `[CombatDamage] MinDamage` → `Rules+0x16C4`, but **never read** anywhere in the binary's damage/combat path. Whole-image byte scan for the +0x16C4 displacement returns only the parse store and the constructor-default write — zero reads. Do NOT implement a minimum-damage clamp from `MinDamage`.

There is **no per-warhead MaxDamage/MinDamage** and no separate Rules "global min" applied to the final number; the only minimums are the two `>= 1` floors above.

---

## (a) MaxDamage upper clamp

**Identity:** `ApplyWarheadDamage` / `WarheadTypeClass__GetDamage` @ `0x00489180` (the armor/Verses/distance kernel). Verified `decompile_function 0x00489180` + `disassemble_function 0x00489180` this run.

**Where in the chain:** last operation inside the kernel, applied to the post-Verses truncated product, **per target**. The kernel is called from `ObjectClass::ReceiveDamage` (D13) and the pre-fire estimator; the cap therefore sits AFTER falloff, AFTER Verses, and is the final value the kernel returns.

**Op (assembly, `disassemble_function 0x00489180`):**
```
00489244: CALL 0x007c5f00            ; EAX = ftol(falloff_int * Verses[armor])   (scaled)
00489249: MOV  ECX,[0x008871e0]      ; ECX = g_Rules
0048924f: MOV  ECX,[ECX+0x16c8]      ; ECX = Rules.MaxDamage
00489255: CMP  EAX,ECX
00489257: JL   0x00489265            ; if scaled < cap -> return scaled
00489259: ...   MOV EAX,ECX          ; else return cap
```
Decompile form (`decompile_function 0x00489180`):
```c
uVar3 = Math__ftol();   // scaled = ftol(falloff * Verses[armor])
if ((int)*(uint *)(g_RulesClass_Instance + 0x16c8) <= (int)uVar3) {
    return *(uint *)(g_RulesClass_Instance + 0x16c8);  // cap
}
return uVar3;
```
So the clamp is a **signed** `if (scaled >= MaxDamage) scaled = MaxDamage`. (`CMP EAX,ECX; JL` = signed less-than; the inclusive-on-equal cap follows from JL skipping when equal.)

**Value / source:** `[CombatDamage] MaxDamage`, stored at `Rules+0x16C8`. Parsed via `CCINIClass__ReadInt` (`0x005276d0`) in `RulesClass__ReadCombatDamage` (string `"MaxDamage"` @ `0x0083ad4c`, `get_xrefs_to 0x0083ad4c` → DATA ref @ `0x0066ce3e`):
```
0066ce2c: MOV EDX,[ESI+0x16c8]   ; push current value as default
0066ce3e: PUSH 0x83ad4c          ; "MaxDamage"
0066ce46: CALL 0x005276d0        ; CCINIClass__ReadInt
0066ce4b: MOV [ESI+0x16c8],EAX   ; store parsed MaxDamage
```
**Constructor fallback 1000.** From the RulesClass constructor `FUN_00665650`, read bytes @ `0x006674e8` (`read_memory 0x006674d0`): `c7 86 c8 16 00 00 e8 03 00 00` = `MOV dword [ESI+0x16C8], 0x3E8` = **1000**. The reader at `0x0066ce2c..0x0066ce51` supplies that current value as the missing-key fallback and stores parsed `MaxDamage` back to `+0x16C8`. Repository stock `ini/rules.ini:716` and `ini/rulesmd.ini:896` both set `MaxDamage=10000`, so standard YR runs with **10000**, not the fallback.

**YR-active:** YES. The kernel runs for every positive-damage hit in normal play; under stock rules the cap fires whenever the scaled hit reaches or exceeds 10000 (inclusive assignment on equality).

---

## (b) The minimum-damage clamps ("damage >= 1")

There is **no single** min-1; there are **two** `>= 1` floors at different stages, plus the overkill clamp. None of them is sourced from `MinDamage`.

### b1 — Defender pre-pipeline min-1 (positive damage)
**Identity:** `TechnoClass::ReceiveDamage` @ `0x00701900` (vtable+0x16C). Verified `decompile_function 0x00701900` this run.

**Exact condition + value (decompile):**
```c
// after country-armor divide (ftol) and vet/elite armor divide (ftol):
if (*in_stack_00000004 < 1) {
    *in_stack_00000004 = 1;
}
```
**Order position (this is the load-bearing part):** the incoming damage at this point is the **attacker-built** number (Fire_At output). The sequence in `TechnoClass::ReceiveDamage` is:
1. `*dmg = ftol(*dmg / (GetArmorMultForType(target) × TechnoClass+0x158))`  — country/unit armor DIVIDE
2. if vet/elite + ARMOR ability: `*dmg = ftol(*dmg / VeteranArmor)`  — vet armor DIVIDE
3. **`if (*dmg < 1) *dmg = 1;`**  ← the min-1 floor
4. TypeImmune gate, then the other immunity gates (D11)
5. fall through to `ObjectClass::ReceiveDamage`, which is **where Verses runs** (D13)

So the min-1 here sits **AFTER the defender country/vet armor divisors** and **BEFORE the immunity gates and BEFORE the Verses kernel.** It floors the post-armor-divide incoming damage so that armor mults / vet armor cannot reduce a positive hit below 1 before Verses is even applied. (It is gated by `ignoreDefenses==0 && damage>=0`; healing and ignore-defenses paths skip it.)

### b2 — Building min-1 (post-Verses)
**Identity:** `ObjectClass::ReceiveDamage` @ `0x005f5390`. Verified `decompile_function 0x005f5390` this run.

**Exact condition + value (decompile):**
```c
// after the Verses kernel (FUN_00489180) writes *dmg:
if (WhatAmI()==6 && *(char*)(... +0x1577)=='\0') {
    if (*in_stack_00000004 < 1) *in_stack_00000004 = 1;
}
```
**Order position:** AFTER the Verses kernel (so a building's Verses-scaled hit is floored to >=1), only for buildings (`WhatAmI()==6`) without the `+0x1577` flag (the CanC4/"can take zero" carve-out). This is D14.

### b3 — Overkill clamp (cap-to-remaining-HP)
**Identity:** `ObjectClass::ReceiveDamage` @ `0x005f5390`. Verified this run.

**Exact op (decompile):**
```c
iVar4 = *in_stack_00000004;        // damage (positive)
if (iVar4 < iVar6 /*currentHealth*/) {
    // Yellow classify using integer (Strength>>1)
} else {
    *in_stack_00000004 = iVar6;    // <-- overkill clamp: damage = currentHealth
}
```
**Order position:** AFTER Verses kernel, AFTER building min-1, as part of the state-classify block, BEFORE the `Health -= dmg` subtraction. This is D17. It guarantees the *reported* damage value equals min(scaledDamage, currentHealth) so kill-credit/EstimatedHealth bookkeeping doesn't over-count.

**Note — there is NO general (non-building) post-Verses min-1 in ObjectClass.** A non-building whose Verses-scaled hit truncated to 0 hits the `if (*dmg == 0) return 0;` early-out (no state change). The only thing keeping a *positive* non-building hit from dropping below 1 is the b1 floor (which runs on the pre-Verses, post-armor number) — Verses can still truncate a tiny post-armor value to 0, which is correct gamemd behavior (returns 0, no effect).

---

## (c) The `MinDamage` INI key — EXISTS, DEAD

**The key is real and parsed.** String `"MinDamage"` @ `0x0083ad40` (`search_strings MinDamage` → 1 match). `get_xrefs_to 0x0083ad40` → single DATA ref @ `0x0066ce5e` in `RulesClass__ReadCombatDamage`. Parse (disassembly `0x0066ce4b`–`0x0066ce6b`):
```
0066ce4b: MOV ECX,[ESI+0x16c4]   ; push current value as default
0066ce5e: PUSH 0x83ad40          ; "MinDamage"
0066ce64: CALL 0x005276d0        ; CCINIClass__ReadInt
0066ce6b: MOV [ESI+0x16c4],EAX   ; store parsed MinDamage at Rules+0x16C4
```
So `MinDamage` lands at **`Rules+0x16C4`** (one DWORD below MaxDamage at +0x16C8), parsed identically (ReadInt, current value as default).

**It is never read.** Whole-image byte scan for the displacement `c4 16 00 00` (= imm32 `0x16C4`, `search_byte_patterns "c4 16 00 00"`) returns 10 hits. Triaged each:
- `0x0066ce4d`, `0x0066ce6d` — the parse store + adjacent MaxDamage store (above).
- `0x006674f2` — `MOV [ESI+0x16C4], EAX` in RulesClass constructor `FUN_00665650` = the **default-init WRITE** (read_memory @ `0x006674e8`).
- `0x00443c0f` (BuildingClass__ToggleGate), `0x0044a949` (BuildingClass__Sell), `0x0045e165` (BuildingTypeClass__constructor), `0x00460b72`/`0x00460b97` (`MOV CL,[EBP+0x16C4]` byte read off a BuildingType base, read_memory @ `0x00460b68`), `0x00736d79`, `0x0073980d` (`MOV CL,[EAX+0x16C4]` byte read in UnitClass__Deploy off a non-rules base) — all are **0x16C4-sized offsets into unrelated structs** (BuildingType/Unit), NOT a `[g_Rules+0x16C4]` read. None is a DWORD read of the rules MinDamage field.

The damage kernel (`0x00489180`) reads only `+0x16C8` (MaxDamage). `Apply_area_damage` (`0x00489280`) reads many Rules fields (+0xfac, +0xff0, +0x1740, +0x54, +0x5c, +0x68, +0x74, +0xfa8, +0xb40/+0xb4c) but **not** +0x16C4. `TechnoClass::ReceiveDamage` and `ObjectClass::ReceiveDamage` do not read it either.

**Conclusion:** `MinDamage` is parsed-and-stored dead data in stock YR. The minimum applied to a hit is the hardcoded `>= 1` floor (b1/b2), NOT the `MinDamage` value. **Do not wire `MinDamage` into the damage pipeline** — doing so would be a fabricated clamp gamemd never applies.
**Default value (Rules+0x16C4):** UNCHECKED-exact (init from EAX in the constructor; not pinned this run) — and irrelevant because the field is never read. Repository stock `ini/rules.ini:717` and `ini/rulesmd.ini:897` both set `MinDamage=1`, but the parsed field remains dead.

---

## Clamp enumeration (address + order)

| # | Clamp | Function @ addr | Op | Order position | YR-active |
|---|---|---|---|---|---|
| 1 | Defender min-1 (positive) | `TechnoClass::ReceiveDamage 0x00701900` | `if (dmg < 1) dmg = 1` | after country+vet armor DIVIDE, before immunity gates & before Verses | YES |
| 2 | MaxDamage cap | `ApplyWarheadDamage 0x00489180` (cap at `0x00489249`) | `if (scaled >= Rules+0x16C8) scaled = Rules+0x16C8` (fallback 1000; stock 10000) | last op in kernel, after falloff+Verses, per target | YES |
| 3 | Building min-1 | `ObjectClass::ReceiveDamage 0x005f5390` | `if (WhatAmI==6 && !+0x1577 && dmg<1) dmg=1` | after Verses kernel, building-only | YES |
| 4 | Overkill clamp | `ObjectClass::ReceiveDamage 0x005f5390` | `if (dmg >= curHP) dmg = curHP` | after Verses + building-min1, before HP subtract | YES |
| — | `MinDamage` (Rules+0x16C4) | parsed `0x0066ce5e`, init `0x006674f2` | NONE — never read | n/a | **DEAD** |

Full positive-damage ordering: attacker Fire_At build → **[1] defender armor-divide + min-1** → immunity gates → Verses kernel (falloff → Verses → **[2] MaxDamage cap**) → **[3] building min-1** → **[4] overkill clamp** → `Health -= dmg`.

---

## YR-active vs TS-legacy

All four live clamps run in a standard YR skirmish (verified live). `MinDamage` is dead in stock YR (never read). No clamp in this set is gated behind a SpecialFlags/TS path. The VeinholeMonster (`WhatAmI==0xF`) `ftol` HP clamp inside `ObjectClass::ReceiveDamage` is TS-legacy (no stock-YR unit) and is NOT one of these damage clamps — do not model it.

---

## Rust handoff

In the `sim/combat/damage/` service:
- **MaxDamage:** apply the verified signed cap as the **final** op of `apply_warhead_damage` (the kernel), per target, AFTER the two damage-value ftol truncations. Plumb `max_damage` from `Rules.MaxDamage` (`[CombatDamage] MaxDamage`); initialize the parser fallback to **1000**, then load the INI value, which is **10000** in stock YR. The native comparison is signed and assigns the cap on equality.
- **Defender min-1:** in the receiver pre-pipeline (`TechnoClass` stage), floor the post-armor-divide positive damage to `>= 1` BEFORE immunity gates and BEFORE calling the kernel. Do NOT move it after Verses.
- **Building min-1:** floor to `>= 1` only for buildings without the CanC4/`+0x1577` carve-out, AFTER the kernel.
- **Overkill clamp:** clamp positive damage to `current_hp` (`dmg = dmg.min(current_hp)`) before subtracting HP, so the reported delta feeding kill-credit/EstimatedHealth matches.
- **`MinDamage`:** parse it for round-trip fidelity if you mirror `[CombatDamage]` (it is a real key at Rules+0x16C4) but **never apply it** — it is dead in gamemd. The only minimums are the hardcoded `>= 1` floors above.

---

## Verification ledger (this run)

- `decompile_function 0x00489180` + `disassemble_function 0x00489180` — MaxDamage cap op @ `0x00489249`, reads `[g_Rules+0x16C8]`, signed `>=`.
- `decompile_function 0x00701900` — defender min-1 `if (dmg<1) dmg=1` after armor/vet divide, before gates.
- `decompile_function 0x005f5390` — building min-1 (WhatAmI==6, !+0x1577) + overkill clamp (`else *dmg = curHP`).
- `decompile_function 0x00489280` (Apply_area_damage) — confirms +0x16C4 NOT read in AoE.
- `search_strings MaxDamage` → `0x0083ad4c`; `search_strings MinDamage` → `0x0083ad40`.
- `get_xrefs_to 0x0083ad4c` → `0x0066ce3e` (MaxDamage parse); `get_xrefs_to 0x0083ad40` → `0x0066ce5e` (MinDamage parse).
- `decompile_function 0x0066ce3e` + `disassemble_function 0x0066ce3e` (RulesClass__ReadCombatDamage) — MaxDamage→+0x16C8 store @ `0x66ce4b`, MinDamage→+0x16C4 store @ `0x66ce6b`, both via CCINIClass__ReadInt `0x005276d0`.
- `search_byte_patterns "c4 16 00 00"` → 10 hits; `get_function_by_address` + `read_memory` on each: only `0x006674f2` (constructor WRITE) and `0x0066ce4d/0x0066ce6d` (parse) touch g_Rules+0x16C4; the rest are non-rules struct offsets. → MinDamage never read = DEAD.
- `read_memory 0x006674d0` — RulesClass constructor: `MOV dword [ESI+0x16C8], 0x3E8` (MaxDamage missing-key fallback 1000); `MOV [ESI+0x16C4], EAX` (MinDamage default init, unread).
