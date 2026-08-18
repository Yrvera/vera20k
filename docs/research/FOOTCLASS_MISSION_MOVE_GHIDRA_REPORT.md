# FootClass::Mission_Move — Audited Ghidra Research Report

**Original report:** 2026-04-23  
**Two-pass audit and correction:** 2026-07-20  
**Binary:** retail Yuri's Revenge `gamemd.exe`, PE x86 little-endian, image base `0x00400000`  
**Primary function:** `0x004D4200`  
**Status:** **CORRECTED**  
**Confidence:** HIGH for the bounded Mission Move body, concrete ground-class bindings, timer formula, and RNG receiver. Broader destination, aircraft, and arrival-tail semantics are explicitly outside this audit.

## Verdict

The old report got the small base handler mostly right but attached several wrong system-level conclusions to it.

The corrected mechanism is:

- Mission id `2` dispatches virtual slot `+0x22C` only when the per-object mission timer is due.
- For a `UnitClass` object, slot `+0x22C` is a concrete wrapper at `0x00740A90`, not direct inheritance of `FootClass::Mission_Move`.
- The base handler polls ILocomotion slot `+0x10`, which is `Is_Moving`, only when `NavCom == 0`.
- Its timer return is `ftol(Rate * 900.0) + Scenario.RandomRanged(0, 2)`.
- The RNG receiver is the embedded `RandomClass` at `ScenarioClass + 0x218`. It is gameplay-deterministic state, not a Rules/noncritical cosmetic stream.
- Stock `[Move] Rate=.016` produces delays of 14, 15, or 16 **frame-counter counts**. No fixed-FPS or wall-time conversion is established here.
- This delay throttles the **Move mission handler**, not `ILocomotion::Process`. The locomotor has an independent Process opportunity later in the same Foot AI turn.

That last distinction is load-bearing for VERA20k movement parity.

## Scope and method

This was an `audit` plus an `exhaustive-slice` re-investigation. Pass 1 enumerated the prior report's load-bearing claims without editing. Pass 2 checked the active binary, current INIs, current Rust, and newer research, then rewrote this report.

Ghidra was read-only. Every binary call named below explicitly targeted `program="gamemd.exe"`. Local Ghidra labels were treated as hints; class identity came from RTTI/vtable bytes and behavior from bodies/callsites.

## Pass 1 — legacy claim inventory

