# Skirmish Start Session Vtable +0x14 Acceptance - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x005D6310`, `0x005CA6D0`, `0x005CB400`, `0x005C5D40`, `0x005C1D80`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the concrete selected-mode object methods reached by the offline Skirmish Start Game vtable `+0x14` call at `0x006AD2BA..0x006AD34B`, and the verified accept/reject cases they can produce before Start Game handoff packing.  
**Non-Scope:** gameplay spawn placement after shell exit, full mission/session startup after `FUN_006AE2C0` returns, and complete cooperative mission internals beyond the `+0x14` acceptance method.  
**Confidence:** High for vtable resolution and branch conditions; Medium for human-readable role names on node `+0x6B` values except where error-string keys state the role.  
**Active in YR:** Yes / Conditional. The mode object list is built from YR string `MPModesMD.ini` and section names `Battle`, `ManBattle`, `Siege`, `Unholy`, `FreeForAll`, `Cooperative` in `0x005D7CE0`; offline Skirmish category population uses these objects when `g_GameMode == 5` in `0x005D6130`.

## 1. Overview

`DAT_00A8B23C` is the currently selected multiplayer game-mode/category object, not the selected map record. Start Game calls that object's vtable `+0x14` after the dialog-level validations and before any final launch packing.

For the ordinary `Battle` and `ManBattle` modes, the concrete `+0x14` method is a trivial accept. `FreeForAll` and `Cooperative` also accept but can perform setup side effects. The only mode-specific Start rejections found in this slice are `Siege` role validation failures and `Unholy` rejecting while its enable flag is unset.

## 2. Concrete Vtable Resolution

`0x005D7CE0` creates factories for the six standard YR mode section strings and calls the shared loader/factory path. The factory call uses each factory object's vtable `+0x04`, which allocates and constructs a mode object. The resulting object vtables resolve as follows:

| Mode string | Factory evidence | Constructed object vtable | vtable `+0x14` target | Active in YR |
|---|---|---:|---:|---|
| `Battle` | string `0x00830C00`; factory vtable `0x007EEEBC`; factory method `0x005D8170`; constructor `0x005C0DD0` | `0x007EE184` | `0x005D6310` | Yes, standard mode object |
| `ManBattle` | string `0x00830BF4`; factory vtable `0x007EEEB0`; factory method `0x005D81B0`; constructor `0x005C6150` | `0x007EE50C` | `0x005D6310` | Yes, standard mode object |
| `Siege` | string `0x00830BEC`; factory vtable `0x007EEEA4`; factory method `0x005D81F0`; constructor `0x005CA630` | `0x007EE6FC` | `0x005CA6D0` | Conditional, when Siege mode is selected |
| `Unholy` | string `0x00830BE4`; factory vtable `0x007EEE98`; factory method `0x005D8230`; constructor `0x005CB3A0` | `0x007EE814` | `0x005CB400` | Conditional, when Unholy Alliance mode is selected |
| `FreeForAll` | string `0x00830BD8`; factory vtable `0x007EEE8C`; factory method `0x005D8270`; constructor `0x005C5CE0` | `0x007EE424` | `0x005C5D40` | Conditional, when Free For All mode is selected |
| `Cooperative` | string `0x00830BCC`; factory vtable `0x007EEE80`; factory method `0x005D82B0`; constructor `0x005C1470` | `0x007EE27C` | `0x005C1D80` | Conditional, when Cooperative mode is selected |

Evidence: vtable addresses were read directly from constructor stores (`MOV [object], imm32`) and vtable memory. The common Start caller loads `ESI = [0x00A8B23C]`, then calls `CALL dword ptr [EAX + 0x14]` at `0x006AD2D2`.

## 3. Start Caller Contract

The Start branch initializes a small string/result buffer with `FUN_007B66C0` immediately before the vtable call. If the mode method returns nonzero, `0x006ACEE0` continues into selected-map mirrors and final launch packing. Active in YR: Yes. Evidence: `0x006AD2C0..0x006AD2D7`, fallthrough to `0x006AD34B`.

