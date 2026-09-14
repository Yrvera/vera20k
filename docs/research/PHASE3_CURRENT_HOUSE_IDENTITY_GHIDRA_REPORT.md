# Phase 3: current-player House identity

This note covers selection and persistence of the active-retail `gamemd.exe`
current-player House pointer at `0x00A83D4C`. It does not close all House behavior,
discovery callbacks, bridge repair, or the Phase 3 rows.

The causal Phase 3 dependency is cell-list publication (row 42):
`Cell::AddContent 0x0047E8A0` links the object, then its visibility branch can
call object virtual+198 with A83D4C at `0x0047E9DC`. For Infantry that slot is
`Techno::DiscoveredBy 0x006F4960`, whose local-House branch can write discovery
state and dispatch Tag event 4. Bridge-hut evacuation also reaches this PUT
path. Correct saved House identity is therefore a prerequisite to those
callbacks, while their effects remain a separate, open integration obligation.

Binary: `gamemd.exe`, x86 little-endian 32-bit, image base `0x00400000`, SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Evidence is original instruction/byte and caller inspection. Rust validation is
reported separately; this note is not a native executable comparison of an entire
launch or save/load transaction.

## Native selection

| Input or transition | Original evidence | Required behavior |
|---|---|---|
| Scenario preparation | `0x0052D9C4` clears EBX; `0x0052D9DE` stores it at A83D4C | Clear the previous identity before constructing another scenario. |
| Campaign selection | `0x0068ACAA` tests game mode; `0x0068ACB7..CB` calls `0x00528A10` with size `0x14`, Basic/Player and empty default | Use the final 20-byte reader: at most 19 payload bytes. |
| String normalization | `0x00528BDE` terminates the buffer; `0x00727CF0` trims unsigned bytes at most `0x20` | Truncate before trimming. Earlier 32-byte Basic.Player reads used for side/theme selection are not this identity reader. |
| House lookup | `0x0050C170` scans HouseArray A8022C in registration order, comparing bytes from House+15FF4 | Case-sensitive original House spelling; return the first exact match. |
| Missing lookup | `0x0068ACD9..DE` maps result -1 to index 0; `0x0068ACE6..E9` loads the registered House and stores A83D4C | Use the first registered House, not alphabetical order or a default actor. |
| Multiplayer selection | `0x00687F10` retains original participant indices while visiting participants by their +53 ordering; `0x006880F8..00688109` selects saved original index 0 | Bind the original first participant's created House before object/MCV placement. |
| Selected House flags | `0x0068AD0C` and `0x0068AD18` set selected House+1EC/+1ED | Set both represented human/control flags; do not clear flags on other Houses. |

`0x0068810F` also sets the chosen multiplayer House's +1ED. Later MCV placement
failure cannot retroactively undo the earlier A83D4C store. A placement result or
presentation selection is therefore not a substitute for launch identity.

### Participant identity and displayed name

`0x00687F97..0x00687FC8` allocates and constructs a separate House for each
participant before copying its name. The non-mode-4 branch at
`0x0068804A..0x0068808D` copies the ASCII literal `<human player>` from
`0x0083DC8C` into House+15FF4. Independently, `0x00688092..0x006880A4`
copies 21 wide characters from the participant record into House+1602A and
terminates House+16052. Original `0x007CA422` is a word-copy/pad loop.
The selected identity at `0x00688109` is the constructed House pointer,
not a lookup by that displayed name.

The AI path similarly allocates a House at `0x0068817F..0x006881A5`, copies
the ASCII literal `Computer` from `0x00824FD8` into +15FF4, then populates
the separate wide name at `0x00688269..0x00688270`. Neutral and Special
are separately allocated and constructed at `0x006882D1..0x00688303` and
`0x00688325..0x00688356`. These generic native strings are not unique
identities to copy into Rust's keyed House registry.

The admitted Rust offline path previously used the editable player handle as
its case-insensitive House key. A handle of `Neutral` or `Special` was then
overwritten by special-House construction; an active `ComputerN` handle
collided with that AI House. The new current-House reference cannot repair an
already aliased record. The bounded correction belongs in the shared normalized
launch slots: use the existing `Player` owner key for those collisions while
preserving the original display handle and every noncolliding owner key.
`Player` cannot equal the fixed special keys or generated `ComputerN` keys.
House creation, initial forces, starts, alliances, colors and the returned
local owner must all consume that same projection.

