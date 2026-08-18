# Spine Rung #17 — "EMPulseClass list tick (reverse walk)" → actually RadSiteClass list tick

**Status:** VERIFIED from binary (gamemd.exe, image base 0x400000).
**Body site:** `LogicClassPerTickUpdateLiveVector` @ `0x0055AFB0`, rung region `0055b5cd–0055b5ea`.
**Verdict on the plan's label:** LABEL DRIFT. This rung is **NOT** EMPulseClass. The array
it walks (`DAT_00b04bd4[]`, count `DAT_00b04be0`) is the **RadSiteClass** (radiation-site)
array, populated by `RadSiteClass__Constructor`/`__Destructor`. The "EMPulse" naming likely
bled over from the *adjacent* call two instructions later — `0055b5f6 CALL 0x004c54a0`
(rung S, whose decomp label `EMPulseClass__UpdateAll` is itself flagged suspect in the
ladder). Rung #17 = the RadSiteClass AI fan-out.

---

## 1. The body-site loop (exact)

Disassembly `0055b5cd–0055b5ea` (verified via `disassemble_function 0x0055AFB0`):

```
0055b5cd: MOV EAX,[0x00b04be0]            ; count = DAT_00b04be0
0055b5d2: LEA ESI,[EAX + -0x1]            ; i = count - 1   (REVERSE walk)
0055b5d5: TEST ESI,ESI
0055b5d7: JL  0x0055b5ea                  ; skip whole loop if count == 0
0055b5d9: MOV ECX,dword ptr [0x00b04bd4]  ; base = DAT_00b04bd4
0055b5df: MOV ECX,dword ptr [ECX + ESI*0x4]   ; this = base[i]
0055b5e2: MOV EDX,dword ptr [ECX]         ; vtable
0055b5e4: CALL dword ptr [EDX + 0x5c]     ; this->vt[0x5c]()  __thiscall, no args
0055b5e7: DEC ESI
0055b5e8: JNS 0x0055b5d9
```

**Gate:** UNCONDITIONAL. Pure reverse loop, no mode/Special/Scen flag. Empty (single
TEST/JL, no work) when `DAT_00b04be0 == 0`. Confirms the plan's stated gate.

Runs in the ladder AFTER `LightningStorm__Process` (`0055b5c8 CALL 0x0053a6c0`, rung P)
and BEFORE `FUN_00554d50` (`0055b5f1`, rung R) / `0x004c54a0` (`0055b5f6`, rung S).

---

## 2. What it walks — RadSiteClass array

`DAT_00b04bd4` = base ptr, `DAT_00b04be0` = count. Array membership proven by
constructor/destructor xrefs (verified via `get_xrefs_to 0x00b04bd4` / `0x00b04be0`):

- `RadSiteClass__Constructor` @ `0x0065b1e0` (verified via `decompile_function 0x0065b220`,
  func entry confirmed via `get_function_by_address 0x0065b220`): sets
  `*param_1 = &vtable__RadSiteClass`, then appends `param_1` to `DAT_00b04bd4[]` and does
  `DAT_00b04be0 = DAT_00b04be0 + 1` (DynamicVector grow via `DAT_00b04bd0+8` predicate).
- `RadSiteClass__Destructor` @ ~`0x0065b340` removes the entry (xrefs WRITE `DAT_00b04be0`
  at `0065b36d`).

Primary vtable `vtable__RadSiteClass` @ `0x007f0810` (via `list_globals vtable__RadSiteClass`).
vt+0x5c = `0x007f086c`; `read_memory 0x007f086c` → bytes `00 b8 65 00` = **`0x0065b800`**.

---

## 3. The driver — `RadSiteClass__AI` @ 0x0065b800

Verified via `decompile_function 0x0065b800` / `get_function_by_address 0x0065b800`
(body `0065b800–0065b8e1`). Per-RadSite each tick:

1. **Decrement lifetime** `this[0x1c] -= 1`.
2. **Rad-damage timer** (gate: `this[10]`/`this[0xc]` countdown; reload interval =
   `*(Rules+0x1810)`): on expiry calls `RadSiteClass__ApplyRadDamage` @ `0x0065bd00`,
   then re-arms `this[10]=frame`, `this[0xc]=Rules+0x1810`.
3. **Light-flash timer** (gate: `this[0xd]`/`this[0xf]` countdown; reload interval =
   `*(Rules+0x1814)`): on expiry calls `FUN_00554aa0` (cell tint setter) with color
   channels `this[0x16..0x18]` scaled by `remaining_life/total_life` (`this[0x1c]/this[0x1b]`),
   then re-arms from `Rules+0x1814`.
4. **Expiry**: when `this[0x1c] < 1`, calls vt+0x20 (`0x0065bed0`
   `RadSiteClass__ScalarDeletingDestructor`, verified via `decompile_function 0x0065bed0`)
   with arg=1 → runs `RadSiteClass__Destructor` (removes self from `DAT_00b04bd4[]`) + frees.

