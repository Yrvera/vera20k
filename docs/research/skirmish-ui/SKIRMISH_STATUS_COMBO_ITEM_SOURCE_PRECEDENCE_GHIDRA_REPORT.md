# Skirmish Status Combo Item Source Precedence - Ghidra Research Report

**Address(es):** `0x00622B50`, `0x00603F00`, `0x006AE3F0`, `0x006040B0`, `0x004E3830`, `0x004E4170`, `0x004E38A0`, `0x004E4230`, `0x004E4E20`, `0x004E42A0`, `0x004E4EC0`, `0x004E5900`, `0x004E4F30`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard offline YR Skirmish dialog `0x102` status-help precedence for combo faces and open dropdown rows: `0x4E8` hovered item result, parent `0x4E9` item-specific text, second parent `0x4E9` with item/index `-1`, `FUN_006040B0` generic control fallback, and blank fallback.  
**Non-Scope:** exact side/country or color item-data-to-STT key tables, CSF availability audit, online host/guest dialogs, Choose Map modal row contracts, rendering composition, and Rust edits.  
**Confidence:** High for source order, `-1` behavior, hovered-row precedence over combo-face help, and current Rust delta.  
**Active in YR:** Yes for standard offline Skirmish dialog `0x102`; item-specific row text is Conditional on an open dropdown returning a non-`-1` item index.

## 0. Working Notes Gate

- Target question: Confirm when open dropdown row item text beats generic combo face help; hovered row vs selected item; fallback when item is `-1`.
- Non-goals: Do not build the side/color STT table, do not audit CSF key availability, do not modify Rust, do not re-cover the full status strip map.
- Evidence needed to mark COMPLETE: `0x4E8` row-index source, common wrapper order, parent `0x4E9` guard/order, side/color/start helper argument behavior, `FUN_006040B0` static fallback evidence, Rust surface comparison.
- Stop conditions: Stop when the resolver contract can say exactly what happens for open row, collapsed face, item-specific miss, and `-1`, with no unresolved source-order questions.

## 1. Overview

The native resolver is ordered, not label-driven. The common shell handler asks the hovered child for an item/index via `0x4E8`, asks the Skirmish parent `0x4E9` for item-specific text using that exact child/index, and only if the shared string holder is still empty does it try parent `0x4E9` again with item/index `-1`, then `FUN_006040B0` generic control help, then blank.

For combo dropdowns, an open row can beat the generic combo face help only when `0x4E8` returns a non-`-1` row index and parent `0x4E9` resolves item-specific text. In the Skirmish parent status path, `item/index == -1` does not mean "use the selected item"; it causes parent `0x4E9` to skip item-specific logic, after which generic `FUN_006040B0` face help is used if the hovered child has a static mapping.

## 2. Key Offsets / Message Fields

| Field / message | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `0x4E8` child message | Hit-test item/index query sent to hovered child with packed cursor coordinates | `0x00622D4B..0x00622D5E`; prior listbox report `0x0061BB47..0x0061BBD9` | Yes / Conditional on hovered child |
| parent `0x4E9` payload `[0]` | Hovered child HWND passed to parent status resolver | `FUN_00603F00`; second pass writes child at `0x00622DBD` | Yes |
| parent `0x4E9` payload `[+4]` | Item/index result from `0x4E8`, or explicit `-1` on second pass | first pass pushes `0x4E8` return at `0x00622D5C`; second pass writes `0xFFFFFFFF` at `0x00622DCA` | Yes |
| `CB_GETCURSEL 0x147` | Selected-index fallback inside combo item helpers when called with `-1` | `0x004E417D..0x004E4193`, `0x004E4E2D..0x004E4E43` | Conditional helper behavior, not reached by Skirmish parent `0x4E9` second pass |
| `CB_GETITEMDATA 0x150` | Converts item index to combo item data | `FUN_004E4170`, `FUN_004E4E20`, AI branch `0x006AE5DA..0x006AE600` | Yes / Conditional on non-`-1` parent item index |

## 3. Core Logic

### 3.1 Common Wrapper Order

Active in YR: Yes. `FUN_006AE3F0` delegates to `FUN_00622B50` before Skirmish-specific message handling, so the common status path is live for dialog `0x102`.

