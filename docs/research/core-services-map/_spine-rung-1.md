# Spine Rung #1 — "A. Cell-action / map-trigger event scan + SW-ready poll"

Part of the `LogicClass::PerTickUpdate` ordered ladder (the per-tick + RNG-draw lockstep
contract). This file documents **rung #1 of 28** only.

Body site: `LogicClassPerTickUpdateLiveVector` @ **0x0055AFB0** (the per-tick fan-out;
sole caller is `Main_Tick` @ 0x0055d360 per the ladder context — body verified via
`decompile_function 0x0055AFB0` + `disassemble_function 0x0055AFB0`).

Driver: **0x006e53a0** (`TechnoClass__ProcessCellAction`). NOTE: the label is **drifted**
— this is not a TechnoClass method and has nothing to do with sidebar placement (see
"Plan / label corrections").

---

## Verdict (one line)

A **map-trigger / Tag event-evaluation** rung: the body walks the global **TagClass array**
(`DAT_008b40cc[]`, count `DAT_008b40d8`) and, per tag, calls the driver with a set of
**trigger event-type codes**; the driver evaluates that tag's trigger conditions and, when
they pass, executes the trigger's actions and plays its EVA/voice. **The driver itself
draws NO RNG.** Any RNG only enters if a *specific trigger action* that draws RNG actually
fires (rare, author-defined). **Inert in a normal skirmish** (no map triggers/tags present);
active on **campaign / scripted maps**. Live YR code path, not TS-legacy.

---

## What the rung is (body site, around order 1)

Disassembly verified via `disassemble_function 0x0055AFB0`. The outer loop is at
`0x0055afe6`–`0x0055b177`:

```
0055afbd: MOV ECX,[0x008b40d8]      ; ECX = TagClass count           (DAT_008b40d8)
0055afcd: MOV EDI,[0x00a8b230]      ; EDI = ScenarioClass instance   (g_ScenarioClass_Instance)
0055afd9: TEST ECX,ECX
0055afe0: JLE 0x0055b17d            ; GATE: count <= 0 -> skip the whole scan
0055afe6: MOV ECX,[0x008b40cc]      ; ECX = TagClass array base      (DAT_008b40cc)
0055afec: MOV ESI,[ECX + EAX*4]     ; ESI = this-tag = array[iterator]   (EAX = DAT_00a83cdc)
...
0055afef: MOV AL,[EDI + 0x34be]     ; placement/mode byte Scen+0x34be ...
0055aff5: TEST AL,AL                ;   ... gates whether event-code 0x32 is tried
0055aff9: MOV EDX,[0x00abccd8]      ; EDX = context object (param_3)  (DAT_00abccd8)
0055b006: PUSH 0x32                 ; event-type code 0x32
0055b008: MOV ECX,ESI              ; ECX/this = the TagClass
0055b00a: CALL 0x006e53a0          ; <-- THE DRIVER
0055b00f: TEST AL,AL; JNZ ...      ; short-circuit: if it returned non-zero, skip rest
...
0055b164: MOV EAX,[0x00a83cdc]; INC EAX; CMP EAX,[0x008b40d8]; JL 0x0055afe6  ; loop
```

So per tag the body does a **short-circuit cascade** of driver calls, each with a different
**event-type code**, gated by scenario placement/mode bytes:

| Scen gate byte        | event codes tried (push value)                | meaning (trigger event class)            |
|-----------------------|-----------------------------------------------|-------------------------------------------|
| `Scen+0x34be`         | `0x32` (50)                                    | one event class                           |
| `Scen+0x34aa`         | `0x1b 0x1c 0x24 0x25` (27,28,36,37)            | a group of cell/area events               |
| `Scen+0x34ab`         | `0x2d 0x2e` (45,46)                            | another pair                              |
| (unconditional)       | `0xd 0x33` (13,51)                             | always tried each tick                    |
| SW-ready poll         | `0xe` (14)                                     | gated by the `Scen+0x11e8` timer (below)  |

The SW-ready `0xe` call (`0055b148`–`0055b159`) only fires when the scenario timer slot
`Scen+0x11e8` (decomp index `0x47a`) has elapsed — this is the **same slot** that rung #2
(0x004f42f0) consumes; here it is read-only to decide whether to issue the `0xe` event.

Receiver / args of every driver call: `ECX/this = ESI = TagClass`, stack args
`(0x??, 0, DAT_00abccd8, 0, 0)` where the first is the event-type code and `DAT_00abccd8`
is a **context object** (read-only here; the "by whom / on whom" pointer passed into
condition evaluation). `DAT_00abccd8` is set in the LogicClass-AI prologue just above the
body (`WRITE @ 0x0055aef2`, verified via `get_xrefs_to 0x00abccd8`); it is never an RNG
receiver.

**`DAT_008b40cc` is the TagClass array** — confirmed because `TagClass__Destructor`
(@ 0x006e4fbb) reads it (`get_xrefs_to 0x008b40cc`).

**Gate of the *rung*** (confirmed, corrected): `DAT_008b40d8 > 0` (tag count). The
plan's "inner dispatch gated by Scen+0x34be/0x34aa/0x34ab placement-mode bytes" is correct
as the **per-event-code sub-gating** described above — but those bytes select *which event
codes* are tried, they are not the rung's entry gate.

---

## The driver — `TechnoClass__ProcessCellAction` @ 0x006e53a0 (label drifted)

Body `0x006e53a0`–`0x006e5558`, verified via `decompile_function 0x006e53a0`,
`get_function_by_address 0x006e53a0`, `get_function_callees 0x006e53a0`. `__thiscall`:
`ECX/this = param_1 = the TagClass`; stack args `(event_code, ?, context, cell?, ?)`.

