# Skirmish Checkbox Owner-Draw Variant Writers - Ghidra Research Report

**Address(es):** `OwnerDraw_Checkbox_006163A0`, `FUN_006AE6E0`, `FUN_0060F9A0`, helper thunks `0x00603DB0` / `0x00603DD0`, non-0x102 caller `FUN_00658330`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Whether standard offline Skirmish dialog `0x102` checkbox controls `0x54E`, `0x693`, `0x696`, `0x69A`, and `0x69D` receive owner-draw variant messages `0x4E5` / `0x4E6`, or equivalent writes to owner-draw record bytes `+0xD9` / `+0xDA`.
**Non-Scope:** Trackbars, in-game Options dialog, online/host/guest lobbies, non-checkbox shell controls, full diplomacy/radar shell behavior, and retail screenshot capture.
**Confidence:** High for standard offline Skirmish `0x102`: decompile plus assembly confirms zero-initialized variant bytes, no `0x4E5/0x4E6` standard init sends, and the only binary-wide immediate sends found do not target these controls.
**Active in YR:** Yes for the standard Skirmish checkbox default path; variant writer helpers are active in YR conditionally outside this `0x102` slice.

## 0. Working Notes

Target question: Do standard offline Skirmish dialog `0x102` checkboxes receive `0x4E5`/`0x4E6` or equivalent writes to checkbox art variant bytes `+0xD9/+0xDA`?
Non-goals: Do not survey all shell controls, non-Skirmish lobbies, trackbars, or gameplay consumers.
Evidence needed to mark COMPLETE: checkbox callback writer proof; setup/default proof for owner-draw record bytes; standard `FUN_006AE6E0` control-message proof; binary-wide immediate `0x4E5/0x4E6` send-site check and liveness classification for any hits.
Stop conditions: stop after all `0x4E5/0x4E6` writer sites found in this slice are classified and every standard `0x102` target checkbox has a default/value answer.

## 1. Overview

Standard offline Skirmish checkboxes use the ordinary owner-draw checkbox callback, but they stay on the default art family. The per-control variant bytes `+0xD9` and `+0xDA` are zero when the owner-draw record is created, and `FUN_006AE6E0` initializes the five standard Skirmish option checkboxes only with `BM_SETCHECK (0xF1)`.

The callback supports `0x4E5` and `0x4E6`, and those messages are live in the binary for other shell checkbox rows. They are not sent to the standard offline Skirmish `0x102` option controls, so the standard player-visible path uses only `cue_i.pcx` for unchecked and `cce_i.pcx` for checked.

## 2. Key Offsets / Messages

| Field / message | Behavior | Evidence | Active in YR |
|---|---|---|---|
| owner-draw record `+0xD9` | Variant-family byte. Zero selects formatted default `c%ce_i.pcx`; nonzero enters alternate branch. | `OwnerDraw_Checkbox_006163A0`; assembly write `0x00616840` | Helper active; standard `0x102` remains zero |
| owner-draw record `+0xDA` | Left/right alternate byte. Read by paint and returned by `0x4E7`; written by `0x4E6`. | paint reads around `0x0061650x`; assembly write `0x006168B3`; `0x4E7` decompile | Helper active; standard `0x102` remains zero |
| `0x4E5` | Writes `+0xD9 = (lParam != 0)` and invalidates if changed. | decompile `0x006163A0`; assembly `0x00616833..0x00616854` | Conditional |
| `0x4E6` | Writes `+0xDA = (lParam != 0)` and invalidates if changed. | decompile `0x006163A0`; assembly `0x006168A6..0x006168C7` | Conditional |
| `0x4E7` | Returns `+0xDA`; it does not expose `+0xD9`. | decompile `0x006163A0` | Conditional |
| `0xF1` | Writes checked state at `+0xE8`; does not touch `+0xD9/+0xDA`. | decompile `0x006163A0`; standard init assembly `0x006AEDF3..0x006AEE1F` | Yes |

## 3. Core Logic

### 3.1 Owner-Draw Record Defaults

`FUN_0060F9A0` creates the shared owner-draw record for subclassed shell controls. On the new-record path, assembly zeroes a local `0x80` dword template with `REP STOSD`, constructs a `0x208` byte record, copies the zeroed template, then sends `0x497`.

Evidence:

- `FUN_0060F9A0` decompile: local `auStack_a00[0x80]` is zeroed before `operator_new(0x208)` and `FUN_00623610`.
- Assembly `0x006100F2..0x00610102`: `MOV ECX,0x80`, `XOR EAX,EAX`, `LEA EDI,[ESP+0xC0]`, `STOSD.REP`.
- Assembly `0x0061013F..0x00610190`: allocates `0x208`, constructs record, then copies the zeroed template.
- Assembly `0x0061032B..0x00610339`: sends `0x497` after record setup.

Active in YR: Yes. This setup is reached by the standard Skirmish dialog through `FUN_006AE3F0 -> FUN_00622B50 -> FUN_0060F9A0`.

Result: standard checkbox records begin with `+0xD9 = 0` and `+0xDA = 0` unless a later writer message changes them.

### 3.2 Callback Writer Semantics

`OwnerDraw_Checkbox_006163A0` is the only verified checkbox owner-proc writer for the two variant bytes.

`0x4E5` path:

- Reads old `+0xDA` into `CL` in the decompiler's switch layout but writes `+0xD9`.
- Converts `lParam`/decompiler `param_4` through `param_4 != 0`.
- Writes `byte ptr [record + 0xD9]`.
- Invalidates only when the comparison says the state changed.

Assembly evidence: `0x00616833..0x00616854` includes `TEST EBX,EBX`, `SETNZ AL`, `MOV byte ptr [EBP + 0xD9],AL`, then `InvalidateRect` through `[0x007E149C]` if changed. Decompile evidence: `case 0x4e5: *(bool *)((int)piVar10 + 0xd9) = param_4 != 0;`.

`0x4E6` path:

- Reads old `+0xDA`.
- Converts `lParam` through `param_4 != 0`.
- Writes `byte ptr [record + 0xDA]`.
- Invalidates only when changed.

Assembly evidence: `0x006168A6..0x006168C7` includes `MOV CL,byte ptr [EBP + 0xDA]`, `SETNZ AL`, `MOV byte ptr [EBP + 0xDA],AL`, then jumps to the same invalidation call. Decompile evidence: `case 0x4e6: ... *(bool *)((int)piVar10 + 0xda) = param_4 != 0;`.

Active in YR: Conditional. The helper code is live if callers send these messages to a checkbox owner-proc, but standard offline Skirmish `0x102` does not.

### 3.3 Standard Skirmish Init Does Not Send Variant Messages

`FUN_006AE6E0` initializes the target Skirmish checkboxes in this exact order:

| Control | Init message | Variant message sent? | Default variant bytes after init | Evidence | Active in YR |
|---:|---|---|---|---|---|
| `0x54E` Short Game | `SendMessageA(hwnd, 0xF1, checked, 0)` | No | `+0xD9=0`, `+0xDA=0` | decompile `FUN_006AE6E0`; assembly `0x006AEDC9..0x006AEDFC` | Yes |
| `0x69A` Super Weapons | `0xF1` | No | `0/0` | decompile `FUN_006AE6E0`; assembly `0x006AEDFE..0x006AEE1F` | Yes |
| `0x69D` Build Off Ally | `0xF1` | No | `0/0` | decompile `FUN_006AE6E0` | Yes |
| `0x693` MCV Repacks | `0xF1` | No | `0/0` | decompile `FUN_006AE6E0` | Yes |
| `0x696` Crates Appear | `0xF1` | No | `0/0` | decompile `FUN_006AE6E0` | Yes |

The assembly around the first two controls is representative and load-bearing: after copying mirrors from `DAT_00A8B3D8..3DC`, the function calls `GetDlgItem` for `0x54E`, pushes `0`, pushes normalized checked state, pushes `0xF1`, and calls `SendMessageA`; it repeats for `0x69A`. No `0x4E5`/`0x4E6` appears in this block. The decompile shows the remaining three controls follow the same `GetDlgItem -> SendMessageA(...,0xF1,checked,0)` pattern.

Active in YR: Yes. `FUN_006AE6E0` is reached from `FUN_006AE3F0` on custom init message `0x497` for standard offline Skirmish dialog `0x102`.

### 3.4 Binary-Wide Immediate `0x4E5/0x4E6` Send-Site Check

A binary scan of retail `gamemd.exe` for direct immediate pushes found:

| Pattern | Hits | Classification |
|---|---:|---|
| `PUSH 0x4E5` | 2 | `0x00603DB9` helper thunk; `0x0052EC81` unrelated non-0x102 path |
| `PUSH 0x4E6` | 1 | `0x00603DD9` helper thunk |
| common `CMP ...,0x4E5/0x4E6` immediate forms | 0 | no additional simple switch/cmp writer sites found |

Helper `0x00603DB0`:

```text
AND EDX,0xFF
PUSH EDX
PUSH 0
PUSH 0x4E5
PUSH ECX
CALL SendMessageA
RET
```

Helper `0x00603DD0` is the same shape with message `0x4E6`. Evidence: assembly context `0x00603DB9` and `0x00603DD9`.

Verified helper callers:

- `0x00657EFF` calls `0x00603DB0`, then sends `0xF1` to the same `EBP` HWND and calls `0x00603DD0`. Assembly context uses globals/fields around house/player state, not Skirmish dialog `0x102` option IDs.
- `FUN_00658330` calls `0x00603DD0` for checkbox controls selected from `DAT_0083923C`. That table is `[-1, 0x6CF, 0x6D0, 0x6D1, 0x6D2, 0x6D3, 0x6D4, 0x6D5]`, not any of `0x54E/0x693/0x696/0x69A/0x69D`. Evidence: decompile `FUN_00658330`, xrefs at `0x00658520`, table read via local PE byte read.
- `0x0052EC81` is in an unrelated shell/message path using controls `0x50F` and `0x670`, then sending `0x4B2` text to the `0x670` control. It is not the standard Skirmish checkbox init block and does not name any target checkbox IDs. Evidence: assembly `0x0052EC5F..0x0052EC9F`.

Active in YR: Conditional outside this report's standard Skirmish slice. These calls prove variant helpers exist, not that the standard `0x102` option checkboxes use them.

## 4. Asset Selection Consequence

With `+0xD9 = 0`, paint uses the default formatted path:

| Checked state | PCX selected | Evidence | Active in standard `0x102` |
|---|---|---|---|
| `+0xE8 != 1` | `cue_i.pcx` via format `c%ce_i.pcx` with `%c='u'` | decompile `0x006163A0`; string `0x00835968` / `0x00835974` | Yes |
| `+0xE8 == 1` | `cce_i.pcx` via format `c%ce_i.pcx` with `%c='c'` | decompile `0x006163A0`; string `0x00835968` / `0x00835998` | Yes |

The alternate names remain helper-supported but unused by standard `0x102`:

| Variant condition | PCX selected | Standard `0x102` status |
|---|---|---|
| `+0xD9 != 0`, unchecked, `+0xDA == 0` | `cue_i.pcx` | Not reached |
| `+0xD9 != 0`, unchecked, `+0xDA != 0` | `cce_ir.pcx` | Not reached |
| `+0xD9 != 0`, checked, `+0xDA == 0` | `cce_il.pcx` | Not reached |
| `+0xD9 != 0`, checked, `+0xDA != 0` | `cce_i.pcx` | Not reached |

## 5. INI Keys

None. Checkbox variant bytes are shell owner-draw message state, not INI state. The checked values themselves still come from the already documented `[Skirmish]` / `[MultiplayerDialogSettings]` option chain, but that chain sends only `0xF1` in this slice.

## 6. Current Rust Implementation Status

Current Rust does not yet render the standard Skirmish owner-draw checkboxes as PCX-backed widgets. The future implementation should render only the normal default pair for the standard `0x102` options:

- `src/ui/skirmish_shell/state.rs` carries partial shell settings but no complete five-checkbox visual state.
- `src/ui/skirmish_shell/layout.rs` has trackbar geometry from prior work but not a full checkbox layout/control collection.
- `src/app_skirmish_shell_render.rs` has no `cue_i.pcx` / `cce_i.pcx` checkbox draw role.

