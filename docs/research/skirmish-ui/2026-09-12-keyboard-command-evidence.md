# Keyboard-exposed HealthNav and CursorCheat

Original source: `gamemd.exe`, SHA256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Instructions and data were independently read from the original image using
Ghidra read-only listings and Capstone over the original PE sections. Existing
annotations were leads. No Ghidra mutations were performed by this owner.

## Native behavior established

HealthNav's vtable `7EB93C` is registered at `532EB2`; its name getter
`5368D0` returns literal `HealthNav` at `827334`. Modifier admission `536940`
tests only Shift. Execute `536950` passes inverted Shift to `733380`.

`733380` returns when `A8B538` is nonzero. In an ordinary live skirmish:

- If selection mode `B0FE54` is not3, discard the old snapshot, copy selected
  techno entries in current selection order (`732050`, `A8ECBC/A8ECC8`). If
  empty, collect tactical draw-array entries using `731F70` and `7335F0`.
  That fallback requires an alive local-owner non-building techno. Always
  clear current selection on this fresh invocation, including Shift, and
  begin with category0.
- In mode3 retain the snapshot and advance category0→1→2→0. Ordinary press
  clears current selection; Shift skips that clear. Categories without
  members are not skipped. All matching snapshot members call Select+14C.
- `5F5DD0` classifies current health divided by type Strength: category0 is
  positive health at or below ConditionRed; category1 is above ConditionRed
  and at or below ConditionYellow; category2 is the remainder. Ratios and
  native rule thresholds use floating point. This filters groups; it does
  not move the camera or sort individual units by health.
- Selected Unit vtable slot `7F5DBC` resolves to `6FBFA0`. Successful base
  selection calls selection voice+360 when the existing voice latch permits
  it; HealthNav does not suppress that latch. Mouse action-line timing is
  not started by this command.
- `733160` removes expired objects from the snapshot. Ordinary selection
  clears the shared mode through `731D00`; HealthNav restores mode3 after
  its Select calls only if its snapshot is nonempty, and clears across-map.
- Feedback uses `MSG:Critical`, `MSG:HeavilyDamaged`, `MSG:Healthy`; selection
  containing another category produces `MSG:Mixed`. `731D90` uppercases
  the category via `7DDCD6` (ASCII a..z in the original C-locale branch),
  then formats `MSG:NoUnitsSel`, `MSG:UnitsWorth`, or `MSG:NavEmpty`.
  Worth is a sum of each selected type's house-adjusted cost virtual+84.
  `73358C..7335A7` posts silently for240 native timer buckets, using the
  local-house color scheme with runtime scheme3 fallback.

CursorCheat's vtable `7EBF54` is installed by `537E70`, unconditionally
registered through `533BF2`; getter `537E80` returns literal `CursorCheat`
at `827ADC`. Execute `537EF0` only toggles byte `A8F7D8`.

- `Set_View_Dimensions` registers tooltip region500 over the entire tactical
  viewport at `4A8B50..4A8B85`. Dynamic callback `4AE4F0`, also reached through
  `658770`, accepts region IDs500..900 and the normal display gate.
- `692300` resolves cursor position. When the flag is set, `4AE580..4AE5BE`
  returns UTF-16 literal `(%d,%d)` from `8201C4`, using each world coordinate
  divided by256 with signed truncation toward zero. This precedes ordinary
  object/shroud text; it does not expose hidden object identities.
- Mouse move `724251→72429E` bypasses the show delay, re-hit-tests the current
  tooltip region and refreshes through `724AD0`. Existing enabled gating,
  button dismissal and10000ms duration remain. The toggle itself does not
  immediately draw or hide anything. The flag also makes sidebar tips
  immediate. It is not the pause flag, despite an older tooltip report's
  annotation.

## Implementation and acceptance

`input/selection_navigation.rs` uses stable-ID snapshots, current selection
ordering, existing selection admission/insertion/voice dispatch, and the
existing silent message surface. TypeSelect and HealthNav now share one
selection-mode enum; ordinary selection invalidates both. Snapshot departures
are reconciled with entity lifecycle. The app's retained input toggle drives
the existing tooltip service and renderer; immediate movement refresh is a
service operation, not another independently painted overlay. Modal owners
suppress tactical tooltips.

New-map installation and successful prepared-load commit clear navigation and
the pending selection ledger before replacing the world. Reusing a stable ID
cannot revive an old pointer snapshot. Failed load preparation does not reset
the current group. Native snapshot destruction is explicit at `730970`, and
individual expired-object removal at `733160`; the Rust replacement boundary
is the stable-ID counterpart. Process-retained hotkey bindings and the
CursorCheat toggle are preserved.

Acceptance: assign both commands through Keyboard; cycle a mixed-health
selection three times and wrap, Shift-add a subsequent category, then manually
select another group and verify the next press restarts. Empty selected group
uses local visible mobile objects. Damage/departures are reflected on later
presses. Selection voices and localized feedback retain their normal owners.
Toggle coordinates, move across tactical cells and sidebar, click to dismiss,
toggle off, and check ordinary tooltip delay/disabled preference. Opening a
shell must not leak a coordinate tip or gameplay input.

Added regression tests cover retained order/category cycling/restart and
departures, threshold membership, feedback variants, shared mode invalidation,
and immediate tooltip refresh/dismissal/expiry/ordinary-delay restoration.
These are Rust regression assertions, not native-execution goldens. This
implementation owner ran no Cargo commands; integration validation and fresh
independent criticism remain the parent owner's responsibility.

## Explicit limits

The existing production model prices units using `ObjectType.cost`; navigation
feedback uses that same value. House-adjusted cost virtual equivalence is not
established, so modified house costs can make the displayed worth differ.
The existing tactical inverse is reused for coordinate display. Off-map
sentinels, subcell boundary/bridge inverse precision and the inherited tooltip
placement limits have no new native executable comparison here. Existing
rule thresholds are stored as f32; stock0.25/0.5 are exact, while arbitrary
modded decimal thresholds retain that shared precision limitation. Localized
non-C-locale uppercase handling beyond the original ASCII branch is not added.
These qualifications prevent whole-command or whole-shell parity claims.