If the mode method returns zero for Start Game, the Start button is re-enabled and the handler returns before launch packing. Active in YR: Yes. Evidence: failure path `0x006AD2D9..0x006AD343`, including `EnableWindow(GetDlgItem(hwnd, 0x617), 1)` and `FUN_007B6760` cleanup.

The mode methods receive the caller's initialized buffer pointer as their single stack argument. Rejection methods write a localized/message key into that buffer by calling `FUN_007B6880`. Active in YR: Yes. Evidence: `Siege +0x14` calls `0x007B6880` at `0x005CA73A`, `0x005CA75E`, `0x005CA782`, `0x005CA7AB`; `FUN_007B6880` frees any prior pointer, allocates `2 * (len + 1)` bytes, and copies the passed wide string.

## 4. Concrete +0x14 Logic

### Battle and ManBattle

`0x005D6310` is the shared `+0x14` method for `Battle` and `ManBattle`. It executes `mov al, 1; ret` and does not read or write the argument buffer. Active in YR: Yes for those modes. Evidence: vtables `0x007EE184 + 0x14` and `0x007EE50C + 0x14` both point to `0x005D6310`.

Result cases:

| Condition | Return | Buffer write | Active in YR |
|---|---:|---|---|
| Any Start call in these modes | accept / `1` | none | Yes |

### FreeForAll

`0x005C5D40` first delegates to `0x005D6310`. If that base method accepted, it iterates `DAT_00A8DA78[0..DAT_00A8DA84)` and rewrites each node's dword at `+0x6B` to the node index when the previous value is not `-1`. It then returns `1`. Active in YR: Conditional, selected `FreeForAll`.

Evidence: `0x005C5D40..0x005C5D88`; node vector globals `DAT_00A8DA78` / `DAT_00A8DA84`; node field write at `0x005C5D71`.

Result cases:

| Condition | Return | Side effect | Active in YR |
|---|---:|---|---|
| Base accept, zero nodes | accept / `1` | none | Conditional |
| Base accept, one or more nodes | accept / `1` | any node with `+0x6B != -1` gets `+0x6B = index` | Conditional |
| Base reject | reject / `0` | none after base failure | The base currently never rejects in this mode family |

### Cooperative

`0x005C1D80` performs one cooperative-specific side effect only when `DAT_00A8DA84 == 2` and `this + 0x40` is non-null: it passes the first two node pointers from `DAT_00A8DA78` to `0x0049B760`. It then delegates to `0x005D6310` and returns that result. Active in YR: Conditional, selected `Cooperative`.

Evidence: node-count check and call at `0x005C1D80..0x005C1DA0`; base delegate at `0x005C1DA5..0x005C1DB2`.

Result cases:

| Condition | Return | Side effect | Active in YR |
|---|---:|---|---|
| exactly two nodes and `this+0x40 != 0` | accept / `1` | calls `0x0049B760(node0, node1)` before accepting | Conditional |
| any other node count or null `this+0x40` | accept / `1` | no cooperative pre-call | Conditional |

### Unholy

`0x005CB400` accepts only if global byte `DAT_00A8B258` is nonzero and the base method accepts. If `DAT_00A8B258 == 0`, it returns `0` without writing a message buffer. Active in YR: Conditional, selected `Unholy`.

Evidence: byte read at `0x005CB400`; false return at `0x005CB41F`; delegate to `0x005D6310` at `0x005CB40E`; mode hook methods at `0x005CB3F0` and `0x005CB430` set `DAT_00A8B258 = 1`.

Result cases:

| Condition | Return | Buffer write | Active in YR |
|---|---:|---|---|
| `DAT_00A8B258 == 0` | reject / `0` | none | Conditional |
| `DAT_00A8B258 != 0` and base accepts | accept / `1` | none | Conditional |

### Siege

`0x005CA6D0` delegates to `0x005D6310`, then validates existing node records in `DAT_00A8DA78[0..DAT_00A8DA84)`. It reads each node dword at offset `+0x6B`. Values are interpreted by branch behavior and error strings:

| `node+0x6B` value | Behavior | Evidence | Active in YR |
|---:|---|---|---|
| `0` | accepted by the scan; no counter changed | `0x005CA701..0x005CA718` | Conditional, Siege selected |
| `1` | marks the single besieged/defender slot; a second `1` rejects | `0x005CA709..0x005CA716`; error key `MP:OnlyOneBeseiged` | Conditional |
| `2` | increments attacker count | `0x005CA70C..0x005CA710`; absence rejects as `MP:NoAttackers` | Conditional |
| any other value | rejects as illegal team | `0x005CA748`; error key `MP:IllegalTeam` | Conditional |

After the scan:

| Condition | Return | Message key | Evidence | Active in YR |
|---|---:|---|---|---|
| base method rejected | reject / `0` | inherited base buffer | `0x005CA6D8..0x005CA6E6` | Base currently does not reject |
| no node had value `1` | reject / `0` | `MP:NoDefender` | `0x005CA720..0x005CA745`; string `0x0082FF1C`; source file string `0x0082FF2C` | Conditional |
| a second node had value `1` | reject / `0` | `MP:OnlyOneBeseiged` | `0x005CA712..0x005CA716`, `0x005CA76C..0x005CA78D`; string `0x0082FEF8` | Conditional |
| any node had value outside `0..2` | reject / `0` | `MP:IllegalTeam` | `0x005CA70C..0x005CA70D`, `0x005CA748..0x005CA769`; string `0x0082FF0C` | Conditional |
| defender exists but attacker count `< 1` | reject / `0` | `MP:NoAttackers` | `0x005CA790..0x005CA7B6`; string `0x0082FEE8` | Conditional |
| exactly one defender and at least one attacker, no illegal values | accept / `1` | none | `0x005CA790` branch to success at `0x005CA7B9` | Conditional |

The error keys are stored near the source file string `D:\ra2mdpost\MPSiege.cpp`, confirming the mode-specific source area. Active in YR: Conditional on selecting Siege; no TS-only gate appears in this method.

## 5. Current Rust Implementation Status

Rust currently exposes `OwnerDrawButton::StartGame0x617` and maps it directly to `SkirmishShellAction::StartGame`, then `launch_settings` builds a simple `SkirmishSettings` from current shell state. There is no current Rust equivalent for selected game-mode objects, MPModesMD category loading, or mode-specific `+0x14` acceptance/rejection.

Evidence: `src/ui/skirmish_shell/state.rs` has `StartGame0x617`, `SkirmishShellAction::StartGame`, and `launch_settings`; `src/app.rs` calls `launch_settings` on Start.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Start caller vtable dispatch | verified | `0x006AD2BA..0x006AD34B` | none for `+0x14` call contract |
| mode object list source | verified | `0x005D7CE0`, strings `0x00830A18`, `0x00830BCC..0x00830C00` | exact contents of retail `MPModesMD.ini` not dumped |
| `Battle` / `ManBattle` vtable `+0x14` | verified | vtables `0x007EE184`, `0x007EE50C`; method `0x005D6310` | none |
| `FreeForAll` vtable `+0x14` | verified | vtable `0x007EE424`; method `0x005C5D40` | downstream meaning of node `+0x6B` consumers out of scope |
| `Cooperative` vtable `+0x14` | verified | vtable `0x007EE27C`; method `0x005C1D80`; helper resolved by `SKIRMISH_COOPERATIVE_PRECALL_0049B760_GHIDRA_REPORT.md` | downstream cooperative campaign/save consumer remains separate |
| `Unholy` vtable `+0x14` | verified | vtable `0x007EE814`; method `0x005CB400` | exact user path that can leave `DAT_00A8B258 == 0` deferred |
| `Siege` vtable `+0x14` | verified | vtable `0x007EE6FC`; method `0x005CA6D0`; `+0x6B` writer resolved by `SKIRMISH_MODE_ROLE_UI_NODE_0X6B_GHIDRA_REPORT.md` | exact attacker constructor role-value assignment remains deferred |
| gameplay spawn placement after shell exit | deferred | user non-scope | separate scenario/spawn investigation |