Rust's case-insensitive interner does not implement the native byte comparison.
Compare original `HouseDefinition.name` before resolving its interned identity.
The relevant order is INI key order, parsed House roster order, then registered
`session.house_order`; it is not the order of a map keyed by House name.

## Active retail inputs

The asset browser's normal YR lookup resolves both samples below from
`mapsmd03.mix`, with no loose override or shadowed winner. Their extracted bytes
were hashed and inspected on 2026-09-14.

| Map | Size and SHA-256 | Raw identity inputs |
|---|---|---|
| `all04dmd.map` | 282057 bytes; `f1621a21e7221c1e475a52dccd4a04bb3125f8a095377c917e2122019f2157b2` | `[Basic] Player=Americans` at line 260; `[Houses] 0=Americans` at line 319. |
| `sov01umd.map` | 329188 bytes; `498b989a4b6f579dee0904a77f1fabf982c14786db21a46c41900fc8344ec72b` | `[Basic] Player=Russians` at line 370; `[Houses] 0=Americans` and `8=Russians` at lines 464/472. |

The Soviet sample demonstrates a retail selection that is not the first House.
These samples establish real inputs, not an exhaustive map census or a claim that
VERA20k's app currently exposes the native campaign launch route. The behavior
above is from active YR code and data; no TS-only feature is required.

## Native persistence and peer identity

The save caller at `0x0067D421` reaches `0x0067F802..08`, which writes four bytes
from A83D4C through IStream+10. The load caller at `0x0067E8B5` reaches
`0x0067F9F3..F9`, which reads four bytes through IStream+C. At
`0x0067FA04..0E`, the loader submits the address of A83D4C to `0x006CF240`,
using the B0C110 swizzle manager. That helper queues the old pointer and its
replacement location, then clears the destination; an old null pointer needs no
replacement. This is saved scenario identity, not a value to reconstruct from the
outgoing match or the next presentation viewer.

The active checksum path includes Main_Tick `0x0055DE40` to `0x00647260`, then
`0x0064731C` to `0x0064DAB0`; the network path also calls it at `0x00647684`.
The complete `0x0064DAB0` body folds represented object coordinates/facings/class,
House+241 and Scenario Random state. It contains no direct A83D4C identity fold.
This supports persisting the process's current-House reference while excluding
that scalar from a shared peer hash: different participants legitimately select
different current Houses. It does not establish equality between the entire
native CRC and Rust's broader hash, or authorize excluding gameplay consequences
of current-House callbacks.

## Rust ownership and delivery boundary

The intended owner is serialized `ScenarioSession.current_house`. An available
reference must identify both a live House record and its registration in
`house_order`; malformed saved references are rejected. House-less development
substrates may retain `None` as unavailable state. This is not evidence that
native accepts an empty House array.

Bind multiplayer identity immediately after launch-House creation. Bind campaign
identity after the shared map-roster House initialization, before object sections.
Preserve the saved identity through `PreparedLoad` and the first headless runtime
frame. Native identity is independent of renderer/UI state.

The main-based prerequisite introduces no notification/viewer binding API. Any
campaign coverage claim must distinguish the actual map-roster/headless path from
the app's existing skirmish launch orchestration. Consumers such as Jumpjet PUT or
Process discovery need their own native effects and integration; storing this
identity alone does not implement those callbacks.

The collision correction preserves the existing loading-label and score-name
sources rather than adding a second mutable House owner. The separate existing
score-handle lifetime gap after loading handoff remains open: once the caller
loses the handle, the score helper falls back to its existing presentation
source. Arbitrary campaign House names that alias in the case-insensitive
interner are also not certified by this offline collision correction.

Required validation covers exact case, pre-interned spelling, registry order,
19-byte truncation followed by trim, fallback and both human flags; multiplayer
binding before objects and retention after MCV placement failure; valid and invalid
saved references; explicit identity roundtrips with equal shared hashes for two
otherwise identical worlds; and full `PreparedLoad` with a different outgoing
House followed by the first headless frame. Admitted reserved-name collisions
must retain distinct local, AI and special House records, the selected House's
human/control flags, correct placement ownership and saved identity, while
preserving the chosen loading/score display name and color projection.
Full library tests, Clippy and fresh
independent review remain delivery gates. Acceptance results belong to the final
candidate's validation receipt and PR, not an inferred completion claim here.
