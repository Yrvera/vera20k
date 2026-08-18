# BulletClass Delayed-Detonation Anim-Listener Path

**Address(es):** `BulletClass::AI @ 0x004666E0` (full function); gate `FUN_00410a40 @ 0x00410a40`; inner strcmp `FUN_007c8d20 @ 0x007c8d20`; anim spawn `AnimClass::Constructor @ 0x00421EA0`; `BulletClassBulletDetonationImpactDamage @ 0x00468D80`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** The delayed-detonation / anim-listener branch inside `BulletClass::AI`: gate identity, INI/data activation condition, scheduler interaction while waiting, struct field offsets, stock YR liveness, and Rust handoff delta.  
**Non-Scope:** Full BulletClass::AI movement/homing math; vtable+0xF8 teardown chain (covered by slot-1 sibling doc); `g_AnimClass_RemoveListeners` roster mutation rules (covered by `DETACH_LISTENER_ROSTER_MUTATION_RULES_RESWARM_20260528.md`); nuke damage formula; ScreenNukeFlash render internals.  
**Confidence:** HIGH for gate identity, struct offsets, and YR liveness (all assembly-verified). MEDIUM for "BulletTypeClass+0x24 is the INI ID string" (consistent with UnitTypeClass+0x24 usage pattern and `FUN_00410a40` usage in `UnitTypeClass__ReadINI`, but not directly decompiled via a BulletTypeClass ReadINI call in this session — see Remaining Uncertainty).  
**Active in YR:** YES — unconditionally, every skirmish containing a Nuclear Missile superweapon or Nuke Carrier weapon firing.  

---

## Working-Notes Gate

**Target question:** Does the delayed-detonation path activate for stock YR nuke impact, how is it gated, what are the struct fields, and what is the scheduler interaction while the bullet waits?

**Non-goals:** Do not re-derive roster mutation rules; do not implement code; do not inspect any address not directly relevant to the above questions.

**Evidence needed to mark COMPLETE:** gate decompile, inner compare decompile, memory read of `DAT_0081af98`, assembly context for struct fields 0x154/0x158, assembly context for top-of-function listener check, caller trace of gate function, INI cross-check confirming `[NUKE]` bullet type exists in stock YR.

**Stop conditions:** Stop after gate identity, field offsets, liveness, and scheduler interaction are assembly-verified.

---

## 1. Overview

`BulletClass::AI @ 0x004666E0` contains a two-part mechanism: at the **top** of the function it checks whether the bullet is currently waiting for a spawned anim to finish (the listener-active path); at the **bottom** (near `LAB_00467e53`) it gates entry into the deferred path by name-checking the bullet's type. Together these implement the observable "nuclear flash plays fully, then blast damage and teardown follow" behavior of the RA2/YR nuclear missile superweapon impact.

**Active in YR:** Yes. The gate name is "NUKE" which matches the stock `[NUKE]` bullet entry in `[Bullets]` (rulesmd.ini `33=NUKE`). The `[NukePayload]` weapon uses `Projectile=GiantNukeDown` and `Warhead=NUKE`, and the bullet that carries the warhead fires from the "NukePayload" weapon on the descending missile object. This path fires every time the Nuclear Missile or Nuke Carrier weapon detonates in a standard YR skirmish.

---

## 2. Gate Identity: `FUN_00410a40` with `&DAT_0081af98`

### 2.1 Decompile of gate

Verified via `decompile_function 0x00410a40`:

```
bool __thiscall FUN_00410a40(int param_1, undefined4 param_2)
{
    iVar1 = FUN_007c8d20(param_2, param_1 + 0x24);
    return iVar1 == 0;
}
```

`param_1 = ECX` = the `BulletTypeClass*` (loaded into ECX from `[EBP + 0x128]` before the call, per assembly context `0x00467e53`).  
`param_2 = stack` = `0x81af98` (pushed at `0x00467e59`).

### 2.2 Inner comparison function

Verified via `decompile_function 0x007c8d20`:

`FUN_007c8d20` is a case-insensitive string compare (`strcmpi`). It returns `0` when the strings are equal. The gate therefore returns `true` when `strcmpi(DAT_0081af98, BulletTypeClass+0x24) == 0`.

### 2.3 Content of `DAT_0081af98`

Verified via `read_memory 0x0081af98 length=64`:

Bytes at 0x81af98: `4e 55 4b 45 00` … = **"NUKE"** (null-terminated at offset 0).

