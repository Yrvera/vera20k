# Radio Message 0x0D Senders / Anim Refresh - Ghidra Research Report

**Address(es):** `0x006F4A70` sender helper, `0x005F5320` ObjectClass receiver, `0x0043C2D0` BuildingClass receiver, `0x0043F180` BuildingClass vtable `+0x124`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** live `0x0D` radio send sites, receiver behavior through ObjectClass/BuildingClass, and the `WeaponsFactory=yes` swallow effect for building animation/production visuals.
**Non-Scope:** every BuildingClass `Receive_Radio` case, full BuildingClass `Mark`/rendering semantics, all non-radio `PUSH 0x0D` constants, and runtime frame capture of each animation transition.
**Confidence:** High for sender identity and receiver branch semantics; Medium for the exact player-visible animation frame effect because static analysis proves the refresh/swallow but not a captured frame diff.
**Active in YR:** Conditional for sending (`+0x418` set and Contacts[0] non-null); Yes for the stock war-factory swallow path (`GAWEAP`, `NAWEAP`, `YAWEAP` set `WeaponsFactory=yes`).

## 0. Investigation Contract

**Target question:** Who sends radio message `0x0D` in live `gamemd.exe`, what does the receiver do with it, and why do `BuildingClass` / `WeaponsFactory` paths swallow it?

**Non-goals:** Do not re-decode every BuildingClass radio case; do not implement Rust; do not audit all building animation slots; do not mutate Ghidra labels/comments.

**Evidence needed to mark COMPLETE:**

- Decompile plus assembly for at least one verified `0x0D` sender.
- Receiver decompile plus dispatch proof for ObjectClass `0x0D`.
- BuildingClass `0x0D` branch decompile plus `WeaponsFactory=yes` INI/default evidence.
- Xref/caller evidence explaining when the sender helper is live.
- Negative scan evidence that obvious immediate `0x0D` radio sends are not elsewhere.

**Stop conditions:** Stop after the bounded sender/receiver slice is drained; stop if further questions require runtime animation capture; do not expand into unrelated radio messages or full BuildingClass switch behavior.

## 1. Overview

Radio message `0x0D` is not a normal dock-state command. It is a synchronous "refresh the radio partner's object mark/animation state" side effect emitted by `TechnoClass__ProcessCloakAndNotify @ 0x006F4A70` after a successful object mark/update when the sender's `TechnoClass+0x418` dock/contact-entered byte is set.

The terminal receiver is `ObjectClass__Receive_Radio @ 0x005F5320`: message `0x0D` calls receiver vtable slot `+0x124` with argument `2` and returns `ROGER=1`. For buildings, slot `+0x124` resolves to `FUN_0043F180`, a BuildingClass mark/update routine that refreshes attached anim positioning/state. `BuildingClass::Receive_Radio` intercepts `0x0D` first and returns `1` with no side effects only when `BuildingType+0x16BD WeaponsFactory=yes`, preventing produced-unit contact churn from forcing the factory through the generic mark/anim refresh path.

## 2. Class Layout / Key Offsets

| Field / slot | Offset | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| Radio transmit-to-first slot | vtable `+0x274` | `Transmit_Radio_ToFirst(msg)`; sends to Contacts[0] only | slot docs; call at `0x006F4A91` | Yes |
| Object receiver slot | vtable `+0x194` | `Receive_Radio(sender,msg,payload)` | prior slot docs; receiver xrefs | Yes |
| Object mark/update slot | vtable `+0x124` | receiver-side action for msg `0x0D`; called with arg `2` | `0x005F5370..0x005F5374` | Yes |
| BuildingClass `+0x124` target | `0x007E3FE0 -> 0x0043F180` | Building mark/update routine | `get_xrefs_from 0x007E3FE0` | Yes |
| TechnoClass dock/contact-entered byte | `+0x418` | gates `0x0D` send; set by radio `0x18`, cleared by `0x19` | `0x006F4A81`, `0x006F4B72`, `0x006F4BA6` | Yes / Conditional |
| BuildingType WeaponsFactory flag | `+0x16BD` | makes BuildingClass swallow `0x0D` before ObjectClass | `0x0043C40B` decompile; INI stock WFs | Yes |

## 3. Core Logic

### 3.1 Sender: `TechnoClass__ProcessCloakAndNotify @ 0x006F4A70`

Decompile:

```text
uVar1 = ObjectClass__Mark(param_2);
if ((char)uVar1 != 0) {
    if ((char)param_1[0x106] != 0) {
        this->vtable[0x274](0x0D);
    }
    return 1;
}
return 0;
```