| ID | Legacy claim | Audit result | Correct disposition |
|---|---|---|---|
| C01 | `0x004D4200` is the Foot Move handler. | CONFIRMED | Foot RTTI and slot `+0x22C` resolve to it. |
| C02 | Mission enum `2` selects slot `+0x22C`. | CONFIRMED | Dispatch jump-table case 2 calls `[vtable+0x22C]`. |
| C03 | Every mobile unit uses `0x004D4200` directly. | WRONG | Unit and Infantry have concrete `+0x22C` bindings before any base tail-call. |
| C04 | The handler moves the object. | CONFIRMED AS NEGATIVE | It does not call Process or mutate position. |
| C05 | `+0x5A4`, `+0x674`, `+0xB4`, `+0xAC` are the fields read by this slice. | CONFIRMED | Direct byte-offset accesses remain exact. |
| C06 | Locomotor slot `+0x10` is `Is_Moving_Now`. | WRONG | The protocol slot is `Is_Moving`; `Is_Moving_Now` is `+0x80`. |
| C07 | `0x004B6610` proves the concrete `+0x10` implementation. | WRONG | Drive ILocomotion `+0x10` resolves to `0x004AFB80`. |
| C08 | Foot slot `+0x484` resolves to `0x004D82B0`. | CONFIRMED | Fresh vtable bytes and body decompile confirm the arrival hook. |
| C09 | With live NavCom, the handler skips the locomotor poll. | CONFIRMED | It goes directly to the timer path. |
| C10 | With null NavCom and null locomotor, the assert safely returns. | MISLEADING | It calls Assert, then execution continues toward a dereference; null is an invariant violation. |
| C11 | Stopped locomotor plus no queued mission calls arrival and returns 1. | CONFIRMED | Exact branch at `0x004D422A..0x004D424E`. |
| C12 | A queued mission suppresses the arrival hook and uses the timer path. | CONFIRMED | `QueuedMission != -1` branches to `0x004D424F`. |
| C13 | Formula is `ftol(Rate * 900) + RandomRanged(0,2)`. | CONFIRMED | Operand order and constant bytes are exact. |
| C14 | `ftol` means truncation here. | CONFIRMED | `Math::ftol` loads x87 control word `0x0E7F` and uses `FISTP`; RC is toward zero. |
| C15 | `0x007E27F8` is the double `900.0`. | CONFIRMED | Raw bytes `0000000000208c40`. |
| C16 | `0x00A8B230` is the Rules singleton. | WRONG | It is assigned the constructed Scenario singleton. |
| C17 | Receiver `+0x218` is noncritical/cosmetic RNG. | WRONG | Scenario construction seeds the embedded RandomClass there; it is deterministic gameplay state. |
| C18 | One `RandomRanged` call means exactly one raw RNG state step. | WRONG | For range 0..2, masked candidate 3 is rejected, so raw advancement is one-or-more. |
| C19 | Jitter exists to desynchronize groups. | UNCHECKED | Plausible intent, but not binary mechanism evidence. Removed as an authoritative claim. |
| C20 | 14–16 counts equal roughly one second at fixed 15 FPS. | STALE/WRONG | Only frame-counter counts are proven; no fixed wall-time conversion is authorized. |
| C21 | Returning 1 means the next frame-counter count is eligible. | CONFIRMED WITH CONDITION | Dispatch stores start=current frame and delay=1; normal later object service is still required. |
| C22 | Dispatch gates only active byte and health. | WRONG | It first executes `ObjectClass::AI`; active is checked next, timer next, and health only on the due path. |
| C23 | Mission Move runs once per tick. | WRONG | Mission Dispatch is entered per eligible Techno turn; the handler runs only when its timer is due. |
| C24 | Dispatch-not-due skips Techno post work. | WRONG | It returns to the instruction after the call, so Techno post work continues. |
| C25 | Mission-control table has 32-byte entries; Rate is `+0x10` and AARate `+0x18`. | CONFIRMED | Lookup is current mission * 8 dwords; Read_INI stores both doubles. |
| C26 | Rate's semantic unit is proven to be “minutes.” | MISLEADING | The verified mechanism is stored double times 900 into frame-counter counts; the prose unit label is unnecessary. |
| C27 | AARate can be globally ignored. | UNCHECKED | Mission Move reads Rate; broader AARate consumers were not audited here. |
| C28 | UnitClass inherits Foot Move unchanged. | WRONG | Unit slot `+0x22C` is `0x00740A90`. |
| C29 | `0x00740A90` is merely a direct `Mission_Guard` caller. | WRONG | Despite its polluted local label, it is Unit's concrete Move slot wrapper. |
| C30 | Infantry `+0x6C4` is a byte sequence field. | WRONG | The override reads it as a dword. |
| C31 | Infantry virtual `+0x3C` updates animation sequence. | WRONG | Infantry's concrete `+0x3C` returns `this+0x21C`, the House/owner pointer used by `IsPlayerControl`. |
| C32 | Values `0x1B..0x1E` have the named prone/panic meanings listed. | UNCHECKED | Numeric branches are exact; semantic names were not re-established and are removed. |
| C33 | The full Aircraft Move state machine in the old report is current authority. | UNCHECKED | Aircraft is outside Checkpoint A; use a dedicated, freshly audited aircraft report. |
| C34 | OnArrival runs “exactly once” because `+0x6B3` is permanent. | MISLEADING | Foot AI clears `+0x6B3` after the Techno-return alive check; it is a per-pass/reentrancy guard. |
| C35 | Move's timer cadence throttles actual movement. | WRONG | Locomotor Process is later in Foot AI and is not gated by Mission Move's return delay. |
| C36 | A ~1-frame settle pause is proven. | WRONG | No runtime measurement or native executable trace establishes that claim. |
| C37 | The old Rust status and links describe the current tree. | STALE | Current Rust now has a host shell/classifier, but still lacks the native body and interleaving. |
| C38 | The detailed Set_Destination/NavQueue producer inventory was verified by this audit. | UNCHECKED | Preserved only as external prior work; it is not part of this report's authority. |
| C39 | Local function labels alone establish Unit/Foot/Infantry identity. | WRONG METHOD | RTTI, vtable bytes, receiver flow, and bodies are the evidence. |

