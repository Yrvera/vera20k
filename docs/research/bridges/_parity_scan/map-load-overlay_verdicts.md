# Bridge Parity Scan — Facet: map-load-overlay — ADVERSARIAL VERDICTS

Auditor re-verified each D<n> against the live binary (Ghidra MCP this session) and current
Rust. Burden of proof: DRIFT-by-default; REAL only if the finder's gamemd reading holds live
AND observable output differs; UNCERTAIN if the gamemd side could not be independently
confirmed or the only-difference is unreachable/unproven-observable.

Live calls this session:
- `get_function_by_address 0x0047E040` → CellClass__SetBridgeDirection_NESW (confirmed).
- `decompile_function 0x0047E040` (dir-6 extra block re-read).
- `get_function_by_address 0x005FC570` → OverlayClass__Mark (confirmed); `decompile_function 0x005FC570`.
- `read_memory 0x0089F688` (g_DirectionOffsets) → all-zero in this static image (runtime-populated; literal dx/dy not byte-readable).
- `get_xrefs_to 0x0089F688` → confirms it is the direction-offset table read by SetBridgeDirection / pathfinding / anim.

---

## D1: AnchorSpan slot-5 (dir-6 extra) placed 1 cell short — VERDICT=REAL

`decompile_function 0x0047E040`: dir-6 extra block reads `param_1->MapCoord` AFTER `param_1`
was reassigned to the OPPOSITE cell in the opposite block (`uVar15 = (param_2-4)&7 = 2` for
dir 6, `param_1 = Get_CellClass(anchor + offset[2])`), then adds `DAT_0089f690`
(= `g_DirectionOffsets` index 2, the SAME offset the opposite step used). So extra cell =
opposite_cell + offset[2] = anchor + 2×offset[2]. With forward=W (dir 6), offset[2] is the
opposite-of-forward (East), giving extra = (7,5) = anchor+2E — NOT anchor+1E. Finder's binary
reading holds. `walk_anchor_pattern` slot 5 (mod.rs:2004-2010) sets `(anchor.x+1, anchor.y)` =
(6,5), which is identically slot 4 (W.opposite()=E, offset (1,0); verified Direction enum
mod.rs:199-238). `stamp_slots` in bridge_facts.rs:242-246 does it correctly
(`opposite.and_then(step dir 2)` → (7,5); confirmed by test at bridge_facts.rs:327). The two
Rust paths genuinely disagree; `walk_anchor_pattern` is wrong by one cell.

Corrected delta — Rust slot5=(anchor.x+1, anchor.y) (=anchor+1E, dup of slot4)  ->  gamemd
extra=opposite_cell+offset[2] (=anchor+2E). Fix: slot 5 = `(anchor.x + 2, anchor.y)` for
direction W (i.e. opposite cell stepped one more East), matching `stamp_slots`.

NOTE the byte literals of g_DirectionOffsets are zero in this image, so N/E/S/W↔index mapping
is doc-sourced (same caveat the finder logged). The STRUCTURAL relationship (extra = opposite
+ same-index-2 offset = two steps from anchor opposite-to-forward) is byte-independent and
holds regardless of the literal table — that alone proves slot5≠slot4 and slot5 is one cell
beyond the opposite. REAL stands on the index-relationship, not the literal mapping.

---

## D2: walk_anchor_pattern collapses slot4/slot5 to same cell — VERDICT=REAL

Same root cause as D1, distinct framing (the 6-cell span contract, not a single slot). For
dir 6 the binary stamps {anchor, W1, W2, W3, opposite(E1), extra(E2)} = 6 distinct cells
({(5,5),(4,5),(3,5),(2,5),(6,5),(7,5)} for anchor (5,5)) per `decompile_function 0x0047E040`.
`walk_anchor_pattern` produces {(5,5),(4,5),(3,5),(2,5),(6,5),(6,5)} — slot5 collides with
slot4 and the true extra (7,5) is absent from the span. Confirmed against mod.rs:1981-2011.
REAL.

Corrected delta — Rust span cells[5]=cells[4]=(6,5)  ->  gamemd cells[5]=(7,5)=anchor+2E.

---

## D3: legacy anchor_walk_direction rotated 90° (NS→E/EW→S vs N/W) — VERDICT=UNCERTAIN

Binary side HOLDS: `decompile_function 0x005FC570` (OverlayClass__Mark) confirms map-load
bridge stamping dispatches ONLY dir 0 (`0x18`→NESW d0, `0xED`→NWSE d0) and dir 6 (`0x19`→NESW
d6, `0xEE`→NWSE d6) — never dir 2/dir 4. So the live stamp forward is N (NS family) / W (EW
family), and the fact-path Rust (`bridge_stamp_direction_to_direction`, mod.rs:1928-1939:
0→N, 6→W) is correct. The legacy fallback `anchor_walk_direction` (mod.rs:1965-1970: NS→E,
EW→S) IS a 90° rotation — proven as code.