Active in YR: Yes. In `FUN_00622B50`, the first source is the hovered child: `0x00622D4B..0x00622D54` sends `0x4E8`, then `0x00622D56..0x00622D5E` calls `FUN_00603F00` with the hovered child and the returned item/index. `FUN_00603F00` sends parent message `0x4E9` with that child/index payload.

Active in YR: Yes. If the string holder is still empty, `0x00622DB0..0x00622DCA` sends parent `0x4E9` again with the same child and item/index `0xFFFFFFFF`. If that still leaves the holder empty, `0x00622E1D..0x00622E38` calls `FUN_006040B0(parent, hovered_child)`. If that returns null, the handler keeps the empty string and sends it to child `0x695` at `0x00622E6D..0x00622E83`.

Resolver contract:

1. Try item-specific text for the hovered child/index returned by `0x4E8`.
2. If no text, try parent `0x4E9` with index `-1`.
3. If no text, try generic static control help from `FUN_006040B0`.
4. If no static key, send blank text.

### 3.2 Open Dropdown Row vs Collapsed Face

Active in YR: Conditional. Prior verified listbox evidence says the custom `0x4E8` handler returns `top_index + y / item_height` for an in-bounds list row and `-1` outside row/client bounds (`0x0061BB47..0x0061BBD9`). The combo scrollbar report verifies the combo-open path forwards `0x4E8` to the active dropdown.

Active in YR: Yes. A collapsed combo face has no row index for parent item-specific handling. The relevant parent `0x4E9` pass receives `-1` or leaves the holder empty, then `FUN_006040B0` maps the combo control id to the generic face help, such as `STT:SkirmishComboCountry`, `STT:SkirmishComboColor`, `STT:HostComboStart`, or `STT:HostComboTeam`.

Active in YR: Conditional. An open dropdown row beats generic face help only if the first parent `0x4E9` pass receives a non-`-1` index and writes a non-empty item-specific string. Evidence: parent `0x4E9` checks item/index at `0x006AE4CA..0x006AE4CE`, branches into side/color/start item helpers at `0x006AE531..0x006AE5C9`, and returns immediately after writing item text.

### 3.3 `-1` Does Not Mean Selected Item In This Parent Status Path

Active in YR: Yes. The Skirmish parent `0x4E9` handler clears its output holder, then checks both payload fields. If the child is null or the item/index field is `-1`, it jumps to the success return without resolving item-specific text (`0x006AE4B4..0x006AE4CE`, target `0x006AE6C6..0x006AE6CF`). This means the second common-wrapper parent call with `{child, -1}` cannot synthesize selected-item text in standard Skirmish `0x102`.

Active in YR: Conditional helper behavior. The side/color/start helpers still contain their own selected-index fallback when their index argument is `-1`: `FUN_004E4170` and `FUN_004E4E20` call `CB_GETCURSEL (0x147)` before `CB_GETITEMDATA (0x150)`, and `FUN_004E5900` has the same shape. That fallback is not reached from the Skirmish parent status path's second `-1` pass because the parent guard exits before calling the helpers.

Implementation consequence: the resolver must not use the selected combo item as a substitute when no dropdown row is hovered. Collapsed face hover should show generic combo help, not the selected country's/color's item-specific help.

### 3.4 Item-Specific Miss Then Generic Fallback

Active in YR: Conditional. For side/country controls, `FUN_006AE3F0` recognizes control ids through `FUN_004E3830`, calls `FUN_004E4170` with the hovered index, and only writes item text if the returned item data is not `-1` (`0x006AE5A9..0x006AE5C9`). `FUN_004E4170` accepts item data in inclusive range `-3..9`; outside that range it falls back to a global side source if present, otherwise returns `-2`.

Active in YR: Conditional. For color controls, `FUN_006AE3F0` recognizes ids through `FUN_004E4230`, calls `FUN_004E4E20` with the hovered index, then calls `FUN_004E42A0`. `FUN_004E42A0` maps `-2` and `0..8`; other values return null. If null leaves the holder empty, the wrapper continues to second `0x4E9` and then `FUN_006040B0`, so the visible fallback becomes generic color combo help.

Active in YR: Conditional. Start-position controls use `FUN_004E4EC0 -> FUN_004E5900 -> FUN_004E4F30`; the item-specific path writes the same generic `STT:HostComboStart` string used by `FUN_006040B0`, so open row and collapsed face are not visibly distinct for start status help in this slice.