## Pass 2 — corrected binary contract

### Concrete slot identities

| Receiver | Verified vtable base | `+0x22C` target | Meaning in this slice |
|---|---:|---:|---|
| FootClass | `0x007E8C94` | `0x004D4200` | Base Move handler |
| UnitClass | `0x007F5C70` | `0x00740A90` | Unit Move wrapper, then possible Foot tail-call |
| InfantryClass | `0x007EB058` | `0x0051F660` | Infantry Move override, then possible Foot tail-call |

Evidence:

- `read_memory(0x007E8C90,8)` plus COL `0x00800948` and TypeDescriptor string `.?AVFootClass@@`; `read_memory(0x007E8EC0,8) -> 0x004D4200`.
- `read_memory(0x007F5C6C,8)` plus COL `0x0080CC68` and `.?AVUnitClass@@`; `read_memory(0x007F5E9C,8) -> 0x00740A90`.
- `read_memory(0x007EB054,8)` plus COL `0x008033B8` and `.?AVInfantryClass@@`; `read_memory(0x007EB284,8) -> 0x0051F660`.

### Exact Foot body

Equivalent control flow, without assigning unproved names to other slots:

```text
if this.NavCom(+0x5A4) == null {
    if this.ActiveLocomotor(+0x674) == null {
        Assert(E_POINTER);             // execution then continues
    }

    moving = ActiveLocomotor.vtable[+0x10]();  // ILocomotion::Is_Moving

    if moving == 0 && this.QueuedMission(+0xB4) == -1 {
        this.vtable[+0x484](0, 1);      // Foot arrival hook
        return 1;
    }
}

entry = GetMissionTimerEntry(this.CurrentMission(+0xAC));
base  = Math::ftol(entry.Rate(+0x10) * 900.0);
jitter = Scenario(+0x218).RandomRanged(0, 2);
return base + jitter;
```

Evidence: `decompile_function(0x004D4200)`; full disassembly; `get_assembly_context` at `0x004D423A` and `0x004D4266`; `get_function_callees(0x004D4200)`.

For Drive, ILocomotion `+0x10` resolves to `0x004AFB80` via `read_memory(0x007E7EC0,8)`. That function returns moving when the head-to coordinate is non-null, or when the current-cell coordinate is non-null and its X/Y differ from the owner's X/Y. Evidence: `decompile_function(0x004AFB80)`.

Foot `+0x484` resolves to `0x004D82B0` via `read_memory(0x007E9118,8)`. Its fresh decompile confirms the `+0x6B3` reentrancy guard, optional queue pop, target/infantry branches, and final `+0x544(0,0)` call. Those tail semantics are adjacent to, not part of, the timer formula.

### Mission timer and RNG

`MissionClass::GetMissionTimerEntry @ 0x005B3A00` returns:

```text
&g_MissionControl_Array + CurrentMission * 8 dwords
```

That is a 32-byte entry. `MissionControlClass::Read_INI @ 0x005B3760` reads `Rate` as a double at `+0x10` and `AARate` at `+0x18`; zero AARate copies Rate.

`Math::ftol @ 0x007C5F00` uses x87 `FISTP` under control word `0x0E7F`, so the conversion is toward zero. The multiplier at `0x007E27F8` is exactly `900.0`.

Effective stock YR data is:

```ini
[Move]
Rate=.016
```