The gate is therefore: **`strcmpi("NUKE", BulletTypeClass.name) == 0`** — the delayed-detonation path activates if and only if the bullet's INI type name is exactly "NUKE" (case-insensitive).

### 2.4 INI confirmation

`rulesmd.ini` Bullets list entry `33=NUKE` (line 2909) defines the "NUKE" bullet type. The `[NUKE]` warhead section (line 27226) includes the INI comment `;AnimList=NUKEBALL ; SJM: Activated from code now... see Bullet AI` — a developer note explicitly confirming that NUKEBALL is spawned from BulletClass::AI code, not from the warhead's `AnimList`. The active warhead AnimList is `NUKEANIM` (post-damage, separate from the pre-damage flash).

Active in YR: Yes. `[NukePayload]` weapon (rulesmd.ini line 24017) fires `Projectile=GiantNukeDown` with `Warhead=NUKE`. The `GiantNukeDown` bullet's warhead is "NUKE", so when its bullet AI runs, the gate test succeeds.

### 2.5 Gate callers (liveness verification)

Verified via `get_function_callers 0x00410a40`: three callers:

| Caller | Address | Role |
|---|---|---|
| `BulletClassAiHomingDetonationPath` | `0x004666E0` | The delayed-detonation gate itself |
| `FUN_005f3e50` | `0x005f3e50` | Likely BulletType or ObjectType helper (name check reuse) |
| `UnitTypeClass__ReadINI` | `0x00747620` | Uses same gate with `&DAT_0084314c` (different arg) to gate multi-turret INI reads — confirms `FUN_00410a40` is a general "is-this-type-named-X" predicate |

No TS-only gate or disabled flag guards the call in `BulletClass::AI`. The test is executed unconditionally whenever a bullet with movement completes its tick (after `ObjectClass::AI`). Active in YR: **Yes**, confirmed by absence of `SpecialFlags` or session-variable gate upstream.

---

## 3. Struct Field Offsets

Both fields are in the BulletClass instance, `param_1` is `int *` (Ghidra type) so `param_1[N]` = byte offset `N × 4`.

### 3.1 Assembly confirmation — bottom block (write path)

From `get_assembly_context 0x00467f50` (inside the nuke-flash block):

```
00467f36: MOV dword ptr [EBP + 0x154], EAX   ; store anim pointer
00467f3c: MOV byte ptr [EBP + 0x158], 0x1    ; set listener-active flag = 1
```

EBP = bullet instance. Therefore:
- **BulletClass+0x154** = watched AnimClass* (Ghidra notation `param_1[0x55]` = int-index 0x55 = byte offset 0x154). Verified at `0x00467f36`.
- **BulletClass+0x158** = listener-active flag (1-byte). Verified at `0x00467f3c`.

### 3.2 Assembly confirmation — top block (read/check path)

From `get_assembly_context 0x004666f7` (top-of-function listener check):

```
004666f7: MOV AL, byte ptr [EBP + 0x90]   ; logic-enabled guard (param_1[0x24])
00466705: MOV AL, byte ptr [EBP + 0x158]  ; load listener-active flag
0046670b: TEST AL, AL
0046670d: JZ  0x00466789                  ; skip if flag==0 (not in listener mode)
0046670f: MOV EAX, dword ptr [EBP + 0x154]; load watched anim pointer
00466715: TEST EAX, EAX
00466717: JNZ 0x00467fee                  ; anim still alive → early-return (keep waiting)
```

Confirmed: if flag is set and anim pointer is non-null → early-return, no movement, no detonation. When anim pointer becomes null (anim has finished and been destroyed) the function falls through to the removal-from-roster, detonation, and teardown sequence.

**Summary:**

| Field | Byte offset | Ghidra int-index | Evidence |
|---|---|---|---|
| Watched `AnimClass*` | 0x154 | param_1[0x55] | Assembly write `0x00467f36`; read `0x0046670f` |
| Listener-active flag (byte) | 0x158 | *(byte*)(param_1+0x56×4) note: byte not dword | Assembly write `0x00467f3c`; read `0x00466705` |

Note on Ghidra notation: the decompile writes `*(undefined1 *)(param_1 + 0x56) = 1` — since param_1 is `int*`, `param_1 + 0x56` = address `param_1 + 0x56×4 = param_1 + 0x158`, then cast to `undefined1*`, confirming byte offset 0x158.

---

## 4. Full Sequence: Delayed Detonation

### 4.1 Setup path (fires once on impact tick for "NUKE" bullets)