Active in YR: Yes. Team controls have no item-specific branch in the scoped Skirmish `0x4E9` handler; they reach generic `STT:HostComboTeam` through `FUN_006040B0`.

## 4. INI Keys

No INI key controls this resolver order. The behavior is Windows-message, dialog-control-id, combo item-data, and string-table driven. Active in YR: Yes, because the verified path is live from standard dialog `0x102`.

## 5. Integration Points

| Function / area | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_006AE3F0` | Standard offline Skirmish dialog proc; delegates common handler and owns parent `0x4E9` item text | decompile; `0x006AE4A3` checks `0x4E9` | Yes |
| `FUN_00622B50` | Common shell status update on `WM_NCHITTEST (0x84)` | decompile; `0x00622D4B..0x00622E83` | Yes |
| `FUN_00603F00` | Packages child/index and sends parent `0x4E9` | decompile and call at `0x00622D5E` | Yes |
| `FUN_006040B0` | Dialog/control id generic `STT:*` fallback | decompile `0x102` branch | Yes |
| owner-draw listbox/dropdown `0x4E8` | Supplies hovered row index or `-1` | prior reports `0x0061BB47..0x0061BBD9`, combo-forward evidence | Conditional while combo is open |

## 6. Current Rust Implementation Status

| Rust surface | Status | Evidence |
|---|---|---|
| Open-row hit testing | structurally aligned: open dropdown rows return `SkirmishHoverTarget::ComboItem` before face hit testing | `src/ui/skirmish_shell/state/hit_test.rs:20`, `:49` |
| Collapsed face hit testing | structurally aligned: face returns `SkirmishHoverTarget::ComboFace` | `src/ui/skirmish_shell/state/hit_test.rs:77`, `:89` |
| Generic face help | implemented for AI/side/color/start/team | `src/ui/skirmish_shell/state/hit_test.rs:196` |
| AI row item-specific help | implemented | `src/ui/skirmish_shell/state/hit_test.rs:138`, `:206` |
| Side/color row item-specific help | mismatch: all non-AI `ComboItem` values fall back to generic combo help | `src/ui/skirmish_shell/state/hit_test.rs:142` |
| Selected-item fallback | Rust does not visibly implement selected item fallback in `status_help_key_for_hover`; this matches the parent status-path contract for collapsed faces | same surface; native guard `0x006AE4CA..0x006AE4CE` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00622B50` first `0x4E8` source | verified | `0x00622D4B..0x00622D5E` | none |
| `FUN_00603F00` parent `0x4E9` send | verified | decompile `0x00603F00` | none |
| second parent `0x4E9` with `-1` | verified | `0x00622DB0..0x00622DCA` | none |
| `FUN_006AE3F0` `-1` guard | verified | `0x006AE4B4..0x006AE4CE`, `0x006AE6C6..0x006AE6CF` | none |
| side helper call order | verified | `0x006AE531..0x006AE5C9`, `FUN_004E4170`, `FUN_004E38A0` | exact STT key table belongs to slots 1/5 |
| color helper call order | verified | `0x006AE53F..0x006AE598`, `FUN_004E4E20`, `FUN_004E42A0` | exact STT key table belongs to slots 2/5 |
| start helper call order | verified | `0x006AE549..0x006AE570`, `FUN_004E5900`, `FUN_004E4F30` | none for source order |
| team generic-only fallback | verified | no team-specific branch in scoped `0x4E9`; `FUN_006040B0` maps `0x76D..0x774` | none |
| open dropdown `0x4E8` row index | verified from prior focused report | `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`, `0x0061BB47..0x0061BBD9` | none for source order |
| current Rust resolver shape | verified | `state/hit_test.rs` and `state/combos.rs` scan | implementation remains |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-001 - Is the common source-order path active in standard YR Skirmish? -> Yes, `FUN_006AE3F0` delegates to `FUN_00622B50`, which updates status child `0x695`.` (evidence: `0x006AE3F0`, `0x00622D4B..0x00622E83`)
- `[RESOLVED] OQ-002 - Does an open dropdown row get a chance before generic combo face help? -> Yes, the hovered child receives `0x4E8`; non-`-1` row index is passed to parent `0x4E9` before `FUN_006040B0`.` (evidence: `0x00622D4B..0x00622D5E`, `0x006AE4CA..0x006AE5C9`)
- `[RESOLVED] OQ-003 - Does collapsed face use selected item-specific text? -> No in this parent status path; item/index `-1` skips item-specific parent logic and then reaches generic fallback.` (evidence: `0x006AE4CA..0x006AE4CE`, `0x00622E1D..0x00622E38`)
- `[RESOLVED] OQ-004 - Are helper-level `-1 -> selected` fallbacks real? -> Yes, side/color/start helpers contain `CB_GETCURSEL` fallback, but the Skirmish parent `0x4E9` guard prevents the second `-1` pass from reaching them.` (evidence: `0x004E417D..0x004E4193`, `0x004E4E2D..0x004E4E43`, `0x006AE4CA..0x006AE4CE`)
- `[RESOLVED] OQ-005 - What happens if item-specific mapping writes no text? -> The common wrapper observes the holder is still empty and proceeds to second `0x4E9`, then `FUN_006040B0`, then blank.` (evidence: `0x00622D95..0x00622E38`)
- `[RESOLVED] OQ-006 - Do side/color open rows need item-specific resolver before generic key? -> Yes, parent `0x4E9` calls side/color item helpers before static fallback.` (evidence: `0x006AE531..0x006AE5C9`, `0x006AE581..0x006AE598`)
- `[RESOLVED] OQ-007 - Do start open rows need distinct row-specific status text? -> No visible distinction found in this slice; start item path loads `STT:HostComboStart`, matching static fallback.` (evidence: `0x006AE559..0x006AE570`, `FUN_004E4F30`)
- `[RESOLVED] OQ-008 - Do team open rows have item-specific parent text? -> No scoped team branch in parent `0x4E9`; static fallback supplies `STT:HostComboTeam`.` (evidence: `FUN_006AE3F0`, `FUN_006040B0`)
- `[RESOLVED] OQ-009 - Does current Rust already model open-row precedence structurally? -> Yes for hit-test order: open `ComboItem` before `ComboFace`.` (evidence: `state/hit_test.rs:20..37`, `:49..52`)
- `[RESOLVED] OQ-010 - Does current Rust already model side/color item-specific status? -> No, non-AI `ComboItem` falls back to generic combo key.` (evidence: `state/hit_test.rs:138..142`)
- `[RESOLVED] OQ-011 - Are INI defaults involved? -> No scoped INI reader participates; this is binary UI message/control/string behavior.` (evidence: decompiled function set)
- `[RESOLVED] OQ-012 - Is this TS legacy? -> No, the path is reached by standard YR Skirmish dialog `0x102` and uses Skirmish-specific control ids and strings.` (evidence: `FUN_006AE3F0`, `FUN_006040B0` `iVar4 == 0x102`)

## 9. Visual/UI Composition Ledger

Not applicable to this slice. This report covers text source precedence for the status strip, not the paint order or visual composition of the strip.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Open dropdown row item-specific text precedes generic combo face help when `0x4E8` returns a non-`-1` row index | `0x00622D4B..0x00622D5E`; `0x006AE531..0x006AE5C9`; prior listbox `0x0061BB47..0x0061BBD9` | mismatch for side/color rows | `src/ui/skirmish_shell/state/hit_test.rs::status_help_key_for_hover` or a richer resolver | Resolve side/country and color `ComboItem` status before falling back to `status_help_key_for_combo` | Open Side dropdown and hover Random/country row; status uses item-specific `STT:PlayerSide*`, not `STT:SkirmishComboCountry` | Do not use generic combo help for every non-AI row; proposed test `test_skirmish_status_open_combo_item_precedes_generic_face_help` |
| `item/index == -1` in the Skirmish parent `0x4E9` status path skips item-specific text; collapsed face falls through to generic `FUN_006040B0` help | `0x006AE4CA..0x006AE4CE`; `0x00622DB0..0x00622E38`; `FUN_006040B0` `0x102` branch | current collapsed-face generic behavior matches; ensure future item resolver does not regress | `hovered_shell_control`, `status_help_key_for_hover`, combo selected-item helpers | Keep selected-item-specific help out of collapsed face hover; only open row hover can use item-specific text | Hover collapsed Side combo with selected Korea; status remains `STT:SkirmishComboCountry`, not Korea side help | Do not treat the second `-1` pass as selected-item fallback; proposed test `test_skirmish_status_combo_face_uses_generic_help_not_selected_item` |
| If item-specific row resolution misses or writes empty, generic static combo help is still attempted before blank | `0x00622D95..0x00622E38`; `FUN_004E42A0` null default for unmapped color data; `FUN_006040B0` static combo cases | unchecked for abnormal unmapped Rust item-data cases | status resolver around combo item-data conversion | Preserve fallback chain: item-specific row text, then generic combo key, then empty | Inject/construct an unmapped color sentinel row and verify status falls back to `STT:SkirmishComboColor` rather than selected color or visible row label | Do not stop at blank after item-specific miss while a generic control key exists; proposed test `test_skirmish_status_combo_item_miss_falls_back_to_generic_combo_help` |

### Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md`: replace "send parent message `0x4E9` with `{hovered_hwnd, -1}` so the Skirmish proc can synthesize item-specific text" with "send parent message `0x4E9` with `{hovered_hwnd, -1}`; in standard Skirmish `0x102`, the parent `0x4E9` handler skips item-specific combo logic for `-1`, so this pass normally leaves the holder empty and allows the later `FUN_006040B0` generic control fallback."
- `docs/research/skirmish-ui/SKIRMISH_0X102_STATUS_HELP_FULL_MAPPING_CURRENT_RUST_GHIDRA_REPORT.md`: after the source-order paragraph, add "For standard Skirmish `0x102`, the second parent `0x4E9` pass with item/index `-1` is not a selected-item fallback for combo status text; `FUN_006AE3F0` exits item-specific handling when the payload index is `-1`, so collapsed combo faces resolve through `FUN_006040B0` generic combo keys."