No Rust implementation should add standard `0x102` support for `cce_il.pcx` or `cce_ir.pcx` unless a later non-standard shell mode explicitly sends `0x4E5/0x4E6`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target question / non-goals / evidence / stop notes | verified | Section 0 | none |
| Checkbox callback `0x4E5` writer | verified | decompile `OwnerDraw_Checkbox_006163A0`; assembly `0x00616833..0x00616854` | none |
| Checkbox callback `0x4E6` writer | verified | decompile `OwnerDraw_Checkbox_006163A0`; assembly `0x006168A6..0x006168C7` | none |
| Checkbox callback `0x4E7` reader | verified | decompile `OwnerDraw_Checkbox_006163A0` | none |
| Owner-draw record zero defaults | verified | `FUN_0060F9A0`; assembly `0x006100F2..0x00610190` | none |
| Standard `FUN_006AE6E0` checkbox init | verified | decompile `FUN_006AE6E0`; assembly `0x006AEDA0..0x006AEE1F` | none |
| Binary-wide direct immediate `0x4E5/0x4E6` send scan | verified | local retail `gamemd.exe` byte scan; Ghidra assembly at `0x00603DB9`, `0x00603DD9`, `0x0052EC81` | exotic non-immediate construction remains theoretically possible but not evidenced in standard Skirmish path |
| Helper `0x00603DB0/0x00603DD0` callers | verified for target exclusion | xrefs `0x00657EFF`, `0x00657F45`, `0x00658520`; `FUN_00658330` table IDs | full non-Skirmish shell semantics out of scope |
| Standard five checkbox IDs | verified-by-prior and re-used | prior checkbox control mapping reports; `FUN_006AE6E0` target IDs | none |
| Current Rust state/render delta | verified enough for handoff | codegraph/source status from prior reports plus source scan context | implementation out of scope |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which bytes drive checkbox art variants? -> `+0xD9` selects default vs alternate branch; `+0xDA` selects alternate side/right behavior and is returned by `0x4E7`.` (evidence: `OwnerDraw_Checkbox_006163A0`; assembly `0x00616833..0x006168C7`)
- `[RESOLVED] OQ-02 - What writes `+0xD9`? -> Message `0x4E5` writes `+0xD9 = (lParam != 0)` and invalidates if changed.` (evidence: `0x00616833..0x00616854`)
- `[RESOLVED] OQ-03 - What writes `+0xDA`? -> Message `0x4E6` writes `+0xDA = (lParam != 0)` and invalidates if changed.` (evidence: `0x006168A6..0x006168C7`)
- `[RESOLVED] OQ-04 - What are record defaults before init messages? -> New owner-draw records copy a zeroed `0x80` dword template, so both bytes start at zero.` (evidence: `FUN_0060F9A0`; assembly `0x006100F2..0x00610190`)
- `[RESOLVED] OQ-05 - Does `FUN_006AE6E0` send `0x4E5`/`0x4E6` to `0x54E`? -> No; it sends only `0xF1` after `GetDlgItem(0x54E)`.` (evidence: `0x006AEDC9..0x006AEDFC`)
- `[RESOLVED] OQ-06 - Does `FUN_006AE6E0` send `0x4E5`/`0x4E6` to `0x69A`? -> No; it sends only `0xF1`.` (evidence: `0x006AEDFE..0x006AEE1F`)
- `[RESOLVED] OQ-07 - Does `FUN_006AE6E0` send `0x4E5`/`0x4E6` to `0x69D`, `0x693`, or `0x696`? -> No; decompile shows only `SendMessageA(...,0xF1,checked,0)` for each.` (evidence: `FUN_006AE6E0`)
- `[RESOLVED] OQ-08 - Are there direct immediate helper sends in the binary? -> Yes, two helper thunks send `0x4E5`/`0x4E6`, and one unrelated path pushes `0x4E5`.` (evidence: byte scan; assembly `0x00603DB9`, `0x00603DD9`, `0x0052EC81`)
- `[RESOLVED] OQ-09 - Do the helper thunk callers target the five standard Skirmish checkboxes? -> No; verified caller table IDs are `0x6CF..0x6D5`/`0x6D6..0x6E4`, and the other assembly path uses `0x50F`/`0x670`.` (evidence: `FUN_00658330`; assembly `0x0052EC5F..0x0052EC9F`; table bytes at `0x0083923C/0x0083925C`)
- `[RESOLVED] OQ-10 - Which PCXs are live in standard `0x102`? -> `cue_i.pcx` unchecked and `cce_i.pcx` checked.` (evidence: `OwnerDraw_Checkbox_006163A0`; zero variant defaults and standard init no variant sends)
- `[RESOLVED] OQ-11 - Are `cce_il.pcx` / `cce_ir.pcx` live in standard `0x102`? -> No; they require nonzero variant state, and no standard writer changes the bytes.` (evidence: `0x006164E1..0x006165C8`; `FUN_006AE6E0`)
- `[RESOLVED] OQ-12 - Does this require any `sim/` state? -> No; all behavior is shell owner-draw message/render state before launch.` (evidence: call path and project layering)

Zero-add pass result: re-reading `OwnerDraw_Checkbox_006163A0`, `FUN_006AE6E0`, and the binary immediate hits added no new target-scope writer sites.

Adversarial corner checks answered from evidence:

- Missing checkbox child: `FUN_006AE6E0` guards `GetDlgItem`; no send means zero variant state is unchanged.
- Non-1 checked value: affects `+0xE8`/checked art only, not variant bytes.
- `0x4E7` before any writer: returns zero because the record was zero-initialized.
- Label click vs icon click: click behavior toggles only `+0xE8`; it never writes variants.
- Disabled control: disabled overlay/text-color branch does not select variant assets.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard offline Skirmish `0x102` checkboxes never receive `0x4E5/0x4E6`; variant bytes stay `0/0` | `FUN_0060F9A0` zero-init; `FUN_006AE6E0` sends only `0xF1`; immediate hit scan excludes target IDs | missing checkbox rendering/state | `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs` | Render standard five option checkboxes using only default checked/unchecked art | Test proposal: `skirmish_standard_checkboxes_use_default_art_variants_only`; scenario: fresh offline Skirmish renders unchecked as `cue_i.pcx` and checked as `cce_i.pcx`, with no left/right variant role requested | Do not implement `cce_il.pcx`/`cce_ir.pcx` for standard `0x102` by default |
| The variant helpers exist but are not standard `0x102` setup | helper thunks `0x00603DB0/0x00603DD0`; callers `0x00657EFF`, `0x00657F45`, `0x00658520` target non-standard control IDs | unchecked for non-standard shells | future shell-wide UI, not current Skirmish standard path | Keep data model open enough to add variants later if a non-standard shell/lobby needs them | Test proposal: `skirmish_checkbox_variant_assets_are_not_loaded_for_standard_shell`; scenario: standard shell asset manifest does not require `cce_il.pcx` or `cce_ir.pcx` | Do not delete parser/asset support globally if later shell work needs variants |
| `0xF1` initializes checked state only; it does not affect variant bytes | callback branch `0xF1`; standard init assembly `0x006AEDF3..0x006AEE1F` | checkbox checked state missing/partial | `src/ui/skirmish_shell/state.rs` input/apply path | Store checked state separately from art variant; standard default variant is constant zero | Test proposal: `skirmish_checkbox_setcheck_does_not_change_art_variant`; scenario: toggling Short Game changes only checked art pair, never alternate art family | Do not conflate checked state with left/right variant state |

### Stale Docs / Follow-up Docs

- Replace the prior deferred wording in `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md` OQ15 with: "`[RESOLVED] OQ15 - Standard offline Skirmish `0x102` checkboxes do not receive `0x4E5/0x4E6`; records are zero-initialized in `FUN_0060F9A0`, `FUN_006AE6E0` sends only `0xF1` to `0x54E/0x69A/0x69D/0x693/0x696`, and the only direct immediate helper sends target non-standard control IDs. See `SKIRMISH_CHECKBOX_OWNERDRAW_VARIANT_WRITERS_GHIDRA_REPORT.md`.`"
- Narrow the earlier shorthand "Helper active; standard `0x102` leaves zero" in `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md` to: "Helper active conditionally outside standard `0x102`; the standard offline Skirmish five option checkboxes leave `+0xD9/+0xDA` at zero and use only `cue_i.pcx` / `cce_i.pcx`."

### Negative Facts / Do Not Do

- Do not use `cce_il.pcx` or `cce_ir.pcx` for standard offline Skirmish `0x102` checkbox rendering.
- Do not treat `BM_SETCHECK (0xF1)` as a variant writer; it writes checked state `+0xE8` only.
- Do not infer variant use from asset preload or callback support; standard init path does not send the variant messages.
- Do not model checkbox art variants in `sim/`; this is shell UI owner-draw state only.
- Do not broaden the non-standard helper callers into a standard Skirmish requirement; their verified control ID tables are not `0x54E/0x693/0x696/0x69A/0x69D`.

### Remaining Uncertainty

None for standard offline Skirmish dialog `0x102`. Non-standard lobbies/shells may use the helper thunks, but they are outside this report's claimed scope.

## Sources

- Ghidra decompile/read-only: `OwnerDraw_Checkbox_006163A0`, `FUN_006AE6E0`, `FUN_0060F9A0`, `FUN_00658330`.
- Ghidra assembly contexts: `0x00616833..0x00616854`, `0x006168A6..0x006168C7`, `0x006100F2..0x00610190`, `0x0061032B..0x00610339`, `0x006AEDC9..0x006AEE1F`, `0x00603DB9`, `0x00603DD9`, `0x0052EC5F..0x0052EC9F`, `0x00657EFF`, `0x00657F45`, `0x00658520`.
- Binary scan: local retail `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe` direct immediate search for `PUSH 0x4E5`, `PUSH 0x4E6`, and common compare forms.
- Prior research checked: `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOXES_AND_TRACKBARS_GHIDRA_REPORT.md`.
- Rust surfaces checked: `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`.