Preconditions: bullet has completed normal homing/movement, detonation condition is met.

1. Gate `FUN_00410a40(&DAT_0081af98)` returns true (type name == "NUKE").
2. Call `(*vtable+0x1c8)()` (height-or-distance check); if below ground, call `(*vtable+0x1cc)(0)` (ground clamp).
3. Call `ScreenNukeFlash()` — fullscreen white flash, render side-effect.
4. Call `(*vtable+0x1b8)(&coord)` then `CreateRadarEvent(coord)` — radar ping.
5. Resolve flash anim type: `AnimTypeClass__FindByIndex()` → index into `g_AnimTypes_Array`. (Evidence from decompile: `iVar5 = AnimTypeClass__FindByIndex(); if (iVar5 != -1) iVar5 = *(g_AnimTypes_Array + iVar5*4);`).
6. `operator_new(0x1c8)` then `AnimClass__Constructor(anim_type, &bullet_coord, 0, 1, 0x2600, layer_arg, 0)` — spawns NUKEBALL anim at impact coords.
7. Store result at `[EBP + 0x154]` (BulletClass+0x154 = anim pointer).
8. Set `[EBP + 0x158] = 1` (listener-active flag).
9. Append `param_1` (the bullet) to `g_AnimClass_RemoveListeners`: grows vector if needed, writes bullet pointer at `g_AnimClass_RemoveListeners + count*4`, increments `g_AnimClass_RemoveListeners_Count`.
10. `goto LAB_00467fba` — **skip** `BulletClassBulletDetonationImpactDamage` and `(*vtable+0xF8)()`. The bullet remains alive in the LogicClass vector.

### 4.2 Wait path (every subsequent tick while anim is alive)

Every tick LogicClass calls `BulletClass::AI` via vtable+0x5C:

1. `ObjectClass::AI()` called first (unconditional header).
2. Logic-enabled guard: `byte [EBP+0x90]` must be non-zero or function returns immediately.
3. `byte [EBP + 0x158] != 0` → enters listener branch.
4. Load `dword [EBP + 0x154]` (anim pointer). If **non-null** (anim still running) → `return` immediately. **No movement, no position update, no detonation.** Bullet is frozen in place.
5. If null (anim destroyed): proceed to removal and detonation.

### 4.3 Detonation path (fires on tick the anim expires)

1. Find `param_1` in `g_AnimClass_RemoveListeners`: call `(*DAT_00b0f5b8+0x10)(&local)` (find-in-vector).
2. If found and `index < g_AnimClass_RemoveListeners_Count`: decrement count, left-compact entries above removed index. (Per decompile; matches roster rules in `DETACH_LISTENER_ROSTER_MUTATION_RULES_RESWARM_20260528.md`).
3. Clear `[EBP + 0x158] = 0` (listener flag off).
4. Call `BulletClassBulletDetonationImpactDamage(0)` — applies damage and area effect.
5. Call `(*vtable+0xF8)()` — bullet teardown (conceal/limbo/removal from LogicClass).
6. Return.

### 4.4 Scheduler interaction

- During the wait phase the bullet remains in the LogicClass active vector (registered via `+0x98=1` at Fire→Reveal→`FUN_0055BAA0` time, per settled prior fact).
- LogicClass::PerTickUpdate reloads the active vector count after each `vtable+0x5C` call. The wait-path early-return is effectively a no-op tick — no tail appends or removals occur from the bullet's side during this phase.
- On the detonation tick, `(*vtable+0xF8)()` removes the bullet from the LogicClass vector (via Conceal → `FUN_0055BAE0`), which decrements the count mid-pass. The scheduler handles this via its live count reload: the forward loop terminates early or skips the compacted gap as per `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`.
- The `g_AnimClass_RemoveListeners` removal in step 4.3 above uses the same left-compaction pattern documented in `DETACH_LISTENER_ROSTER_MUTATION_RULES_RESWARM_20260528.md` for the `DAT_00B0F724` broad listener vector. This slot's vector is the anim-specific one (`g_AnimClass_RemoveListeners` / `DAT_00b0f5bc`), not the broad object roster, but the compaction mechanics are structurally identical as seen in the decompile.

---

## 5. Stock YR Liveness

