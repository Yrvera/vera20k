# BuildingTypeClass Flags +0x16B3/+0x16BB/+0x16BC — Ghidra Research Report

**Address(es):** `0x0045FE50` (BuildingTypeClass INI reader body), `0x0045DD90` (constructor), `0x00447B20` (BuildingClass::GetDockCoord), `0x0043C2D0` (BuildingClass::Receive_Radio), `0x00672660` (RulesClass::ReadBuildingTypes)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** field identity and stock YR values for BuildingTypeClass `+0x16B3`, `+0x16BB`, `+0x16BC`, plus immediate docking implications for the refinery-pad contradiction.
**Non-Scope:** full dock/unload state machine, stock refinery art/passability, and non-stock mod behavior beyond what these three flags imply.
**Confidence:** High
**Active in YR:** Yes

## 0. Working Notes

- Target question: Are `+0x16B3`, `+0x16BB`, and `+0x16BC` DockUnload/Refinery/Weeder, and do stock GAREFN/NAREFN set `+0x16BC`?
- Non-goals: Do not re-investigate full miner docking, `RemoveOccupy`, `QueueingCell`, or all `GetDockCoord` branches.
- Evidence needed to mark COMPLETE: binary parser/read mapping for all three offsets, constructor defaults, stock YR INI values for GAREFN/NAREFN, and at least one active use site proving the offsets are consumed in live YR logic.
- Stop conditions: all scoped offset/key/value questions resolved or deferred; report saved; one `.swarm-claims.md` completion row appended; no Rust/INI/in-repo docs changed.

## 1. Overview

The coord-cell doc's refinery-pad claim is stale. Binary parsing maps `DockUnload` to BuildingTypeClass `+0x16B3`, `Refinery` to `+0x16BB`, and `Weeder` to `+0x16BC`. Stock YR `GAREFN` and `NAREFN` set `DockUnload=yes` and `Refinery=yes`; neither sets `Weeder=yes`, so the `+0x16BC` branch in `BuildingClass::GetDockCoord` is not the standard stock refinery miner-deposit branch.

## 2. Class Layout / Key Offsets

| Offset | Type | INI key | Constructor default | Active in YR | Evidence |
|---|---:|---|---:|---|---|
| `+0x16B3` | bool byte | `DockUnload` | `0` | Yes | parser `0x0045FE50`, string xref `0x004609DE`, constructor `0x0045DD90`, radio use `0x0043C2D0` |
| `+0x16BB` | bool byte | `Refinery` | `0` | Yes | parser `0x0045FE50`, string xref `0x00460A5C`, constructor `0x0045DD90`, mission-deploy use `0x0073D630` |
| `+0x16BC` | bool byte | `Weeder` | `0` | Conditional | parser `0x0045FE50`, string xref `0x004604BA`, constructor `0x0045DD90`, use sites `0x00447B20`, `0x0043C2D0`, `0x00739EC0` |

## 3. Core Logic

### Parser mapping

`BuildingTypeClass_ReadINI_Water` at `0x0045FE50` contains the BuildingType INI reader body. In that body, the three scoped keys read and write these exact fields:

- Active in YR: Yes. `CCINIClass__ReadBool(..., "Weeder", old byte at +0x16BC)` writes the returned bool back to `+0x16BC`. Evidence: decompile `0x0045FE50`; `search_strings("Weeder") -> 0x0081AC50`; byte-reference search for `50 AC 81 00 -> 0x004604BA`.
- Active in YR: Yes. `CCINIClass__ReadBool(..., "DockUnload", old byte at +0x16B3)` writes the returned bool back to `+0x16B3`. Evidence: decompile `0x0045FE50`; `search_strings("DockUnload") -> 0x0081AA94`; byte-reference search for `94 AA 81 00 -> 0x004609DE`.
- Active in YR: Yes. `CCINIClass__ReadBool(..., "Refinery", old byte at +0x16BB)` writes the returned bool back to `+0x16BB`. Evidence: decompile `0x0045FE50`; `search_strings("Refinery") -> 0x0081AA5C`; byte-reference search for `5C AA 81 00 -> 0x00460A5C`.

The constructor at `0x0045DD90` initializes all three bytes to zero before INI parsing. `ReadBool` uses the existing byte as default, so absent keys remain false.