BUT no observable output difference is demonstrated. The legacy branch is gated by
`legacy_anchor = bridge_facts.family == None && bridge_layer.is_anchor_overlay`
(mod.rs:634-639) and `fact_anchor` is checked first (mod.rs:633,640-648). Verified in
resolved_terrain.rs:691-708 that EVERY overlay id 0x18/0x19/0xED/0xEE runs
`stamp_set_bridge_direction`, whose `attach()` sets `family` to Nesw/Nwse on the anchor — so a
real stamped high-bridge anchor always has `family != None`, making `legacy_anchor` false. The
rotation cannot affect a real high bridge at load. Per burden of proof (REAL requires changed
observable output; reachability in any real scenario is unproven), this is UNCERTAIN: latent
code defect, no proven player-visible delta. Downgrade from finder's LIKELY-DRIFT framing —
the rotation is real-as-code but the observable claim is unestablished.

(Forward N-vs-other for dir 0 not byte-confirmable — g_DirectionOffsets zero in image — but
that does not change the verdict: the legacy path is unreachable regardless.)

---

## D4: is_anchor_overlay treats every dir-marked body cell as anchor — VERDICT=REFUTED (as observable disparity)

Binary side HOLDS: `decompile_function 0x0047E040` plate + body confirms bit 0x80 set only on
`param_1` (the anchor) via `uVar9<<7` where `uVar9=param_3&1`; the forward/opposite/extra
cells' flag masks never write bit 7 (they OR `uVar11|uVar12|uVar15|uVar16|param_3|uVar13` with
no `<<7` term). So gamemd marks exactly one 0x80 anchor per span. `is_anchor_overlay`
(mod.rs:1958-1960) returning true for all of 0x18/0x19/0xED/0xEE IS over-broad as a predicate.

REFUTED as an observable disparity because: (a) it is used ONLY inside the `legacy_anchor`
arm, which is gated by `family == None` (mod.rs:634-636) — never true for a stamped
high-bridge cell (resolved_terrain.rs:696-708 always sets family), so it cannot fire on real
high bridges; (b) the LIVE anchor selection uses `is_anchor_self()` = bit 0x80
(bridge_facts.rs:76-78, mod.rs:633), which correctly selects the single anchor and is checked
first; bridge_facts only sets `BRIDGE_FLAG_ANCHOR_SELF` on the Anchor slot (stamp_intact,
bridge_facts.rs:131-137; tested bridge_facts.rs:372-389). No observable output difference in
normal play. Latent trap for future callers only — not a player-visible parity gap.

---

## NEW DISPARITIES THE FINDER MISSED

MISS: D1/D2 collision flips the OPPOSITE cell's ROLE from Tail to Body. Pass-2 role tagging
(mod.rs:668-683) iterates `span.iter_cells()` in slot order 0..5 (iter_cells = enumerate over
cells array, mod.rs:283-288). For a dir-6 fact span, slot 4=(6,5) is tagged `Tail` (slot==4),
then slot 5=(6,5) is tagged `Body` (else-arm) — same cell, LAST write wins, so the opposite
cell ends up `BridgeCellRole::Body` instead of `Tail`. The finder reported the cell-position
collision but not this role-classification side effect. Role feeds the damage-state-machine
match (mod.rs:1317-1326 treats Anchor|Body|Tail identically for RNG, so collapse RNG is
unaffected) and repair iteration — but any role-keyed logic that distinguishes Tail from Body
for the EW-high opposite cell would diverge. Same-low visibility as D1 but a distinct concrete
consequence; surfaced for completeness.

MISS: the TRUE extra cell (7,5)=anchor+2E for dir-6 spans gets NO `anchor_span_id` and NO role
from `walk_anchor_pattern` (it is never in the span's cells[]), yet in `bridge_facts.rs` that
cell carries `BRIDGE_FLAG_EXTRA_SIDE` + anchor-pointer (stamp_intact ExtraDir6,
bridge_facts.rs:171-175; binary writes `iVar10+0x140 &0xfffeffff | uVar16` and the
field_0x2c anchor-pointer at 0x0047E040 dir-6 block). So the fact-layer and the AnchorSpan
layer disagree on which cells (7,5) belongs to: the binary attaches (7,5) to the anchor span
(anchor-pointer at +0x2c), but the Rust AnchorSpan omits it entirely. Consequence: span-driven
iteration (repair `body_cell_repair_state` mod.rs:1300-1306) never visits (7,5). Finder noted
the missing-cell symptom for repair but framed it only as "wrong slot value," not as a
two-layer (facts vs span) attachment inconsistency where the binary's +0x2c anchor-pointer on
(7,5) has no AnchorSpan counterpart.
