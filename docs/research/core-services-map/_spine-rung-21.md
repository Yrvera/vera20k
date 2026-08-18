# Spine Rung 21 — U. AnimClass-subset vector tick (MODE-GATED, separate from rung T)

Driver: per-object `vt+0x5c` (= `AnimClass::AI` @ `0x00423ac0`) over the **secondary anim
vector** `DAT_00a83e00`/`DAT_00a83e04` (base) / `DAT_00a83e10` (count).
Body call site: forward loop `0x0055b61b..0x0055b649` inside `LogicClass::PerTickUpdate`
(`LogicClassPerTickUpdateLiveVector` @ `0x0055AFB0`), order #21.

## Verdict summary

| Field | Value |
|---|---|
| Order | 21 — immediately after rung T (MAIN object vector over `param_1+4`/`param_1+0x10`, loop `0x0055b5fb..0x0055b619`), before rung V (`FUN_0053d310` wave-splash @ `0x0055b64b`). Confirmed via `disassemble_function 0x0055AFB0`. |
| Driver | `(**(code **)(*obj + 0x5c))()` per entry — `vt+0x5c` resolves to `AnimClass::AI` @ `0x00423ac0` (read `0x007e3354 + 0x5c = 0x007e33b0` → `c0 3a 42 00` = `0x00423ac0`, via `read_memory 0x007e33b0`). |
| Walks | The DynamicVector at `DAT_00a83e00` (cap 10): base ptr `DAT_00a83e04`, count `DAT_00a83e10`. **NOT** the main `g_AnimClass_Array` (`0x00a8e9ac`/count `0x00a8e9b8`). This is a *subset* vector holding the move/click-feedback ("MoveFlash") cursor anims relocated out of the main list. |
| Gate | `g_GameMode != 0 && g_GameMode != 5` **AND** count `DAT_00a83e10 > 0`. Confirmed (see below). |
| Draws RNG | **Yes** (conditionally, inside `AnimClass::AI` — debris spawn, tiberium-spread overlay, anim-chain randomized delay/rate). |
| RNG stream | **`Scen->Random`** = `g_ScenarioClass_Instance(0x00a8b230) + 0x218` — verified at every `Random__RandomRanged` (`0x0065c7e0`) call site in `AnimClass::AI`. Not `g_MainRng`, not `g_MapGenRng`. |
| RNG draws | 0 on the common path (MoveFlash anims have no splittable/spread/chain fields). Up to 2 (debris) + 2-per-tiberium-cell (overlay frame+state) + 1 (loop delay) or 1 (next-anim rate) per object on data-driven paths. See "RNG" below. |
| Active in YR | **Yes** — the move/click-feedback anim (`Rules->MoveFlash`, `Rules+0xbb4`) is created on every move/attack click in a live game and ticked here. Visible every match. |
| TS legacy | No (driver/gate are live YR). The `g_GameMode == 5` exclusion is the only mode that suppresses it. |

## Purpose (one line)

Per-tick AI advance for the **secondary anim vector** — the small DynamicVector that holds
the move/attack click-feedback cursor animation(s) (`Rules->MoveFlash`), which are
deliberately moved out of the main `g_AnimClass_Array` so they tick in this separate,
mode-gated slot.

## What it walks / does

Verified via `decompile_function 0x0055AFB0` + `disassemble_function 0x0055AFB0`
(loop `0x0055b61b..0x0055b649`):