Because `param_1[0x106]` is a byte-indexed decompiler view, this is `TechnoClass+0x418`. The send is `Transmit_Radio_ToFirst(0x0D)`, not directed `Transmit_Radio(0x0D,target)`.

Assembly context:

```asm
006F4A81  MOV AL, byte ptr [ESI + 0x418]
006F4A87  TEST AL, AL
006F4A89  JZ  0x006F4A97
006F4A8B  MOV EDX, dword ptr [ESI]
006F4A8D  PUSH 0xD
006F4A8F  MOV ECX, ESI
006F4A91  CALL dword ptr [EDX + 0x274]
```

**Active in YR:** Conditional. This code is live in Techno-derived objects, but it sends only when `ObjectClass__Mark(param_2)` succeeds and `+0x418 != 0`. In standard YR, `+0x418` is set by radio `0x18` during refinery docking and stock war-factory exit contact setup.

### 3.2 Sender callers / when it can happen

`get_function_xrefs(0x006F4A70)` returned:

- `0x0043F198` and `0x0043F5DB` in `FUN_0043F180` (`BuildingClass` vtable `+0x124`).
- `0x004D3799` in `TechnoClass__DoCloak @ 0x004D3780`.
- DATA xref `0x007F4A84` from the TechnoClass vtable slot.

`TechnoClass__DoCloak` calls the helper for `param_2 != 2`; for `param_2 == 2` it returns `1` immediately. `BuildingClass +0x124` calls the helper at entry before doing its building-specific mark/update work. This explains why ObjectClass receiving `0x0D` calls `+0x124(2)`: for buildings it refreshes mark/attached anim state, while for unit-like Techno paths the `DoCloak(2)` wrapper is a no-op success.

**Active in YR:** Yes for the callable functions; Conditional for `0x0D` emission because it still requires `+0x418`.

### 3.3 ObjectClass receiver: msg `0x0D` means vtable `+0x124(2)`

`ObjectClass__Receive_Radio @ 0x005F5320` handles exactly `0x0D` and `0x22`. For `0x0D`, it does not inspect the sender or payload:

```text
if (msg == 0x0D) {
    this->vtable[0x124](2);
    return 1;
}
```

Assembly:

```asm
005F5320  MOV EAX, [ESP+8]
005F5327  CMP EAX, 0xD
005F532A  JZ  0x005F5370
...
005F5370  MOV EDX, [ECX]
005F5372  PUSH 2
005F5374  CALL dword ptr [EDX + 0x124]
005F537A  MOV EAX, 1
```

For BuildingClass receivers, `get_xrefs_from 0x007E3FE0` proves vtable `+0x124` resolves to `FUN_0043F180`. That function updates building mark/attached-animation state in mode `2`, including attached animation coordinate/state propagation through the building's 21-slot anim pointer block.

**Active in YR:** Yes when a message reaches ObjectClass. For stock war factories, BuildingClass intercepts first and prevents this call.

### 3.4 BuildingClass receiver / WeaponsFactory swallow

`BuildingClass__Receive_Radio @ 0x0043C2D0` has a direct case `0x0D`:

```text
case 0x0D:
    if (this->Type[0x16BD] != 0) return 1;
    break; // fall through to TechnoClass -> RadioClass -> ObjectClass
```

For `WeaponsFactory=yes`, the result is a `ROGER=1` return with no `ObjectClass +0x124(2)` call. For non-weapon-factory buildings, the switch falls through to `TechnoClass__Receive_Radio`, then `RadioClass__Receive_Radio`, then `ObjectClass__Receive_Radio`, so the generic mark/anim refresh still runs.

Stock YR evidence: `rulesmd.ini` has `WeaponsFactory=yes` and `Factory=UnitType` on `GAWEAP`, `NAWEAP`, and `YAWEAP` (`rulesmd.ini:11775`, `12565`, `13309`; clone sections also repeat the flag). These are normal land war factories.

**Active in YR:** Yes for stock war factories; Conditional for other BuildingClass receivers depending on whether they receive a contacted peer's `0x0D`.

### 3.5 Negative sender scan