Evidence: `ini/rulesmd.ini:30484-30485`. Base `rules.ini` agrees at `22635-22636`.

For this positive input:

```text
ftol(.016 * 900.0) = ftol(14.4) = 14
return = 14 + {0,1,2}
```

These are delay counts compared against `g_CurrentFrameCounter`. They are not a proven wall-clock duration.

The callsite loads `[0x00A8B230]`, forms receiver `+0x218`, and calls `RandomRanged(0,2)` at `0x004D4275`. `Init_Game` constructs `ScenarioClass @ 0x006832C0` and stores the returned object at `0x00A8B230`; the constructor seeds the embedded RNG at `this+0x218` (`0x006832C9..0x006832CF`). Evidence: `get_assembly_context` at `0x004D4266`, `0x0052BA94`, and `0x006832C9`.

`RandomRanged @ 0x0065C7E0` is inclusive and swaps reversed bounds. For 0..2 it makes one API call but masks raw values to two bits and rejects candidate 3. Therefore an enabled Scenario stream advances its raw XOR-lag state **one or more times** until the candidate is 0, 1, or 2. Evidence: `decompile_function(0x0065C7E0)`.

### Concrete Unit wrapper

Unit `+0x22C` is `0x00740A90`. The local label `UnitClass__Mission_Guard` is not trustworthy for slot identity.

Address-order behavior:

1. Read byte `this+0x6E0`, then clear byte `this+0x6D2`.
2. If the saved `+0x6E0` value or the subsequently read `+0x6E1` or `+0x6E2` byte is nonzero, call virtual `+0x1E8(5,0)` and return 1. MissionClass vtable work identifies `+0x1E8` as Queue_Mission; the exact meanings of the three gate bytes are deferred.
3. Otherwise use the embedded object at `this+0x350`:
   - call `0x004A51D0`;
   - if it returns zero, call `0x004A5240` with the two dwords at `Type(+0x6C4)+0x3C8/+0x3CC`;
   - tail-call `FootClass::Mission_Move @ 0x004D4200`.

Evidence: `read_memory(0x007F5E9C,8)`; `decompile_function` and `disassemble_function` at `0x00740A90`. This wrapper is part of an ordinary vehicle/MCV Move-dispatch fixture even when its default-state branches appear inert.

### Concrete Infantry override

Infantry `+0x22C` is `0x0051F660`.

- It reads `this+0x6C4` as a dword and tests `0x1B..0x1E`.
- In that numeric-state set, virtual `+0x3C` resolves to `0x006F9DC0`, which returns `this+0x21C`; that pointer is passed to `HouseClass::IsPlayerControl @ 0x0050B730`.
- Player-controlled path: virtual `+0x480(0,1)`, return 1.
- Non-player path with signed `*(Type(+0x6C0)+0x6C4) < 0`: virtual `+0x558(0x1F,0,0)`, then return `*(*(Type+0xE3C)+0x460)`.
- Otherwise tail-call `0x004D4200`.

Evidence: `read_memory(0x007EB284,8)` and `read_memory(0x007EB094,8)`; `decompile_function(0x0051F660)`, `decompile_function(0x006F9DC0)`, and `decompile_function(0x0050B730)`. Numeric behavior is verified; old animation-state names are not.

## Dispatch integration

`MissionClass::Mission_Dispatch @ 0x005B3060`:

1. calls `ObjectClass::AI @ 0x005F3E70` unconditionally;
2. reads byte `+0x90` and returns if zero;
3. tests the `+0xC8` start / `+0xD0` delay timer against `g_CurrentFrameCounter`;
4. if not due, returns without reading health or invoking a mission handler;
5. when due, requires signed dword `+0x6C > 0`;
6. for current mission 2, calls virtual `+0x22C`;
7. stores current frame at `+0xC8`, an uninitialized stack scratch dword at `+0xCC`, and the handler return at `+0xD0`.