```
0055b61b: MOV EAX,[0x00a8b238]   ; g_GameMode
0055b620: TEST EAX,EAX
0055b622: JZ  0x0055b64b         ; GameMode == 0 -> skip (shell / not-in-game)
0055b624: CMP EAX,0x5
0055b627: JZ  0x0055b64b         ; GameMode == 5 -> skip
0055b629: MOV EAX,[0x00a83e10]   ; count
0055b62e: XOR ESI,ESI
0055b630: TEST EAX,EAX
0055b632: JLE 0x0055b64b         ; count <= 0 -> skip
0055b634: MOV EAX,[0x00a83e04]   ; vector element base
0055b639: MOV ECX,[EAX+ESI*4]    ; obj ptr (forward order)
0055b63c: MOV EDX,[ECX]          ; vtable
0055b63e: CALL [EDX+0x5c]        ; AnimClass::AI
0055b641: MOV EAX,[0x00a83e10]   ; reload count (objects can self-delete)
0055b646: INC ESI
0055b647: CMP ESI,EAX
0055b649: JL  0x0055b634
```

Forward iteration; count is reloaded each pass (`0x0055b641`) so an anim that completes and
removes itself mid-loop is handled safely.

### The vector is a distinct subset, not all anims

`DAT_00a83e00` is a global `DynamicVectorClass<AnimClass*>` (vtable `0x7e9f24`, capacity
`0xa`), initialized as a CRT/atexit static (`disassemble_bytes 0x004e7c40..0x004e7cd5`:
sets `[0x00a83e00]=0x7e9f24`, `[0x00a83e14]=0xa`). It is **separate** from the main
`g_AnimClass_Array` @ `0x00a8e9ac` (count `0x00a8e9b8`), where `AnimClass::Constructor`
(`0x00421ea0`) registers *every* new anim.

How an anim gets into `DAT_00a83e04` (verified `decompile_function 0x004d7d50`,
`FootClass__ClickedAction_Cell`, case 1 / 0x3e — a move/attack click on a playfield cell):
1. Build the MoveFlash anim from `Rules+0xbb4` via `AnimClass__Constructor` (which
   registers it into `g_AnimClass_Array` like any anim), having first set scenario field
   `Scen+0x214 = 0xfffffffd` and the anim-type's `+0x340 = 0xffffec78` (a special z-adjust)
   **only when `g_GameMode != 0 && g_GameMode != 5`**.
2. Then, **only when `g_GameMode != 0 && g_GameMode != 5`**: find+remove it from
   `g_AnimClass_Array` and push it into `DAT_00a83e04`/`DAT_00a83e10`.

So the same mode gate that admits the anim into this vector also gates the driver — i.e. in
modes 0 and 5 the vector is never populated *and* never ticked. The destructor side mirrors
this: `AnimClass::~AnimClass` (`0x00422a60`) removes from `DAT_00a83e04` only when the
object's RTTI ID `== -2` (`AbstractClass__IRTTITypeInfo_GetID`), otherwise from
`g_AnimClass_Array` — confirming the relocated MoveFlash anims carry RTTI ID `-2` while in
this vector.

### `AnimClass::AI` body (driver, `0x00423ac0`)

Verified `decompile_function 0x00423ac0` + `disassemble_function 0x00423ac0`. Per object it
runs the standard anim per-tick state machine (the receiver's `AnimTypeClass*` is at
`this+0xC8`):
- looping-sound update; `BounceAI` + `ObjectClass::AI` when the type is a bouncer
  (`type+0x354`, e.g. meteors);
- attached-to-tiberium / hides-if-no-tiberium visibility flags (`type+0x373/0x359`);
- meteor/projectile **impact** branch (`this+0x194` set, height/cell test): spawns
  splash/debris anims, applies area damage, and on certain types scatters tiberium/overlay;
- delay countdown (`this+0x184`) → `AnimClass::Middle`;
- CDTimer frame advance: `CurrentFrame += FrameStep` when the timer expires, reload from
  Rate; loop/end/next-anim chain transitions (`type+0x2c8` next-anim) and self-destruct on
  completion (`this+0x179 = 1`).

For the MoveFlash anims actually living in this vector, almost all of the data-driven
sub-branches are inactive (MoveFlash is a simple non-bouncing, non-splittable,
non-tiberium, single-shot anim), so the per-tick cost is just the frame/timer advance and
eventual self-removal.

## Gate — confirmed