The immediate scan found `PUSH 0x0D` at many addresses, but only `0x006F4A8D` is immediately paired with a radio transmit slot. A follow-up scan of all observed vtable `+0x274` call sites found only this `PUSH 0x0D -> CALL [vtable+0x274]` radio send. The other nearby `PUSH 0x0D` hits are animation/drawing/string/control constants or non-radio calls; for example `0x00459564` is an argument to `BuildingClass__CreateAnimForSlot @ 0x00451890`, followed separately by a `PUSH 3 -> CALL [vtable+0x274]`.

No `PUSH 0x0D` radio send was found for `+0x278` directed sends or `+0x27C` payload sends in this slice. This does not prove no dynamically computed `0x0D` ever exists, but it closes the prior open question for immediate/equivalent static radio dispatch: the live named sender is `TechnoClass__ProcessCloakAndNotify`.

## 4. INI Keys

| Key | Stock YR value | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `WeaponsFactory=` | `yes` on `GAWEAP`, `NAWEAP`, `YAWEAP` | Gates BuildingClass `0x0D` swallow | `rulesmd.ini` stock sections; `BuildingType+0x16BD` | Yes |
| `Factory=` | `UnitType` on stock WFs | Production category; not the swallow gate | `rulesmd.ini` stock sections | Yes |
| Building animation keys (`ActiveAnim*`, `ProductionAnim*`, `SpecialAnim*`) | present on many buildings | Data affected by `+0x124` mark/update refresh; not directly read by radio code | `art.ini`/`artmd.ini`; `FUN_0043F180` attached anim updates | Conditional |

There is no INI key for the sender-side `+0x418` byte; it is runtime radio state set/cleared by messages `0x18`/`0x19`.

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `TechnoClass__ProcessCloakAndNotify @ 0x006F4A70` | Sole verified immediate `0x0D` radio sender | decompile + `0x006F4A81..0x006F4A91` | Conditional |
| `TechnoClass__DoCloak @ 0x004D3780` | One caller; skips helper for arg `2` | xref `0x004D3799`; decompile | Yes |
| `BuildingClass +0x124 / FUN_0043F180 @ 0x0043F180` | Caller and ObjectClass receiver-side target for buildings | xrefs `0x0043F198`, `0x0043F5DB`; vtable `0x007E3FE0` | Yes |
| `ObjectClass__Receive_Radio @ 0x005F5320` | Terminal handler for `0x0D`: calls `+0x124(2)` | decompile + assembly `0x005F5370..0x005F537A` | Yes |
| `BuildingClass__Receive_Radio @ 0x0043C2D0` | Intercepts/swallow `0x0D` only for WeaponsFactory | decompile case `0x0D` | Yes / Conditional |
| `TechnoClass__Receive_Radio @ 0x006F4AB0` | Sets/clears `+0x418` through `0x18`/`0x19` | decompile; prior `+0x418` report | Yes |

## 6. Current Rust Implementation Status

Current Rust has contact-like state but not the generic radio side-effect layer:

- `src/sim/game_entity.rs` has per-entity `radio_contacts` and helpers `mark_live_contact_with`, `has_live_contact_with`, `clear_live_contact_with`.
- `src/sim/miner/miner_dock.rs` has refinery `contacts` and `contact_entered`, explicitly described as `+0x418`-like state for miners.
- `src/sim/production/production_spawn.rs::mark_war_factory_spawn_contact` marks produced-unit/factory contact for war-factory exit behavior.
- `src/app_building_anim.rs` and `GameEntity::building_anim_overlays` own current building overlay animation ticking/spawning.

