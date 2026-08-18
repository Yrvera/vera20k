# Skirmish Siege Attacker Role Constructor - Ghidra Research Report

**Address(es):** `0x005CAEB0`, `0x005CAE10`, `0x005D8C50`, `0x005D8CB0`, `0x005CA6D0`, `0x005D7DA0`, `0x005D81F0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** constructor-time integer assignment for the Siege attacker role object around `0x005CAEB0`, how that value reaches node `+0x6B` through `MultiplayerTeam` virtual method `0x005D8CB0`, and whether the path is active in standard YR offline Skirmish.  
**Non-Scope:** online/WOL lobby role UI, full `MPModes` override file extraction, gameplay spawn placement, and Rust implementation changes.  
**Confidence:** High for constructor arguments, vtable binding, writer behavior, and standard stock-data liveness.  
**Active in YR:** Conditional. Binary support is present and not TS-gated, but exposed stock `ini/mpmodesmd.ini` has no `[Siege]` entry, so standard offline Skirmish does not expose this role path.

## 0. Working Notes Required Before Investigation

Target question: Does the Siege attacker role constructor assign the role value later written from object `+0x08` into node `+0x6B`, and what exact value is it?
Non-goals: Do not re-cover ordinary Skirmish controls, complete MPModes parsing, online lobby UI, or Rust implementation.
Evidence needed to mark COMPLETE: constructor call-site stack arguments, base constructor write to `+0x08`, attacker vtable writer slot, Siege validator interpretation, and stock MPModes liveness.
Stop conditions: no mutable Ghidra actions; if function boundaries block decompile, use read-only assembly/raw binary bytes and record uncertainty.

## 1. Overview

`0x005CAEB0` constructs the binary object whose string evidence is `MP:AttackerTeam`. The constructor does not assign attacker value `2`; it passes literal `1` into the shared `MultiplayerTeam` constructor, which writes that argument to object `+0x08`.

This is load-bearing because the shared role writer `0x005D8CB0` copies object `+0x08` directly into `DAT_00A8DA78[node_index] + 0x6B`, while Siege Start validation treats node `+0x6B == 2` as the attacker count. In the stock exposed YR offline roster this mismatch is dormant because `MPModesMD.ini` does not list `[Siege]`.

## 2. Class Layout / Key Offsets

| Object / field | Offset | Verified value / behavior | Evidence | Active in YR |
|---|---:|---|---|---|
| `MultiplayerTeam` vtable | `+0x00` | base vtable `0x007EEEDC`; subclass constructors overwrite it | `0x005D8C64`, `0x005CAE48`, `0x005CAEE8` | Conditional |
| `MultiplayerTeam` name/string | `+0x04` | initialized by `FUN_007B6720(first_arg)` | `0x005D8C57..0x005D8C5B` | Conditional |
| `MultiplayerTeam` role integer | `+0x08` | assigned from second stack argument | `0x005D8C60..0x005D8C6A` | Conditional |
| network/player node role field | `node + 0x6B` | receives role integer from object `+0x08` | `0x005D8CCD..0x005D8CD5` | Conditional |
| Siege mode selected object vtable | `+0x00` | vtable `0x007EE6FC`; `+0x14` points to `0x005CA6D0` | raw vtable dwords and `0x005CA658` | Conditional |

## 3. Core Logic

### Base Role Constructor

`0x005D8C50` is the shared `MultiplayerTeam` constructor. Active in YR: Conditional, when a mode/team-role object is built.

1. Reads first stack argument and passes it to the string constructor at object `+0x04`.
2. Stores base vtable `0x007EEEDC` into object `+0x00`.
3. Reads the second stack argument at `0x005D8C60`.
4. Writes it unchanged to object `+0x08` at `0x005D8C6A`.
5. Returns the object.

Evidence: Ghidra force-decompile of `0x005D8C50`; assembly `MOV ECX,dword ptr [ESP + 0xC]` then `MOV dword ptr [ESI + 0x8],ECX`.

### Defender Role Constructor

`0x005CAE10` builds the role object for string `MP:DefenderTeam` (`0x0082FFE4`) and source string `D:\ra2mdpost\MPSiegeTeam.cpp` (`0x0082FFF4`). Active in YR: Conditional, only if Siege role UI construction runs.

The constructor calls `0x00734E60` with line literal `0x6`, copies the resulting string wrapper, then calls `0x005D8C50` with arguments `(name_wrapper, 1)`. After the base constructor returns, it overwrites the vtable with `0x007EE7E4`.

Evidence: assembly `0x005CAE35 PUSH 0x1`, `0x005CAE37 PUSH EAX`, `0x005CAE3A CALL 0x005D8C50`, `0x005CAE48 MOV [ESI],0x7EE7E4`.

### Attacker Role Constructor

`0x005CAEB0` builds the role object for string `MP:AttackerTeam` (`0x00830044`). Active in YR: Conditional, only if Siege role UI construction runs.

The constructor calls `0x00734E60` with line literal `0x16`, copies the resulting string wrapper, then calls `0x005D8C50` with arguments `(name_wrapper, 1)`. After the base constructor returns, it overwrites the vtable with `0x007EE7F4`.

Evidence: assembly `0x005CAED5 PUSH 0x1`, `0x005CAED7 PUSH EAX`, `0x005CAEDA CALL 0x005D8C50`, `0x005CAEE8 MOV [ESI],0x7EE7F4`. Raw PE bytes for this function match the Ghidra assembly; no `PUSH 0x2` occurs in the constructor body.

### Shared Node Writer

`0x005D8CB0` is the writer used by the attacker vtable. Active in YR: Conditional, when the role object's vtable writer slot is invoked.

1. Calls the role object's vtable `+0x04` validity method with the node index.
2. Returns false without writing if that validity method returns zero.
3. Reads object dword `+0x08`.
4. Reads node pointer from `DAT_00A8DA78[index]`.
5. Stores the object `+0x08` value to node `+0x6B`.
6. Returns true.

Evidence: Ghidra decompile of `0x005D8CB0`; assembly `0x005D8CCD MOV EAX,[ESI+0x8]`, `0x005D8CD0 MOV EDX,[ECX+EDI*4]`, `0x005D8CD5 MOV [EDX+0x6B],EAX`.

### Vtable Binding

Raw binary vtable dwords verify the attacker role object uses the shared writer:

| Vtable | Slot 0 | Slot 1 / validity | Slot 2 / writer | Meaning |
|---:|---:|---:|---:|---|
| `0x007EE7E4` | `0x005CAF10` | `0x005CAE70` | `0x005D8CB0` | defender-specific validity, shared writer |
| `0x007EE7F4` | `0x005CAF40` | `0x005D8C90` | `0x005D8CB0` | generic range validity, shared writer |

Evidence: raw `gamemd.exe` PE read at `0x007EE7E4` / `0x007EE7F4`, plus Ghidra xref to `0x005D8CB0` from `0x007EE7EC`.

### Siege Start Validator

`0x005CA6D0` reads node `+0x6B` and interprets values as:

| Node `+0x6B` | Branch result | Evidence | Active in YR |
|---:|---|---|---|
| `0` | allowed neutral/no counter | `0x005CA704 SUB EAX,0`; zero branch to loop continuation | Conditional |
| `1` | defender/besieged; second `1` rejects | `0x005CA709 DEC`, `0x005CA70A JZ 0x005CA712`, duplicate error `MP:OnlyOneBeseiged` | Conditional |
| `2` | attacker; increments attacker count | `0x005CA70C DEC`, `0x005CA70D JNZ illegal`, `0x005CA70F INC EDI` | Conditional |
| other | rejects as illegal team | `0x005CA748` uses `MP:IllegalTeam` | Conditional |

After scanning, no defender rejects as `MP:NoDefender`, and defender present but attacker count `< 1` rejects as `MP:NoAttackers`. Evidence: `0x005CA720`, `0x005CA790`, strings at `0x0082FF1C` and `0x0082FEE8`.

## 4. INI Keys / Stock Data

| File / section | Stock value | Effect | Evidence | Active in YR |
|---|---|---|---|---|
| `ini/mpmodesmd.ini:[Battle]` | present | standard offline Skirmish battle modes | `ini/mpmodesmd.ini` | Yes |
| `ini/mpmodesmd.ini:[ManBattle]` | present | human battle variants | `ini/mpmodesmd.ini` | Conditional |
| `ini/mpmodesmd.ini:[FreeForAll]` | present | FFA mode | `ini/mpmodesmd.ini` | Conditional |
| `ini/mpmodesmd.ini:[Unholy]` | present | Unholy Alliance mode | `ini/mpmodesmd.ini` | Conditional |
| `ini/mpmodesmd.ini:[Cooperative]` | present | Cooperative mode | `ini/mpmodesmd.ini` | Conditional |
| `ini/mpmodesmd.ini:[Siege]` | absent | no standard exposed offline Siege selection | absence in file; binary category registration at `0x005D7DA0` | No for standard exposed roster; conditional for custom data |

The binary registers the `Siege` category with string `0x00830BEC` at `0x005D7DA0`, and the factory at `0x005D81F0` constructs a `0x40`-byte Siege object via `0x005CA630`. That proves binary support exists, but stock offline local data does not expose it.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Mode category registration | `Siege` is registered between `ManBattle` and `Unholy` | `0x005D7DA0 PUSH 0x830BEC`; `0x005D7590` loader path | Yes, binary registration |
| Siege mode object factory | allocates `0x40` bytes and calls `0x005CA630` | `0x005D81F0..0x005D821E` | Conditional |
| Siege mode constructor | delegates to shared game-mode constructor and stores vtable `0x007EE6FC` | `0x005CA630..0x005CA661` | Conditional |
| Siege role list population | nearby role construction creates observer, defender, then attacker objects | `0x005CA9B2..0x005CAAB5` | Conditional; exact UI caller not stock-exposed |
| Role selection writer | copies role object `+0x08` to node `+0x6B` | `0x005D8CB0` | Conditional |
| Start validation | selected Siege mode reads node `+0x6B` before launch packing | `0x005CA6D0`; sibling Start acceptance report | Conditional |

## 6. Current Rust Implementation Status

Scoped search found no Rust model for `MPModesMD.ini` mode objects or Siege role UI. Current shell Start path packs a `SkirmishLaunchSession` with `SkirmishLaunchMode::Battle`.

Evidence: `src/ui/skirmish_shell/state.rs` has `SkirmishShellAction::StartGame`, `launch_session`, and `mode: SkirmishLaunchMode::Battle`; `src/skirmish_launch.rs` defines the launch mode; `src/app.rs` calls `launch_session` on Start.

Rust-facing conclusion: no change is required for standard exposed YR offline Skirmish Siege roles because stock data cannot select Siege. Future custom-mode support must not infer an attacker value of `2` from the `MP:AttackerTeam` name alone.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Required working notes | verified | section 0 | none |
| Attacker constructor `0x005CAEB0` | verified | assembly `0x005CAED5..0x005CAEE8`, raw bytes | none |
| Defender constructor `0x005CAE10` | verified | assembly `0x005CAE35..0x005CAE48` | none |
| Base `MultiplayerTeam` constructor `0x005D8C50` | verified | decompile and assembly `0x005D8C60..0x005D8C6A` | none |
| Attacker vtable writer slot | verified | raw vtable `0x007EE7F4`, slot 2 = `0x005D8CB0` | none |
| Shared node writer | verified | `0x005D8CB0`, `0x005D8CCD..0x005D8CD5` | none |
| Siege validator role interpretation | verified | `0x005CA701..0x005CA790` | none |
| Standard exposed offline liveness | verified | `ini/mpmodesmd.ini` lacks `[Siege]`; binary registration at `0x005D7DA0` | none for stock roster |
| Online/WOL role UI | deferred | user non-scope | separate investigation if needed |
| Custom extracted override data | deferred | user non-scope | archive extraction/content audit |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Which constructor is the attacker role object? -> `0x005CAEB0`; it uses string `MP:AttackerTeam` at `0x00830044` and stores vtable `0x007EE7F4`.` (evidence: `0x005CAEB0..0x005CAEE8`, raw string read)
- `[RESOLVED] OQ-2 - What role integer does the attacker constructor pass to the base `MultiplayerTeam` constructor? -> Literal `1`, not `2`.` (evidence: `0x005CAED5 PUSH 0x1`, `0x005CAEDA CALL 0x005D8C50`)
- `[RESOLVED] OQ-3 - What does the base constructor do with that argument? -> It writes the second argument unchanged into object `+0x08`.` (evidence: `0x005D8C60..0x005D8C6A`)
- `[RESOLVED] OQ-4 - Does the attacker vtable use a different writer that could transform the value to `2`? -> No; attacker vtable slot 2 is `0x005D8CB0`, the shared writer that copies `+0x08` directly.` (evidence: raw vtable `0x007EE7F4`; `0x005D8CCD..0x005D8CD5`)
- `[RESOLVED] OQ-5 - What does Siege Start validation require for attackers? -> It increments attacker count only for node `+0x6B == 2`; `1` is the single defender/besieged role.` (evidence: `0x005CA709..0x005CA70F`, `0x005CA790`)
- `[RESOLVED] OQ-6 - Is this path active in standard exposed YR offline Skirmish? -> No for exposed stock roster; `[Siege]` is absent from `ini/mpmodesmd.ini`, though the binary registers the category.` (evidence: `ini/mpmodesmd.ini`; `0x005D7DA0`)
- `[RESOLVED] OQ-7 - Is the path TS-only gated? -> No TS-only flag gate was found in the bounded constructor/writer/validator chain; liveness is data-driven by mode selection.` (evidence: `0x005D7DA0`, `0x005D81F0`, `0x005CA630`, `0x005CA6D0`)
- `[RESOLVED] OQ-8 - Does ordinary offline Skirmish control packing write node `+0x6B`? -> No; inherited from prior verified role report and outside this constructor slice.` (evidence: `SKIRMISH_MODE_ROLE_UI_NODE_0X6B_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-9 - What online/WOL UI surface invokes these role objects?` (category: `out-of-scope`; reason: user scoped this slot to constructor assignment and standard offline Skirmish liveness; next-step-if-pursued: online lobby role-control dispatch trace)
- `[DEFERRED] OQ-10 - Could hidden archive override INIs expose a stock Siege row outside the plain repo `mpmodesmd.ini`?` (category: `out-of-scope`; reason: this slot was limited to standard exposed local roster and constructor binary evidence; next-step-if-pursued: archive extraction audit of shipped `MPModesMD.ini` sources)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard exposed YR offline Skirmish has no selectable Siege row, so no Siege role UI is needed for stock Battle launch. | `ini/mpmodesmd.ini` lacks `[Siege]`; binary only registers support at `0x005D7DA0` | none for stock Battle path | `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs` | Keep default shell launch as Battle unless MPModes support is added. | Test proposal: `skirmish_shell_stock_modes_do_not_expose_siege_roles` | Do not add visible Siege role controls to the stock shell from binary support alone. |
| `MP:AttackerTeam` constructor stores role integer `1`, and its writer copies that value directly to node `+0x6B`. | `0x005CAED5`, `0x005D8C6A`, `0x005D8CD5`, vtable `0x007EE7F4` | missing only if future custom Siege UI is implemented | future MPModes/role UI model | If supporting custom Siege data, model this as verified binary behavior, including the mismatch with validator value `2`. | Test proposal: `custom_siege_attacker_constructor_role_value_matches_gamemd_literal_one` | Do not "fix" the constructor to `2` because the validator says `2` means attacker; that would diverge from this binary slice. |
| Siege Start validation requires exactly one defender value `1` and at least one attacker value `2`, but stock constructor evidence does not show the attacker role object producing `2`. | validator `0x005CA701..0x005CA790`; constructor `0x005CAEB0` | unchecked because Rust has no Siege mode | future mode-specific validation surface | Any future Siege support must decide explicitly whether it is reproducing stock binary bug/dormancy or a mod-facing corrected mode, and tests must name that policy. | Test proposal: `siege_validation_rejects_attacker_constructor_only_distribution_as_no_attackers` | Do not infer role semantics solely from strings like `MP:AttackerTeam`. |

## 10. Negative Facts / Do Not Do

- Do not claim `MP:AttackerTeam` constructor assigns `2`; the verified constructor pushes `1`.
- Do not claim the shared writer transforms or maps role values; it copies object `+0x08` directly.
- Do not add stock offline Siege UI controls in Rust just because the binary registers a `Siege` category.
- Do not treat Unholy or ordinary Battle/ManBattle as consumers of Siege node `+0x6B` roles.
- Do not overwrite the prior "ordinary offline controls do not write `+0x6B`" finding; this report only closes the constructor gap.

## 11. Remaining Uncertainty

- None for the scoped constructor-time assignment, writer propagation, validator interpretation, or standard exposed offline Skirmish liveness.
- Online/WOL role UI invocation and hidden archive override content remain intentionally out of scope, not blockers for this report's claim.

## Stale Docs / Follow-up Docs

Replace prior wording:

> Exact constructor-time integer assignment for the attacker role object remains deferred.

with:

> The Siege attacker role constructor at `0x005CAEB0` passes literal `1` to `MultiplayerTeam__Constructor`, which writes it to role object `+0x08`; attacker vtable `0x007EE7F4` uses shared writer `0x005D8CB0`, so this object would write `1` to node `+0x6B`, not the validator's attacker value `2`. This path is dormant in standard exposed offline YR because stock `ini/mpmodesmd.ini` has no `[Siege]` section.

## Sources

- Ghidra read-only decompile/force-decompile: `0x005D8C50`, `0x005D8CB0`, `0x005CA630`, `0x005CABD0`.
- Ghidra read-only assembly context: `0x005CAE10`, `0x005CAEB0`, `0x005CA6D0`, `0x005D7DA0`, `0x005D81F0`, `0x005CA9B2..0x005CAAB5`.
- Raw `gamemd.exe` PE reads for vtables `0x007EE7E4`, `0x007EE7F4`, `0x007EEEDC`, `0x007EE6FC` and strings `0x0082FFE4`, `0x00830044`, `0x0082FEE8`, `0x0082FEF8`, `0x0082FF0C`, `0x0082FF1C`.
- Prior reports: `SKIRMISH_MODE_ROLE_UI_NODE_0X6B_GHIDRA_REPORT.md`, `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_RETAIL_VALUES_AUDIT_GHIDRA_REPORT.md`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/mpmodesmd.ini`.
- Rust scan: `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/app.rs`, `src/app_skirmish.rs`.