What it does (one line): for one Tag, walks its attached **trigger-condition list**, and if
the conditions for the given event-type pass, plays the trigger's voice and queues/executes
its actions; returns 1 if it fired.

**Driver gate (corrected vs plan):** the *driver itself* gates on
`g_IsMapEditor == 0 && *(char*)(this+0x35) == 0 && *(char*)(this+0x34) == 0` (re-entrancy /
disabled flags on the Tag), then `*(int*)(this+0x24) != 0` (the Tag has a linked
TriggerType). It sets `this+0x35 = 1` as a recursion guard during the walk. It does **not**
read the Scen placement bytes — those are read by the *body site*, not the driver.

Per-condition logic uses `*(int*)(*(int*)(this+0x24)+0x9c)` = the trigger's **repeat/persist
type** (0 = one-time/OR-fire, 1 = one-time/AND-fire requiring `this+0x2c==1`, 2 = repeating),
which decides whether the action is added to the fire-queue (`DynamicVectorClass__Add`) and
whether `bVar4`/`bVar3` are latched. On loop end it calls `Detach_From_All_Lists` and, if it
fired, registers the Tag into a deferred list `DAT_008b40cc`-family vector.

Callees (verified via `get_function_callees 0x006e53a0`):
`TriggerActionEntry__EvaluateConditions` @ 0x007264c0, `TriggerActionEntry__PlayVoiceForObjects`
@ 0x007265c0, `DynamicVectorClass__Add` @ 0x00726720, `MapClass__Get_CellClass` @ 0x005657a0,
`Detach_From_All_Lists` @ 0x007258d0, `FUN_00485250`, `FUN_005f5b50`.

---

## RNG

**The driver and its direct condition/voice callees draw NO RNG.**

- `TriggerActionEntry__EvaluateConditions` @ 0x007264c0 (verified `decompile_function`):
  walks condition entries, calls `HouseClass__Find_By_Country_Index` and
  `TriggerCondition__Evaluate` + small flag helpers (`FUN_0071fa30/0071f950/0071f9c0/00726400`).
  No `Scen->Random` / `g_MainRng` / `g_MapGenRng` receiver in the body.
- `TriggerActionEntry__PlayVoiceForObjects` @ 0x007265c0 (verified `decompile_function`):
  loops the trigger's action list, calls `HouseClass__Find_By_Country_Index` then
  `TriggerAction__Execute`. No RNG draw in this function.

RNG **can** be consumed transitively only if a fired action inside
`TriggerAction__Execute` @ 0x006dd8b0 happens to be an RNG-drawing action (its callee set
includes `ChronoSphere__WarpUnitsAtCell`, `CreateRadarEvent`, sell/online/offline, spawn
helpers, etc. — verified via `get_function_callees 0x006dd8b0`). That is **conditional on an
author-placed trigger actually firing this tick**, not a property of the rung. For the
lockstep draw-order contract: **rung #1 contributes zero RNG draws in the common case
(skirmish, no/inert triggers)**; on scripted maps the draws are whatever the fired action
types perform, in tag-array order then per-tag action-list order.

Which stream a fired action would use is **per-callsite ECX** inside `TriggerAction__Execute`
and was not exhaustively traced here (out of scope for a single rung); it is not the
driver's own stream. Marked **conditional/unknown** accordingly.

---

## Active in YR? TS legacy?

**Active in YR: conditional.** This is the live **map trigger/tag event system**. In a
**normal multiplayer/skirmish match the global TagClass array is empty** (no author triggers),
so `DAT_008b40d8 == 0` and the rung is **skipped entirely each tick** (gate at `0055afe0`).
On **campaign / custom scripted maps** it is fully live and player-visible (reinforcements,
mission EVA voices, win/lose triggers, "entered by" / "discovered by" / "attacked by" cell
events). It is therefore *reachable and visible* in YR, but only when the map defines tags.

**TS legacy: no (mechanism is live).** The trigger/tag system is the standard RA2/YR
mission-scripting engine, not a dead TS path. Individual *trigger-action types* dispatched by
`TriggerAction__Execute` may include actions never used by YR maps (some TS-era action
indices), but the rung's machinery is current.

---

## Plan / label corrections

- **Label drift:** driver `0x006e53a0` is named `TechnoClass__ProcessCellAction`, which is
  **misleading** — it is **not a TechnoClass method** and is **unrelated to sidebar/placement
  cell-action scanning**. It is a **TagClass per-event trigger-evaluation method**
  (`this = TagClass`). Recorded as label drift; trust the body, not the name.
- **Rung purpose:** plan titles it "Sidebar/placement cell-action scan + SW-ready poll."
  The actual purpose is **map-trigger / Tag event evaluation** (walk `DAT_008b40cc[]`
  TagClass array, evaluate per-tag trigger conditions for a set of event codes, fire voices
  + actions). The "SW-ready poll" sub-piece is real but is just the single `0xe`-event call
  gated on the `Scen+0x11e8` timer.
- **What it walks:** `DAT_008b40cc[]` = the **TagClass array** (confirmed via
  `TagClass__Destructor` xref), count `DAT_008b40d8`, iterator `DAT_00a83cdc`. The plan's
  "walks DAT_008b40cc[] count DAT_008b40d8" is correct.
- **Driver gate vs body gate:** the *body* sub-gates event codes on `Scen+0x34be/0x34aa/
  0x34ab`. The *driver's own* gate is different: `g_IsMapEditor==0 && Tag+0x34==0 &&
  Tag+0x35==0 && Tag+0x24 (TriggerType) != 0`. Both verified.
- **RNG:** driver + EvaluateConditions + PlayVoiceForObjects draw **no** RNG. Any RNG is
  transitive-through-a-fired-action only (conditional), so the rung is **RNG-neutral in the
  common case**.