`Rules+0x1810` / `Rules+0x1814` are the radiation application/light-flash intervals
(RadApplicationDelay / radiation-tint refresh). Not re-derived to INI key names here;
read straight from `g_RulesClass_Instance` in the AI body.

---

## 4. RNG — NONE. No stream touched.

Full callee tree swept; **no RNG draw anywhere in this rung.**

- `RadSiteClass__AI` callees (`get_function_callees 0x0065b800`): `FUN_00554aa0`,
  `RadSiteClass__ApplyRadDamage`.
- `RadSiteClass__ApplyRadDamage` @ 0x0065bd00 callees (`get_function_callees 0x0065bd00`):
  `FUN_00487d00`, `MapClass__Get_CellClass` (0x005657a0), `Math__ftol` (0x007c5f00),
  `Sqrt_Approx` (0x004cac40). It walks the rad radius, computes a distance-falloff value
  `dVar5 = ((radius-dist)/radius) * this[0x4c]` and feeds it to `FUN_00487d00`.
- `FUN_00487d00` @ 0x00487d00 (verified via `decompile_function`): subtracts the falloff
  from the **CellClass** radiation level field (`cell+0xf0`), clamps at 0. Pure arithmetic,
  no RNG. (Note: this adjusts the CELL's stored rad level — it does not roll warhead damage
  on a unit, so no warhead/armor RNG either.)
- `FUN_00554aa0` @ 0x00554aa0 (verified): deterministic tint-field store; on change calls
  `FUN_00554af0`.
- `FUN_00554af0` @ 0x00554af0 (verified via `decompile_function`): radiation-glow lighting
  spread over the radius, queues lighting cells into `DAT_00abca44[]`. Geometric/lighting
  math only (`Sqrt_Approx`, `Math__ftol`, `FUN_005657e0` cell-validity, `FUN_00483e30`,
  `operator_new`). No RNG.
- `Sqrt_Approx` @ 0x004cac40 (verified): float-bit-trick sqrt via `&DAT_008650bc` LUT.
  Pure. `Math__ftol` is the float→long truncation helper. Pure.

**RNG stream: none. Draws: 0.** This rung is RNG-neutral — it does not advance
Scen->Random, g_MainRng, or g_MapGenRng, so it is inert for the lockstep RNG-draw order
(its only lockstep relevance is the per-tick ORDER position and the cell rad-level/lighting
state writes).

---

## 5. Active-in-YR? YES. Not Tiberian Sun legacy.

- RadSiteClass instances are created by `WarheadTypeClass__Detonate` @ `0x004690b0` — the
  ONLY constructor caller (verified via `get_function_callers 0x0065b1e0`). A RadSite spawns
  when a warhead with radiation detonates.
- In stock YR this is the **Desolator** (deploy weapon / Radiation Eradicator) and any
  radiation warhead — standard YR content, reachable and visible (radiation puddle + green
  cell glow + damage-over-time to infantry/units) in a normal skirmish.
- Therefore: **ACTIVE in YR**, player-visible (the green radiation glow render — cf. recent
  repo commits on radiation-glow-render T1–T4). NOT dead/TS-gated.

---

## 6. Lockstep-contract notes for this rung

- Order: fixed at ladder position #17 — runs after rung P (LightningStorm) and before
  rung R (`FUN_00554d50` shroud/lighting flush). The lighting cells this rung queues into
  `DAT_00abca44[]` (via `FUN_00554af0`) are consumed by rung R later in the same tick.
- Walk direction: **reverse** (i = count-1 .. 0), so a RadSite that self-deletes on expiry
  (vt+0x20) removes the *highest* index first — safe under reverse iteration.
- No RNG ordering impact (0 draws).

### Citations
- `disassemble_function 0x0055AFB0` (body site, rung region 0055b5cd–0055b5ea)
- `decompile_function 0x0055AFB0`
- `get_xrefs_to 0x00b04bd4`, `get_xrefs_to 0x00b04be0` (RadSite array membership)
- `decompile_function 0x0065b220` + `get_function_by_address 0x0065b220` (constructor)
- `list_globals vtable__RadSiteClass`; `read_memory 0x007f086c` (vt+0x5c = 0x0065b800)
- `decompile_function 0x0065b800` + `get_function_by_address 0x0065b800` (RadSiteClass__AI)
- `decompile_function 0x0065bd00` (ApplyRadDamage) + `get_function_callees 0x0065bd00`
- `get_function_callees 0x0065b800` (AI callees)
- `decompile_function 0x00487d00`, `0x00554aa0`, `0x00554af0`, `0x0065bed0`, `0x004cac40`
- `get_function_callers 0x0065b1e0` → `WarheadTypeClass__Detonate 0x004690b0` (sole creator)
- `get_function_callers 0x0065b800` → none direct (vtable-only dispatch)