Delta: Rust does not appear to have a generic "when contacted object is mark/unmark refreshed, send `0x0D` to Contacts[0]" layer, nor a receiver-side `ObjectClass` radio fallback that maps `0x0D` to building mark/attached-animation refresh while swallowing it for `WeaponsFactory=yes`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Sender helper `0x006F4A70` | verified | decompile + `0x006F4A81..0x006F4A91` | none |
| Sender callers/xrefs | verified | `get_function_xrefs 0x006F4A70` | none for scoped callers |
| ObjectClass `0x0D` receiver | verified | `0x005F5320`, `0x005F5370..0x005F537A` | exact non-building class effects by `+0x124` are out-of-scope |
| BuildingClass `0x0D` branch | verified | `0x0043C2D0` decompile | none |
| BuildingClass `+0x124` binding | verified | `get_xrefs_from 0x007E3FE0 -> 0x0043F180` | full mark routine semantics deferred |
| `WeaponsFactory=yes` stock data | verified | `rulesmd.ini` stock WF sections | none |
| Immediate `PUSH 0x0D` radio sender inventory | verified for immediate sends | `search_byte_patterns 6A 0D`; vtable-call context scan | dynamically computed message values not exhaustively proven absent |
| Current Rust surface | touched-not-exhausted | codegraph + `rg` scan | implementation not attempted |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-0D-001 - What sends radio msg 0x0D? -> `TechnoClass__ProcessCloakAndNotify @ 0x006F4A70` sends `Transmit_Radio_ToFirst(0x0D)` when `ObjectClass__Mark` succeeds and `+0x418 != 0`.` (evidence: `0x006F4A70`, `0x006F4A81..0x006F4A91`)
- `[RESOLVED] OQ-0D-002 - Is the send directed or Contacts[0]? -> Contacts[0] only via vtable `+0x274`.` (evidence: `0x006F4A91`; RadioClass slot docs)
- `[RESOLVED] OQ-0D-003 - What gates the sender? -> successful mark/update plus `TechnoClass+0x418 != 0`.` (evidence: `0x006F4A70`; `+0x418` lifecycle report)
- `[RESOLVED] OQ-0D-004 - Who calls the sender helper? -> BuildingClass vtable `+0x124` and `TechnoClass__DoCloak`; also TechnoClass vtable DATA binding.` (evidence: `get_function_xrefs 0x006F4A70`)
- `[RESOLVED] OQ-0D-005 - What does ObjectClass do with 0x0D? -> Calls receiver vtable `+0x124(2)`, returns `1`.` (evidence: `0x005F5320`, `0x005F5370..0x005F537A`)
- `[RESOLVED] OQ-0D-006 - What does BuildingClass do before ObjectClass sees it? -> If `WeaponsFactory=yes`, returns `1`; otherwise falls through to base receivers.` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ-0D-007 - Is the WeaponsFactory swallow active in stock YR? -> Yes; stock `GAWEAP`, `NAWEAP`, `YAWEAP` set `WeaponsFactory=yes`.` (evidence: `rulesmd.ini:11775`, `12565`, `13309`)
- `[RESOLVED] OQ-0D-008 - Does the receiver use sender/payload? -> No for ObjectClass `0x0D`; sender/payload are ignored.` (evidence: `0x005F5320`)
- `[RESOLVED] OQ-0D-009 - Is `0x0D` a BREAK/OVER_AND_OUT alias? -> No; BREAK is `0x03`. `0x0D` is a contacted-object mark/animation refresh notification.` (evidence: sender `0x006F4A70`; receiver `0x005F5320`; BuildingClass case `0x03` separate)
- `[DEFERRED] OQ-0D-010 - Exact visual frame delta from executing BuildingClass `+0x124(2)` vs swallowing it on a war factory?` (category: `needs-runtime-debugger`; reason: static code proves the function and swallow, but exact frame reset/skip needs a captured runtime comparison; next-step-if-pursued: trace GAWEAP produced tank exit with and without the `WeaponsFactory` branch)
- `[DEFERRED] OQ-0D-011 - Are there dynamically computed non-immediate `0x0D` radio sends?` (category: `bounded-cost-too-high`; reason: all immediate `PUSH 0x0D` and radio-slot contexts in scope were scanned; proving all register-computed values absent requires a full binary dataflow pass; next-step-if-pursued: script all vtable `+0x274/+0x278/+0x27C` callers and back-slice message arguments)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Contacted Techno mark/update sends `0x0D` to Contacts[0] only when `+0x418` is set. | `0x006F4A70`, `0x006F4A81..0x006F4A91`; `+0x418` report | missing / unchecked generic radio side-effect | `src/sim/game_entity.rs` contact state; miner and production contact flows | Preserve a sender-side event equivalent: contacted object mark/unmark refresh may notify its first contact with `0x0D` if contact-entered is true | Proposed test: `radio_0d_contacted_mark_refresh_sends_only_to_first_contact` | Do not broadcast `0x0D` to every contact; gamemd uses `+0x274` Contacts[0] only |
| Non-WF building receivers let `0x0D` reach ObjectClass, which invokes building vtable `+0x124(2)` / mark-attached-animation refresh. | `0x005F5320`, `0x007E3FE0 -> 0x0043F180` | missing | building animation/mark refresh surface; `src/app_building_anim.rs` / sim-visible anim overlay state | Add an explicit receiver-side effect or equivalent refresh hook for buildings that should not swallow | A contacted refinery/other dock building receiving `0x0D` refreshes attached anim state without changing radio contact membership | Do not model `0x0D` as `BREAK`; it does not clear Contacts[] |
| `WeaponsFactory=yes` buildings swallow `0x0D` and return `ROGER=1` before ObjectClass sees it. | `0x0043C2D0`; `rulesmd.ini` stock WFs | likely missing | `src/sim/production/production_spawn.rs`, building type flags, animation refresh hook | Ensure stock war factories do not run the generic building `0x0D` mark/anim refresh during produced-unit contact churn | `war_factory_radio_0d_swallow_preserves_production_anim_state` | Do not apply the generic anim-refresh side effect to `GAWEAP`/`NAWEAP`/`YAWEAP` |