## 7. Open Questions - Final State

[RESOLVED] OQ-1 - What is the concrete object behind `DAT_00A8B23C` for this call? It is the selected MPModes game-mode/category object, populated into control `0x6EB` and selected into `DAT_00A8B23C`; not the scenario record. Evidence: `0x005D6130`, `0x005E7160`, `0x006AD2BA`.

[RESOLVED] OQ-2 - Which concrete `+0x14` methods exist for standard YR offline Skirmish modes? `Battle`/`ManBattle -> 0x005D6310`; `Siege -> 0x005CA6D0`; `Unholy -> 0x005CB400`; `FreeForAll -> 0x005C5D40`; `Cooperative -> 0x005C1D80`. Evidence: constructor vtable stores and vtable memory listed in section 2.

[RESOLVED] OQ-3 - Does the default Battle-like mode reject Start? No. `0x005D6310` returns `1` unconditionally. Evidence: method body at `0x005D6310`.

[RESOLVED] OQ-4 - Which concrete mode-specific methods can reject Start? `Siege` can reject invalid node `+0x6B` role distributions; `Unholy` can reject when `DAT_00A8B258 == 0`. Evidence: `0x005CA6D0`, `0x005CB400`.

[RESOLVED] OQ-5 - Are these paths active in YR rather than TS-only? Yes/Conditional. The mode loader uses `MPModesMD.ini` and offline Skirmish category list population checks `g_GameMode == 5`; mode-specific methods are active when the corresponding YR mode object is selected. Evidence: `0x005D7CE0`, `0x005D6130`.

[RESOLVED/PARTIAL] OQ-6 - What exact UI control writes node `+0x6B` values before Siege validation? Ordinary offline Skirmish `0x102` controls do not write node `+0x6B`; the direct writer is the multiplayer team/role virtual method at `0x005D8CB0`, which stores the role object's dword at object `+0x08` into `DAT_00A8DA78[index]+0x6B`. Binary Siege support exists, but exposed local `ini/mpmodesmd.ini` has no `[Siege]` entry. Exact attacker constructor role-value assignment remains deferred. Evidence: `SKIRMISH_MODE_ROLE_UI_NODE_0X6B_GHIDRA_REPORT.md`.

[RESOLVED] OQ-7 - What does `0x0049B760` do for Cooperative's two-node pre-call? It copies the first two node/player names into the Cooperative progress/save record at `+0x00` and `+0x1C`, then the `+0x14` path still delegates to base accept `0x005D6310`. It does not reject Start or alter team/alliance/session packing. Evidence: `SKIRMISH_COOPERATIVE_PRECALL_0049B760_GHIDRA_REPORT.md`.

## Sources

- Ghidra decompiled/read: `0x006ACEE0`, `0x005D6130`, `0x005E7160`, `0x005D6310`, `0x007B66C0`, `0x007B6760`, `0x007B6880`.
- Ghidra memory/assembly decoded read-only: factories at `0x005D8170`, `0x005D81B0`, `0x005D81F0`, `0x005D8230`, `0x005D8270`, `0x005D82B0`; constructors at `0x005C0DD0`, `0x005C6150`, `0x005CA630`, `0x005CB3A0`, `0x005C5CE0`, `0x005C1470`; vtables at `0x007EE184`, `0x007EE50C`, `0x007EE6FC`, `0x007EE814`, `0x007EE424`, `0x007EE27C`.
- String evidence: `0x00830A18` `MPModesMD.ini`; `0x00830BCC` `Cooperative`; `0x00830BD8` `FreeForAll`; `0x00830BE4` `Unholy`; `0x00830BEC` `Siege`; `0x00830BF4` `ManBattle`; `0x00830C00` `Battle`; Siege messages at `0x0082FEE8`, `0x0082FEF8`, `0x0082FF0C`, `0x0082FF1C`, file string `0x0082FF2C`.
- Prior context report: `docs/research/skirmish-ui/SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`.
- Rust scan: `src/ui/skirmish_shell/state.rs`, `src/app.rs`.