## 11. Negative Facts / Do Not Do

- Do not use the selected combo item as status help for a collapsed combo face. Active in YR: Yes. Evidence: parent `0x4E9` guard at `0x006AE4CA..0x006AE4CE` skips item-specific handling for `-1`; generic fallback follows at `0x00622E1D..0x00622E38`.
- Do not interpret the helper-level `CB_GETCURSEL` fallback as proof that the Skirmish parent status path uses selected-item text. Active in YR: No for this path. Evidence: helper fallback exists at `0x004E417D..0x004E4193` and `0x004E4E2D..0x004E4E43`, but parent guard exits first.
- Do not use visible dropdown row labels as status help. Active in YR: Yes. Evidence: side/color rows load string-table ids through `FUN_004E38A0` / `FUN_004E42A0`; generic fallback uses `FUN_006040B0` `STT:*` keys.
- Do not collapse open-row and face hover into the same source for side/color combos. Active in YR: Conditional on open dropdown. Evidence: first pass item helpers at `0x006AE531..0x006AE5C9` precede generic static fallback at `0x00622E1D`.
- Do not make team rows item-specific without new evidence. Active in YR: No item-specific team branch found in this scoped parent `0x4E9`; `FUN_006040B0` maps team controls to `STT:HostComboTeam`.