Spine claim "`g_GameMode != 0 && g_GameMode != 5`" is **correct**, verified in the disasm
above (`0x0055b61b..0x0055b627`). `0x00a8b238` is `g_GameMode` (the `MultiplayerGameMode`
selector; `list_globals GameMode`), distinct from `g_ScenarioClass_Instance` at
`0x00a8b230`. Observed active values 1/2/3/4 (skirmish/campaign/LAN/network modes — see
`Main_Game` @ `0x0048ce2d` and `FUN_0055cfd0`); 0 = not-in-game/shell (set to 0 on game
end). Value 5 is the only nonzero mode that suppresses this rung; in a normal YR skirmish
GameMode is a non-5 active value, so the rung is live every tick the vector is non-empty.

Contrast with **rung T** (the MAIN object vector, loop `0x0055b5fb..0x0055b619` over
`param_1+0x4`/`param_1+0x10`), which has **no mode gate** — confirming the spine's "U is
mode-gated, separate from T."

## RNG — `Scen->Random`, conditional

`AnimClass::AI` calls `Random__RandomRanged` (`0x0065c7e0`, `__thiscall` — RNG instance in
ECX). At **every** call site inside `AnimClass::AI` the ECX receiver is loaded as
`[0x00a8b230] + 0x218` = `g_ScenarioClass_Instance + 0x218` = **`Scen->Random`**
(verified `disassemble_function 0x00423ac0`):
- `0x00423f4f` (`MOV ECX,[0x00a8b230]; ADD ECX,0x218`) — debris count draw #1,
  `RandomRanged(0, type+0x2f4)`.
- `0x00423f72` (`MOV EAX,[0x00a8b230]; LEA ECX,[EAX+0x218]`) — debris count draw #2.
- `0x004240fa` (`LEA ECX,[EAX+0x218]`) — tiberium-spread overlay frame `RandomRanged(0,3)`.
- `0x00424140` (`LEA ECX,[EDX+0x218]`) — tiberium-spread overlay damage-state
  `RandomRanged(0,2)`.
- `0x004247d4` (`LEA ECX,[EAX+0x218]`) — loop randomized delay `RandomRanged(type+0x2dc, type+0x2e0)`.
- `0x004248ab` (`LEA ECX,[EDX+0x218]`) — next-anim randomized rate `RandomRanged(type+0x2e4, type+0x2e8)`.

`Random__RandomRanged` reads/advances the RandomClass state held at the ECX pointer
(`decompile_function 0x0065c7e0` — operates on `*param_1` state words at `+0x4/+0x8/+0xc`),
so the stream is fully determined by ECX. **`Scen->Random` for all six** — none route to
`g_MainRng` or `g_MapGenRng`. Consistent with the project's RNG-routing truth note
(gameplay callsites bind `Scen->Random` per ECX).

Draw counts per object per tick:
- **Common case (MoveFlash, the actual occupants of this vector): 0 draws.** MoveFlash has
  no splittable/spread/random-delay/random-rate fields set, so none of the six branches
  fire. The rung is RNG-inert in normal play.
- Splittable/meteor impact path: up to **2** draws (debris counts) — only for anim types
  with `type+0x2f0`/`+0x2f4` set, which are not in this vector under stock YR.
- Tiberium-spread path: **2 draws per qualifying neighbour cell** (overlay frame + state) —
  only for `type+0x358` Tiberium-spawn anims; not MoveFlash.
- Chain transition: **1** draw (random loop delay) or **1** draw (next-anim random rate),
  again only when those INI-driven fields are non-zero.

Because the vector under stock YR holds only MoveFlash anims, the **expected lockstep RNG
consumption of rung U is 0 draws/tick**. The non-zero paths exist in the driver but are
unreachable from this vector's contents in a stock skirmish (they fire for general anims
ticked under rung T instead). Note: `AnimClass::Constructor` (`0x00421ea0`) *also* draws
`Scen->Random` (next-anim rate, plus `Random__Next` bounce scatter) — but that is at
creation time (rung A/click handling / other constructors), not in this per-tick driver.