## Negative Facts / Do Not Do

- Do not treat radio `0x0D` as `BREAK`/`OVER_AND_OUT`; `0x03` owns contact teardown. Evidence: `RadioClass::Receive_Radio`, `BuildingClass::Receive_Radio` case `3`, and sender `0x006F4A70`.
- Do not send `0x0D` unconditionally on every mark/update; it is gated by `+0x418 != 0`. Evidence: `0x006F4A81..0x006F4A89`.
- Do not broadcast `0x0D` to all contacts; the verified sender calls vtable `+0x274`, which targets Contacts[0] only. Evidence: `0x006F4A91`.
- Do not let stock war factories run the generic ObjectClass `0x0D` receiver effect; `BuildingClass` returns `1` when `Type+0x16BD` is set. Evidence: `0x0043C2D0`; stock INI.
- Do not key the swallow on `Factory=UnitType` or `Bib=yes`; the binary checks `WeaponsFactory=yes` (`BuildingType+0x16BD`). Evidence: BuildingClass case `0x0D`; INI.

## Remaining Uncertainty

- The exact visible frame/animation delta from `FUN_0043F180(mode=2)` versus the war-factory swallow needs runtime capture. Static evidence supports "building mark/attached-anim refresh" but not a screenshot-level frame assertion.
- A full binary-wide dataflow proof against register-computed `0x0D` sends was not attempted; the immediate radio send inventory is complete for this slice.

## Stale Docs / Follow-up Docs

- `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` open question "What sends 0x0D in the live game?" should be replaced with: "`TechnoClass__ProcessCloakAndNotify @ 0x006F4A70` sends `Transmit_Radio_ToFirst(0x0D)` after successful `ObjectClass__Mark` when `TechnoClass+0x418` is nonzero; ObjectClass receives it as `vtable+0x124(2)`, while `BuildingClass` swallows it for `WeaponsFactory=yes`."
- `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` case `0x0D` wording "fires for WeaponsFactory buildings when a manufactured unit disconnects" should be narrowed to: "fires when a contacted peer sends the `0x0D` mark/anim-refresh notification; stock war factories swallow it with `ROGER=1` so produced-unit contact churn does not invoke the generic ObjectClass `+0x124(2)` refresh."
- Any wording calling `0x0D` "OVER_AND_OUT" should be avoided; use "mark/anim refresh notification" unless a future runtime trace gives a better canonical name.

## Sources

- Ghidra decompile: `0x006F4A70` `TechnoClass__ProcessCloakAndNotify`.
- Ghidra assembly context: `0x006F4A81..0x006F4A91`.
- Ghidra xrefs: `get_function_xrefs 0x006F4A70`.
- Ghidra decompile: `0x004D3780` `TechnoClass__DoCloak`.
- Ghidra decompile: `0x0043F180` BuildingClass vtable `+0x124`.
- Ghidra vtable data xref: `get_xrefs_from 0x007E3FE0 -> 0x0043F180`.
- Ghidra decompile/disassembly context: `0x005F5320` `ObjectClass__Receive_Radio`, especially `0x005F5370..0x005F537A`.
- Ghidra decompile: `0x0043C2D0` `BuildingClass__Receive_Radio`.
- Ghidra decompile: `0x006F4AB0` `TechnoClass__Receive_Radio` (`+0x418` set/clear context).
- Ghidra scans: `search_byte_patterns "6A 0D"` and vtable `+0x274/+0x278/+0x27C` call-site context scans.
- Existing reports: `RADIOCLASS_CORE_PRIMITIVES_VERIFIED_GHIDRA_REPORT.md`, `RADIO_VTABLE_BINDING_AND_SLOT_HELPERS_GHIDRA_REPORT.md`, `UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`, `WAR_FACTORY_EXIT_CONTACT_ROW_SKIP_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.

## Status

COMPLETE for the scoped immediate sender/receiver/WeaponsFactory-swallow slice.