### Reader activity

`RulesClass__ReadBuildingTypes` at `0x00672660` reads the live `[BuildingTypes]` list and calls `BuildingTypeClass__FindOrAllocate` (`0x004653C0`) for each entry. The BuildingType vtable contains `0x0045FE50` at `0x007E45D4`, establishing it as the BuildingType INI reader slot used by the type-load pipeline. Active in YR: Yes for standard building type loading.

### Use-site implications

`BuildingClass::GetDockCoord` at `0x00447B20` checks `Type+0x16BC` first. If true, it returns building occupied-cell `+(2,1)` converted to centered leptons with the building Z. If false and `Type+0x16BB` is true, it returns the requester's `GetCoords()` shifted east by `0x80` leptons. Active in YR: Conditional; the first branch is active only for `Weeder=yes` buildings, not for stock GAREFN/NAREFN.

`BuildingClass::Receive_Radio` at `0x0043C2D0` uses `+0x16B3` and `+0x16BC` together for some `CAN_DOCK` acceptance plumbing, then constructs the accepted cell as building occupied-cell `+(3,1)` before sending radio `0x12`. Assembly context around `0x0043CA80` confirms `ADD DX,0x3` and `INC AX`, then `MapClass__Get_CellClass` and radio `0x12`. Active in YR: Yes for stock DockUnload refineries because `+0x16B3` is true.

`UnitClass::Mission_Deploy_Building` at `0x0073D630` checks `BuildingType+0x16BB` for refinery animation/state-4 behavior. Active in YR: Yes for stock GAREFN/NAREFN because `Refinery=yes` sets `+0x16BB`.

## 4. INI Keys

| Section | Key | Stock YR value | Field effect | Active in YR |
|---|---|---:|---|---|
| `[GAREFN]` | `DockUnload` | `yes` | sets `+0x16B3 = 1` | Yes |
| `[GAREFN]` | `Refinery` | `yes` | sets `+0x16BB = 1` | Yes |
| `[GAREFN]` | `Weeder` | absent | default `+0x16BC = 0` | No for stock GAREFN |
| `[NAREFN]` | `DockUnload` | `yes` | sets `+0x16B3 = 1` | Yes |
| `[NAREFN]` | `Refinery` | `yes` | sets `+0x16BB = 1` | Yes |
| `[NAREFN]` | `Weeder` | absent | default `+0x16BC = 0` | No for stock NAREFN |

Evidence: `ini/rulesmd.ini:11722..11730`, `ini/rulesmd.ini:12515..12523`; base RA2 fallback agrees at `ini/rules.ini:8554..8560`, `ini/rules.ini:8597..8603`. No live `Weeder=yes` or `Weeder=no` lines were found in `rulesmd.ini` or `rules.ini`.

`[YAREFN]` in `rulesmd.ini` does not set `DockUnload=yes`, `Refinery=yes`, or `Weeder=yes`; it is a deployed Slave Miner building with different resource mechanics. This report does not generalize stock harvester docking behavior to YAREFN.

## 5. Integration Points

- `0x00672660` reads `[BuildingTypes]` and allocates BuildingType objects from the active rules set. Active in YR: Yes.
- `0x0045FE50` parses BuildingType rules/art keys and writes the scoped flag bytes. Active in YR: Yes.
- `0x00447B20` consumes `+0x16BC` and `+0x16BB` in `GetDockCoord`. Active in YR: Conditional; `+0x16BC` branch needs `Weeder=yes`, while `+0x16BB` branch is live for stock refineries.
- `0x0043C2D0` consumes `+0x16B3` and `+0x16BC` in radio docking. Active in YR: Yes for stock `DockUnload=yes` refineries through `+0x16B3`.
- `0x0073D630` consumes `+0x16BB` in the unload/animation state machine. Active in YR: Yes for stock `Refinery=yes` refineries.

## 6. Current Rust Implementation Status

Rust currently parses `Refinery=yes` into `ObjectType::refinery` and uses that through `RuleSet::is_refinery_type`. I did not find parsed `DockUnload` or `Weeder` BuildingType fields in the focused scan.