Evidence: `decompile_function(0x005B3060)`; `get_assembly_context` at `0x005B3060`, `0x005B30A1`, and `0x005B334E`; `read_memory(0x005B34E8,32)`. `get_function_callers(0x005B3060)` returns only `TechnoClass::AI_Update @ 0x006F9E50`.

The exact downstream consumer status of `+0xCC` was not re-audited exhaustively here. It is not read by this timer gate. Prior mission-verb research classifies it as dead scratch; that broader negative claim remains external evidence, not a new exhaustive proof.

## Cadence correction

The normal mobile-object path is not “Mission Move every 14–16 ticks, therefore movement every 14–16 ticks.”

It is:

```text
eligible object turn
  -> Techno common body
     -> Mission_Dispatch
        -> maybe Mission_Move, only if timer due
  -> Foot common body
     -> ILocomotion::Process, if five Foot gates pass
```

The Mission handler's return controls only its next dispatch eligibility. It does not gate the later locomotor Process opportunity. Newer scheduler research also finds one live-object pass per Main_Tick and no separate 15-Hz Drive gate. See `OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md`.

## Current Rust status

The old Rust section is superseded.

Current source has a live-order object host at `src/sim/world/techno_ai.rs:68` and a Unit bracket at `:525`, but it is not the native contract:

- `techno_common_pre` is empty (`:376`).
- The bracket performs one health-based `is_alive()` check, not the binary's segmented `+0x90` guards.
- It defers miners.
- It increments the tick counter and copies `derived_mission()` into mission state; it does not execute Mission_Dispatch or a concrete Move handler.
- It calls `techno_common_post` without a second alive check; that helper currently contains only the damage-Spark-related implementation (`:418`), not the full post-dispatch sequence.
- `unit_dispatch_family` in `src/sim/mission/dispatch.rs:50` is a pure classifier.
- `derived_mission` in `src/sim/game_entity.rs:559` is a projection from other machines, not native mission authority.
- Actual ground movement remains a global pass in `src/sim/movement/movement_tick.rs:831`, after the object host, rather than being interleaved inside each live object's Foot turn.

This is **DRIFT**, not an internal-only difference. Checkpoint A authorizes research only; it does not authorize a production flip.

## Supporting material not re-certified here

| Legacy section | Status after this audit |
|---|---|
| Full OnArrival tail | Core body freshly decompiled; semantic names for every tail slot remain outside this report. |
| Set_Destination_Internal, Stop_Moving, PointerExpired | Not re-audited. Use the dedicated navigation reports and re-verify before implementation. |
| NavQueue producer reachability | Not re-audited. Use the dedicated producer audit. |
| Aircraft Move override | Not re-audited; excluded from this ground checkpoint. |
| `0x0073B0B0` legacy label discussion | Removed from authority; concrete vtable bytes identify Unit Move at `0x00740A90`. |
| Wall-time observations | Removed; require executable/native measurement. |

## Open Questions Log

No item remains silently open.