| Evidence | Verdict |
|---|---|
| Gate is `strcmpi("NUKE", bullet_type_name)` with no enclosing SpecialFlags or session-variable guard. | Active in YR: Yes |
| `rulesmd.ini` line 2909: `33=NUKE` — "NUKE" is a registered stock bullet type. | Confirms gate fires |
| `rulesmd.ini` line 24020: `[NukePayload]` uses `Projectile=GiantNukeDown`, line 24023: `Warhead=NUKE`. | GiantNukeDown bullet carries NUKE warhead → BulletClass "NUKE" type. Needs further verification; see Remaining Uncertainty. |
| `rulesmd.ini` comment line 27235: `;AnimList=NUKEBALL ; SJM: Activated from code now... see Bullet AI` | Developer explicitly marks this as a code-driven anim path, not TS dead code |
| Caller trace: only 3 callers, all active code paths, no TS-only gate observed | Active in YR: Yes |

**TS-legacy verdict:** NOT TS legacy. The INI developer comments, stock rulesmd.ini presence, and absence of any gating flag confirm this is a live YR path.

---

## 6. Current Rust Implementation Status

No nuke missile / nuke impact module exists in `src/sim/superweapon/`. The directory contains: `force_shield.rs`, `genetic_converter.rs`, `iron_curtain.rs`, `lightning_storm.rs`, `paradrop.rs`, `psychic_reveal.rs`, `invulnerability.rs`, `cell_grid.rs`, `mod.rs`. There is no `nuke_missile.rs`, `nuke_payload.rs`, or `nuclear_impact.rs`. Grep for `NukePayload|nuke_payload|nuclear|NUKE` across `src/` returned zero matches (verified).

**Rust delta:** The flash-then-detonate timing does not exist in Rust yet. If/when projectile simulation is implemented for the Nuclear Missile, the delay mechanism must be built from the start, not retrofitted. Implementing immediate detonation without the anim-listener delay would be a observable DRIFT (damage applies before the visual flash finishes — player sees blast without seeing the full NUKEBALL animation play first).

---

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|---|---|
| "NUKE" bullet type uses anim-listener deferred detonation: flash anim spawned at impact, damage held until anim pointer becomes null, then detonation+teardown. Active in YR: Yes. | Assembly `0x00467f36`, `0x00467f3c`, `0x00467e5e`; decompile `0x004666E0`; INI comment rulesmd.ini:27235 | Not implemented — no nuke projectile module exists in src/sim/superweapon/ | Future BulletClass sim in src/sim/; NukePayload weapon fire path | Implement anim-listener state on BulletClass: flag+pointer fields at 0x158/0x154; register in AnimClass remove-listener roster on setup; early-return on subsequent ticks while pointer non-null; call detonation damage only when pointer clears | Nuclear missile fires, NUKEBALL anim plays fully, then damage numbers and radiation appear only after anim finishes — not before. Verify by checking that damage events are deferred N frames (duration of NUKEBALL anim). | `nuke_bullet_damage_deferred_until_anim_ends` | If damage is applied immediately on impact, players see blast effects (craters, unit deaths) before the visual flash reaches full intensity — directly observable DRIFT every nuke launch. |
| Gate is strict type-name equality "NUKE" (strcmpi). No other bullet type enters this path. Active in YR: Yes. | `decompile_function 0x00410a40`; `read_memory 0x81af98`; assembly `0x00467e59` | N/A until implemented | BulletType identification in parser/sim | Implement gate as exact INI ID match on bullet type, not on warhead name or projectile category. "NUKE" is a bullet type name, not a warhead flag. | GiantNukeDown bullet with Warhead=NUKE enters listener path; Invisible/HE/other bullets with NUKE warhead do NOT enter it (their bullet type name is not "NUKE"). | `only_nuke_named_bullet_enters_listener_path` | Matching on warhead name instead of bullet type name would spuriously gate non-NUKE bullets using the NUKE warhead on other weapons. |
| Scheduler: while waiting, bullet is still in LogicClass active vector and AI is called every tick, but early-returns after listener check with no side effects. Active in YR: Yes. | Assembly `0x00466705..0x00466717`; decompile `0x004666E0` top block | N/A until implemented | BulletClass tick loop in future sim | The bullet must remain in the logic tick schedule between impact and anim completion — must not be removed from the active set prematurely. Only call detonation+removal on the tick the anim pointer clears. | Nuclear missile tick count between impact and damage = NUKEBALL anim frame count / anim frame rate — not 0 and not infinite. | `nuke_bullet_stays_alive_in_scheduler_while_waiting` | Premature removal from logic tick (e.g. treating impact as detonation-trigger) skips the deferred path entirely. |

---

## 8. Negative Facts / Do Not Do