The suspicious current drift is in `src/sim/miner/miner_dock_sequence.rs::refinery_pad_cell`, whose no-`DockingOffset` fallback now returns `NW+(2,1)` and whose comments describe that as the retail refinery offset. This target's evidence says that is the `Weeder=yes` `GetDockCoord` branch, not the standard stock refinery deposit anchor. `src/sim/miner/miner_tests.rs` currently expects `(12,11)` for `refinery_pad_cell(10,10,4,3,None)`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `+0x16B3` field identity | verified | parser `0x0045FE50`, string xref `0x004609DE`, radio use `0x0043C2D0` | none |
| `+0x16BB` field identity | verified | parser `0x0045FE50`, string xref `0x00460A5C`, mission-deploy use `0x0073D630` | none |
| `+0x16BC` field identity | verified | parser `0x0045FE50`, string xref `0x004604BA`, GetDockCoord use `0x00447B20` | none |
| Constructor defaults | verified | constructor `0x0045DD90` | none |
| Stock GAREFN/NAREFN scoped values | verified | `rulesmd.ini` and base `rules.ini` lines above | none |
| Stock YAREFN scoped values | verified only for absence of these keys | `rulesmd.ini:13234..13291` | no full Slave Miner mechanics claim |
| Full stock refinery pad/passability | deferred | out of this slot | handled by slot 5 |

## 8. Open Questions — Final State

