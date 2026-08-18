# Skirmish Shell Focus/Hover Review Plan

Review target: `docs/plans/2026-05-23-skirmish-shell-focus-hover-plan.md`

Purpose: catch parity drift while implementing the player-name edit/focus hardening
and `0x695` hover status resolver. This review plan is for code review and
verification after implementation, not another research pass.

---

## Review Stance

Treat this as a parity-risk review. Findings should focus on player-visible
behavior, stale state, wrong status strings, focus/input leakage, modal ownership,
and accidental broadening of shell control models.

Do not require literal Win32 message plumbing. Do require the observable outcomes
verified by the reports.

---

## Primary Review Sources

- `docs/plans/2026-05-23-skirmish-shell-focus-hover-plan.md`
- `docs/research/skirmish-ui/SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_PLAYER_NAME_EDIT_FOCUS_MESSAGES_0X4B0_0X4AF_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_SHELL_INPUT_FOCUS_MESSAGE_BROAD_RECHECK_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`

---

## Review Checkpoints

### 1. Model Boundaries

Check:

- `sim/` remains untouched by shell focus/status behavior.
- Hover/status identity is a dedicated target type, or `ShellControlId` is clearly
  guarded as an incomplete subset.
- Dynamic row/item identity is represented without pretending to model all `0x102`
  children.
- No pseudo-Win32 message bus is introduced for `0x4B0`, `0x4AF`, `0x4E8`, or
  `0x4E9`.

Reject if:

- A complete-looking `0x102` control enum is added without evidence and guardrails.
- Status/help resolution is spread ad hoc through `app.rs` hit-test branches.

### 2. Player-Name Edit

Check:

- Focused edit consumes printable text, Backspace, Delete, Left, Right, Home, End,
  and Tab before global hotkeys.
- First focus/select-all behavior remains covered.
- Text remains capped at 19 chars.
- Tab does not insert control text and does not claim an unverified exact native
  next-focus target unless separately traced.
- Escape behavior is not documented or tested as retail parity.
- Start launch still reads `player_name_edit.text`.

Reject if:

- Text input can leak into global shortcuts while the edit is focused.
- Any test/comment claims Escape-to-Back or Enter-to-Start parity from this work.

### 3. Hover Status Resolver

Check:

- `0x695` self-hover resolves to blank.
- No-hover resolves to blank.
- Status keys use `STT:*`, not visible `GUI:*` labels.
- AI row-state item-specific overrides take precedence over generic AI combo
  fallback:
  - `-1 -> STT:PlayerNone`
  - `2 -> STT:PlayerDumbAI`
  - `1 -> STT:PlayerSmartAI`
  - `0 -> STT:PlayerGeniusAI`
- Start controls use `STT:HostComboStart`, not invented
  `STT:SkirmishComboStart`.
- Team controls use `STT:HostComboTeam`.
- Checkbox mappings match the verified checkbox report.

Reject if:

- Any status text is copied from rendered labels.
- Unknown controls keep stale prior status text.
- Start/team combo status keys are guessed from naming patterns.

### 4. Modal Ownership

Check:

- Opening Choose Map or validation modal clears existing parent `status_help_text`.
- Parent shell hover/status updates are blocked while a modal owns input.
- Parent `0x102` status text is not drawn under or inside the Choose Map modal,
  even if stale state exists.
- No modal-specific status behavior is invented for `0x6B`.

Reject if:

- Hover Start, open Choose Map, and the Start help text remains visible.
- Parent controls update status while modal is open.

### 5. Render Path

Check:

- Status text draw is tied to `layout.status_help`.
- Empty status text emits no visible text.
- Non-empty status text is scoped to the `0x695` rect and shell text scissor/color
  path.
- Existing edit frame/text/selection/caret rendering is not regressed.

Reject if:

- `0x695` is treated as a right-panel static or permanent label.
- Modal draw path duplicates parent status text.

---

## Required Test Gates

Run:

```powershell
cargo test player_name --lib
cargo test status_help --lib
cargo test skirmish_shell --lib
cargo fmt
```

Expected new or updated coverage:

- hover identity resolves Start, player edit, status strip, checkbox, trackbar,
  combo face, and open dropdown row.
- status key tests cover Start, player edit, self-hover blank, AI item override,
  checkbox, trackbar, preview, side combo, start combo, and team combo.
- modal-open render suppresses parent status text even with stale parent state.
- focused player edit consumes text/control keys before global shortcuts.

---

## Manual Review Scenario

At 800x600 in the Rust native Skirmish shell:

1. Fresh shell: status strip blank.
2. Hover Start: localized Start help appears.
3. Hover player name: localized player edit help appears.
4. Hover status strip: status clears.
5. Hover Start, open Choose Map: status clears and stays blocked.
6. Click player name, type one char: default `Player` is replaced.
7. Type 20 printable chars: displayed/stored text caps at 19.
8. Press Tab while focused: no tab character appears; edit focus leaves without
   claiming exact native destination.
9. Start launch carries the edited name.

---

## Review Output Format

Use code-review style:

1. Findings first, ordered by severity.
2. Each finding must include file and line reference.
3. Focus on player-visible risk and parity evidence.
4. Then list open questions.
5. Then summarize tests run and residual risk.

If no issues are found, say so directly and still mention any unverified behavior
left outside this pass, especially Escape/Enter and Choose Map `0x6B` status.