- **[RESOLVED] OQ-01:** Foot handler identity and boundary — `0x004D4200`.
- **[RESOLVED] OQ-02:** Foot/Unit/Infantry `+0x22C` bindings — `0x004D4200` / `0x00740A90` / `0x0051F660`.
- **[RESOLVED] OQ-03:** Locomotor poll slot — `+0x10 Is_Moving`, not `Is_Moving_Now`.
- **[RESOLVED] OQ-04:** Arrival hook target — Foot `+0x484 -> 0x004D82B0`.
- **[RESOLVED] OQ-05:** Rate storage and stride — `+0x10` in a 32-byte mission-control entry.
- **[RESOLVED] OQ-06:** Conversion — multiply by 900.0 and x87 truncate toward zero.
- **[RESOLVED] OQ-07:** Stock Move result — 14/15/16 frame-counter counts.
- **[RESOLVED] OQ-08:** RNG owner — Scenario singleton embedded RandomClass at `+0x218`.
- **[RESOLVED] OQ-09:** Draw count — one API call; one-or-more raw state advances for 0..2.
- **[RESOLVED] OQ-10:** Timer-not-due behavior — no handler, no health read, normal return to Techno caller.
- **[RESOLVED] OQ-11:** Unit wrapper ordering — three bytes, embedded tracker, then possible Foot tail-call.
- **[RESOLVED] OQ-12:** Infantry `+0x3C` receiver — returns House/owner pointer.
- **[DEFERRED] OQ-13:** Exact semantic names of Unit bytes `+0x6E0..+0x6E2`. Reason: numeric gate is sufficient for Checkpoint A. Next step: focused Unit deploy-state field audit before production activation.
- **[DEFERRED] OQ-14:** Semantic names for Infantry numeric states `0x1B..0x1E`. Reason: branch behavior is exact and Infantry population work belongs to Checkpoint C.
- **[DEFERRED] OQ-15:** Exhaustive reader proof for Mission `+0xCC`. Reason: not a timer or host-order input. Next step: MissionClass field-use audit if byte-perfect storage is brought into scope.
- **[DEFERRED] OQ-16:** Exact name of OnArrival virtual `+0x544`. Reason: outside Move timer/host contract. Next step: dedicated arrival-tail vtable audit.
- **[DEFERRED] OQ-17:** Full Aircraft Move state machine. Reason: outside the ground Checkpoint A population.
- **[DEFERRED] OQ-18:** Native wall-time measurements. Reason: requires executable oracle/runtime work in Checkpoint E.

## Coverage and cold checks

Primary bodies read in this correction:

- `0x004D4200` Foot Move
- `0x005B3060` Mission Dispatch
- `0x005B3A00` mission timer lookup
- `0x005B3760` mission-control INI reader
- `0x0065C7E0` RandomRanged
- `0x007C5F00` ftol
- `0x006832C0` Scenario constructor context
- `0x004AFB80` Drive Is_Moving
- `0x004D82B0` Foot arrival hook
- `0x00740A90` Unit Move wrapper
- `0x0051F660` Infantry Move override
- `0x006F9DC0` Infantry owner-return slot
- `0x0050B730` House IsPlayerControl
- `0x005F3E70` Object AI

Cold spot-check 1 re-read `0x004D4200` plus Foot/Unit/Infantry `+0x22C` bytes after synthesis; no conclusion changed. Cold spot-check 2 re-read the dispatch due branch and Scenario RNG callsite; no conclusion changed. A final zero-add pass introduced no new Checkpoint-A question.

## Sources

### Direct binary evidence

- Ghidra `decompile_function`: `0x004D4200`, `0x005B3060`, `0x005B3A00`, `0x005B3760`, `0x0065C7E0`, `0x007C5F00`, `0x004AFB80`, `0x004D82B0`, `0x00740A90`, `0x0051F660`, `0x006F9DC0`, `0x0050B730`, `0x005F3E70`.
- Ghidra `disassemble_function`: `0x004D4200`, `0x005B3060`, `0x00740A90`.
- Ghidra `get_assembly_context`: `0x004D423A`, `0x004D4266`, `0x005B3060`, `0x005B30A1`, `0x005B334E`, `0x0052BA94`, `0x006832C9`.
- Ghidra `read_memory` / `inspect_memory_content` for RTTI, vtables, `0x007E27F8`, `0x00822D80`, and dispatch jump-table bytes as cited inline.
- Ghidra `get_function_callers` / `get_function_callees` for `0x004D4200` and `0x005B3060`.

### Data and current implementation

- `ini/rulesmd.ini:30439-30515`
- `ini/rules.ini:22635-22636`
- `src/sim/world/techno_ai.rs:68`
- `src/sim/world/techno_ai.rs:376`
- `src/sim/world/techno_ai.rs:418`
- `src/sim/world/techno_ai.rs:525`
- `src/sim/mission/dispatch.rs:50`
- `src/sim/game_entity.rs:559`
- `src/sim/movement/movement_tick.rs:831`
- `docs/research/OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md`
- `docs/research/ILOCOMOTION_COM_PROTOCOL_SPEC.md`
- `docs/research/MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md`