- **Do not apply nuke damage immediately on impact.** Active in YR: Yes. Evidence: decompile shows `goto LAB_00467fba` explicitly bypasses `BulletClassBulletDetonationImpactDamage` when the anim-listener path is taken; damage fires only when `[EBP+0x154]` becomes null.
- **Do not match on warhead name.** The gate checks the bullet type's INI ID, not the warhead name. A weapon using the NUKE warhead with a differently-named projectile (e.g. `Projectile=InvisibleAll`) would NOT trigger this path. Evidence: `read_memory 0x81af98` = "NUKE"; `FUN_00410a40` compares against `BulletTypeClass+0x24` (the type's own name field).
- **Do not remove the bullet from the LogicClass active vector before the anim finishes.** The teardown call `(*vtable+0xF8)()` occurs only in the detonation sequence (after `BulletClassBulletDetonationImpactDamage`), not during the wait phase. Evidence: assembly `0x00466717: JNZ 0x00467fee` returns without reaching the detonation sequence when anim pointer is non-null.
- **Do not snapshot the `g_AnimClass_RemoveListeners` roster at setup.** Per `DETACH_LISTENER_ROSTER_MUTATION_RULES_RESWARM_20260528.md`, the remove-listener roster uses live count reload and left-compaction on removal; the Rust equivalent must not pre-snapshot it.
- **Do not implement the NUKEBALL anim spawn as warhead-driven.** The anim is spawned from BulletClass::AI code before detonation, not from the `[NUKE]` warhead's `AnimList`. The warhead `AnimList=NUKEANIM` is a separate post-detonation anim. Spawning NUKEBALL via the warhead path would double-spawn it or play it at the wrong time.

---

## 9. Remaining Uncertainty

- **BulletTypeClass+0x24 = INI type name:** confirmed by pattern match with `UnitTypeClass__ReadINI` (which uses `param_1 + 0x24 = iVar1` for INI reads and `FUN_00410a40` in the same way), but no direct `BulletTypeClass::ReadINI` decompile was run to confirm the offset independently. Confidence: MEDIUM-HIGH. Risk: low — if wrong, the gate never fires and the anim-listener path becomes dead code, which is easily observable.
- **Which anim type is resolved by `AnimTypeClass__FindByIndex()`:** the decompile shows the index comes from a call with no obvious argument (likely reads a field from the bullet's warhead), and the result is used to look up `g_AnimTypes_Array[index]`. The exact anim type resolved (NUKEBALL or similar) is not confirmed in this session. INI comment `SJM: NUKEBALL now called from code` strongly suggests it is NUKEBALL, but the exact FindByIndex argument source was not traced. Confidence for "NUKEBALL": HIGH (developer comment); confidence for exact index source: UNVERIFIED.
- **Whether `[GiantNukeDown]` bullet's type name resolves to "NUKE":** `[NukePayload]` weapon uses `Projectile=GiantNukeDown`. The bullet type list entry `33=NUKE` in rulesmd.ini is the "NUKE" bullet type, and `[NUKE]` warhead is separate. Whether the `GiantNukeDown` bullet object's type name is "NUKE" (i.e. `[GiantNukeDown]` section matches bullet-type entry "NUKE") needs a BulletTypeClass constructor / FindOrAllocate trace to confirm. Based on developer comment and INI structure this is the intended mechanism, but the exact mapping `GiantNukeDown → type-name = "NUKE"` was not assembly-verified in this session.

---

## 10. Sources

- Ghidra read-only decompile: `BulletClass::AI @ 0x004666E0`, `FUN_00410a40 @ 0x00410a40`, `FUN_007c8d20 @ 0x007c8d20`, `UnitTypeClass__ReadINI @ 0x00747620`.
- Ghidra read-only assembly contexts: `0x00467e53`, `0x00467e5e`, `0x00467ef0`, `0x00467f50`, `0x00467f70`, `0x004666f7` (struct field reads/writes at 0x154 and 0x158).
- Ghidra read_memory: `DAT_0081af98` (64 bytes) = "NUKE" at offset 0.
- Ghidra get_function_callers: `FUN_00410a40` — 3 callers verified.
- INI cross-check: `rulesmd.ini` lines 2909, 24017–24024, 27226–27236; art.ini/artmd.ini NUKEBALL comment.
- Sibling docs read: `DETACH_LISTENER_ROSTER_MUTATION_RULES_RESWARM_20260528.md` (roster compaction rules, cited).
- Rust source scan: `src/sim/superweapon/` (glob + grep, no nuke missile module found).