## Active-in-YR / TS-legacy

**Active in YR, not TS legacy.** The occupants of this vector are the move/attack
click-feedback cursor animations (`Rules->MoveFlash`), produced by `FootClass::ClickedAction`
on essentially every ordered move in a live game. They are visible every match (the brief
flash at the click target). The driver `AnimClass::AI` is the same per-tick AI used by all
anims. The only suppression is `g_GameMode == 0` (shell) or `== 5`; in a normal skirmish
the rung runs. No `SpecialFlags`/`FogOfWar` gate, no TS-only flag on the driver, gate, or
the vector's producer/consumer.

## Label-drift / pitfalls

- The driver is `vt+0x5c`, which for AnimClass is `AnimClass::AI` @ `0x00423ac0` (the plate
  comment on `0x00423ac0` calls it "vtable[24], offset 0x60" — that is `+0x60` counting the
  4 secondary-vtable slots differently; the **actual primary-vtable byte offset used by the
  spine is `+0x5c`**, verified by reading the slot at `0x007e33b0`). Trust the address
  `0x00423ac0`, not the slot-numbering wording.
- Do **not** conflate `DAT_00a83e04` (this rung's subset vector) with `g_AnimClass_Array`
  (`0x00a8e9ac`). The main array is ticked as ordinary objects elsewhere; this rung ticks
  only the relocated RTTI-(-2) MoveFlash subset.
- `0x00a8b238` (gate = `g_GameMode`) vs `0x00a8b230` (= `g_ScenarioClass_Instance`, RNG
  base) differ by 8 bytes; the decomp prints both as bare globals — verify the address.

## Ghidra calls cited

- `decompile_function 0x0055AFB0`, `disassemble_function 0x0055AFB0` — body site, order T→U→V, gate disasm `0x0055b61b..0x0055b649`.
- `read_memory 0x007e33b0` — vtable `+0x5c` slot = `0x00423ac0` (`AnimClass::AI`).
- `list_globals vtable__AnimClass` — primary vtable `0x007e3354`.
- `decompile_function 0x00423ac0`, `disassemble_function 0x00423ac0` — driver body + per-callsite RNG ECX = `[0x00a8b230]+0x218`.
- `decompile_function 0x0065c7e0` — `Random__RandomRanged` is `__thiscall` (RNG instance in ECX).
- `get_xrefs_to 0x00a83e04`, `get_xrefs_to 0x00a83e10`, `analyze_data_region 0x00a83e04` — vector identity, writers, ARRAY classification.
- `decompile_function 0x00422a60` — `AnimClass::~AnimClass`: removes from `DAT_00a83e04` iff RTTI ID == -2, else from `g_AnimClass_Array`.
- `decompile_function 0x00421ea0` — `AnimClass::Constructor` registers into `g_AnimClass_Array` (not this vector); also draws `Scen->Random` at construction.
- `decompile_function 0x004d7d50` — `FootClass__ClickedAction_Cell`: builds MoveFlash (`Rules+0xbb4`), then (gated `GameMode != 0 && != 5`) moves it from `g_AnimClass_Array` into `DAT_00a83e04`.
- `get_assembly_context 0x004e7c67/0x004e7c91/0x004e7cc7`, `disassemble_bytes 0x004e7c40..` — vector static-init (`DAT_00a83e00` vtable `0x7e9f24`, capacity `0xa`).
- `list_globals GameMode` — `g_GameMode @ 0x00a8b238`; `list_globals ScenarioClass` — `g_ScenarioClass_Instance @ 0x00a8b230`; `list_globals g_AnimClass_Array` — `0x00a8e9ac`.
- `decompile_function 0x0048ce2d` (`Main_Game`), `decompile_function 0x0055d300` (`FUN_0055cfd0`) — GameMode value semantics (1-4 active, 0 = not-in-game).