## 12. Remaining Uncertainty

None for this target's resolver ordering contract. Exact side/color item-data-to-STT tables and CSF key availability are intentionally owned by sibling slots.

## Sources

- Ghidra read-only decompile: `FUN_00622B50 @ 0x00622B50`, `FUN_00603F00 @ 0x00603F00`, `FUN_006AE3F0 @ 0x006AE3F0`, `FUN_006040B0 @ 0x006040B0`, `FUN_004E3830 @ 0x004E3830`, `FUN_004E4170 @ 0x004E4170`, `FUN_004E38A0 @ 0x004E38A0`, `FUN_004E4230 @ 0x004E4230`, `FUN_004E4E20 @ 0x004E4E20`, `FUN_004E42A0 @ 0x004E42A0`, `FUN_004E4EC0 @ 0x004E4EC0`, `FUN_004E5900 @ 0x004E5900`, `FUN_004E4F30 @ 0x004E4F30`.
- Ghidra assembly context: `0x00622D4B..0x00622E83`, `0x006AE4B4..0x006AE6CF`, `0x004E417D..0x004E4193`, `0x004E4E2D..0x004E4E43`.
- Prior docs: `docs/research/skirmish-ui/SKIRMISH_0X102_STATUS_HELP_FULL_MAPPING_CURRENT_RUST_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`.
- Rust scan only, no edits: `src/ui/skirmish_shell/state/hit_test.rs`, `src/ui/skirmish_shell/state/combos.rs`, `src/app.rs`.