- `[RESOLVED] OQ1 — Which INI key writes +0x16B3? -> DockUnload.` (evidence: `0x0045FE50`, string `0x0081AA94`, xref `0x004609DE`)
- `[RESOLVED] OQ2 — Which INI key writes +0x16BB? -> Refinery.` (evidence: `0x0045FE50`, string `0x0081AA5C`, xref `0x00460A5C`)
- `[RESOLVED] OQ3 — Which INI key writes +0x16BC? -> Weeder.` (evidence: `0x0045FE50`, string `0x0081AC50`, xref `0x004604BA`)
- `[RESOLVED] OQ4 — Are the three fields default false? -> yes, constructor zeroes them before ReadBool defaults apply.` (evidence: `0x0045DD90`)
- `[RESOLVED] OQ5 — Do stock GAREFN/NAREFN set +0x16BC? -> no, no Weeder key; default remains false.` (evidence: `rulesmd.ini:11722..11730`, `rulesmd.ini:12515..12523`, parser/default above)
- `[RESOLVED] OQ6 — Is +0x16BC the normal stock refinery pad flag? -> no, it is Weeder.` (evidence: parser `0x0045FE50`; stock INI absence)
- `[RESOLVED] OQ7 — Does stock refinery still reach radio docking via these flags? -> yes, through +0x16B3 DockUnload in Receive_Radio.` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ8 — Is +0x16BB consumed by stock refinery unload behavior? -> yes, Mission_Deploy_Building checks it.` (evidence: `0x0073D630`)
- `[DEFERRED] OQ9 — Which stock/mod buildings ever set Weeder?` (category: out-of-scope; reason: no stock `Weeder=` lines found, but exhaustive mod/map scan was not requested; next-step-if-pursued: scan all retail maps and mod inputs)
- `[DEFERRED] OQ10 — Full stock refinery passable pad relation to RemoveOccupy.` (category: out-of-scope; reason: assigned to swarm slot 5; next-step-if-pursued: use slot-5 report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `+0x16BC` is `Weeder`, not stock refinery. | parser `0x0045FE50`; string `0x0081AC50`; xref `0x004604BA`; stock INI absence | mismatch in comments/tests if `NW+(2,1)` is treated as stock refinery | `src/sim/miner/miner_dock_sequence.rs::refinery_pad_cell`, related docs | Do not use `+0x16BC` branch as stock GAREFN/NAREFN deposit anchor | GAREFN at `(10,10)` with no `Weeder` must not assert `GetDockCoord`/pad `NW+(2,1)` as its normal accepted dock cell | Do not implement "Refinery means Weeder" | `stock_garefn_refinery_pad_does_not_use_weeder_getdockcoord_branch` |
| Stock GAREFN/NAREFN set `DockUnload=yes` and `Refinery=yes`. | parser `+0x16B3/+0x16BB`; `rulesmd.ini:11726..11727`, `12519..12520` | Rust parses `Refinery`, DockUnload unchecked/not represented in focused scan | `src/rules/object_type.rs`, `src/rules/ruleset.rs`, miner dock admission | If exact DockUnload gating is modeled, parse/use `DockUnload` separately from `Refinery`; stock miner admission must be gated by DockUnload where gamemd uses `+0x16B3` | A fake building with `Refinery=yes` but no `DockUnload=yes` should not automatically inherit stock DockUnload radio behavior | Do not collapse DockUnload and Refinery into one bool for active radio semantics | `refinery_yes_without_dockunload_does_not_accept_stock_harvester_radio` |
| `Receive_Radio(0x0E)` stock DockUnload acceptance constructs `NW+(3,1)`. | `0x0043C2D0`; assembly context `0x0043CA80..0x0043CAB8` (`+3`, `+1`, map-cell, radio `0x12`) | current Rust has `refinery_can_dock_queue_cell(rx,ry) -> (rx+3,ry+1)`, but `refinery_pad_cell` now separately returns `+2,+1` | `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs` | Keep accepted CAN_DOCK cell distinct from Weeder GetDockCoord; stock accepted cell should remain `NW+(3,1)` | GAREFN at `(10,10)` admits CMIN/HARV to `(13,11)` regardless of absent `DockingOffset0` | Do not "fix" every miner deposit to `NW+(2,1)` | `stock_refinery_candock_accepted_cell_is_nw_plus_3_1_not_weeder_plus_2_1` |

### Negative Facts / Do Not Do

- Do not label `BuildingTypeClass+0x16BC` as refinery pad. Evidence: binary reader maps `"Weeder"` to `+0x16BC` at `0x0045FE50`.
- Do not infer stock GAREFN/NAREFN take the `GetDockCoord` first branch. Evidence: stock `rulesmd.ini` has no `Weeder=` key, constructor default is false.
- Do not use `NW+(2,1)` as "every miner deposit" for standard DockUnload refineries. Evidence: radio accepted cell is `NW+(3,1)` at `0x0043CA80..0x0043CAB8`.
- Do not collapse `DockUnload` and `Refinery` as the same field. Evidence: parser writes `"DockUnload"` to `+0x16B3` and `"Refinery"` to `+0x16BB`.
- Do not treat `YAREFN` as evidence for standard HARV/CMIN docking. Evidence: stock `[YAREFN]` lacks `DockUnload=yes` and `Refinery=yes`; it is a Slave Miner deployed form.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/coord-cell-conversions/fn-building-getdockcoord.md`: replace "Only branch 1 is active for refineries in standard YR" with "Branch 1 is gated by `BuildingTypeClass+0x16BC`, which the BuildingType reader maps to `Weeder`; stock GAREFN/NAREFN do not set `Weeder`, so this is not the standard stock refinery miner-deposit branch."
- `C:/Users/enok/Documents/ra2-rust-game-docs/coord-cell-conversions/_parity.md`: replace row 35 wording that says refinery branch fixed / GAREFN-NAREFN pad `(12,11)` with "UNCHANGED/STALE: `GetDockCoord` `+0x16BC` branch is Weeder, not stock refinery; standard DockUnload refinery CAN_DOCK acceptance remains NW+(3,1), pending parent reconciliation."
- `C:/Users/enok/Documents/ra2-rust-game-docs/coord-cell-conversions/_system.md`: replace "Refinery dock pad — NW+3 -> NW+2 (every miner deposit)" with "Do not apply NW+2 to stock refinery deposits; verify/undo stale Weeder-vs-refinery reclassification and keep stock DockUnload accepted cell NW+(3,1) unless other slots contradict."

## Sources

- Ghidra decompile: `0x0045FE50`, `0x0045DD90`, `0x00447B20`, `0x0043C2D0`, `0x00672660`, `0x0073D630`, `0x00739EC0`
- Ghidra string searches: `"DockUnload" -> 0x0081AA94`, `"Refinery" -> 0x0081AA5C`, `"Weeder" -> 0x0081AC50`
- Ghidra byte-reference searches: `94 AA 81 00 -> 0x004609DE`, `5C AA 81 00 -> 0x00460A5C`, `50 AC 81 00 -> 0x004604BA`
- Ghidra vtable memory: `0x007E45B0..0x007E45FF`, entry `0x007E45D4 = 0x0045FE50`
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`
- Rust scan: `src/rules/object_type.rs`, `src/rules/ruleset.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`

