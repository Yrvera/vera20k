# Loading Post-Marker Text, Mode 5 Content and Layout — Ghidra Report

**Date:** 2026-07-27  
**Program:** active retail Yuri's Revenge `gamemd.exe`  
**Primary function:** `ScenarioClass__DrawLoadingScreen @ 0x00552D60`  
**Investigated slice:** `0x00553687..0x005540AD`, especially `0x00553EC0..0x00554100`  
**Investigation mode:** bounded read-only `/re-swarm` slot  
**Status:** **VERIFIED for mode-5 mechanism, content keys, order, gates, and logical rectangles; UNCHECKED for final pixel parity**

## 1. Verdict

The mode-5/non-campaign loading renderer does not stop after `mmpb.shp` markers.
After `FUN_00640A40 @ 0x00640A40`, it composes four ordered text layers on the
screen-sized hidden loading `BSurface` at manager offset `+0x60`:

1. localized country name;
2. localized country special-unit name, uppercased;
3. localized country `LoadBrief:*` paragraph;
4. localized `GUI:LoadingEx` (`Loading...`).

The first, third, and fourth layers receive a measured black alpha backing and
use the loading-side color scheme. The special-unit layer is black text without
that measured alpha backing. This is separate from the campaign-only
`LSLoadMessage` / scenario briefing fields.

## 2. Active-YR Reachability and Inputs

- `0x00687588 CALL 0x00552D60` is the sole direct caller found by live
  `get_bulk_xrefs(0x00552D60)`. The preceding caller code has already selected
  the active `MPModes` object and launch session.
- `0x00553304..0x0055331D` reads `DAT_00A8DA78[0]`, calls
  `FUN_00696F10`, and stores the result as the country selector used by all
  three country-dependent text tables.
- `FUN_00696F10 @ 0x00696F10` returns session-node `+0x4B` when node
  `+0x4F != -2`, otherwise `-2`. Normal resolved mode-5 launch packing supplies
  a country value `0..9`.
- The observer substitution/suppression checks at `0x005536A0` and
  `0x005539E4` require `g_GameMode == 3 || g_GameMode == 4` plus node
  `+0x6B == -1`. They are not taken for mode 5.
- `DAT_00A8B23C` is the selected `MPModes` object. Its vtable `+0x08`
  is false for Battle, ManBattle, Siege, Unholy, and FreeForAll because those
  vtables point to `0x005C0E40` (`xor al,al; ret`). Cooperative overrides the
  slot with `0x005C4EF0` (`mov al,1; ret`). In this slice that predicate changes
  the high-resolution briefing Y coordinate.

No mode-5 post-marker string depends on a parsed map section or
`ScenarioClass` briefing field. The required data is already available from the
resolved launch session, selected mode, loaded CSF table, loaded game font, and
loaded color schemes before Rust begins scenario parsing.

## 3. Ordered Composition Contract

All rectangles below are logical `(x, y, width, height)` rectangles. For any
screen width `>= 800`, the renderer adds
`dx = (screen_width - 800) / 2` and `dy = (screen_height - 600) / 2` to X and Y.
At exactly 640 pixels wide it uses the 640-specific rectangle. Any other width
below 800 uses the 800-base rectangle without an offset.

| Order / draw | Content | 640x480 base rect | 800x600 base rect | Font / alignment | Color and backing |
|---|---|---|---|---|---|
| 1 / `0x005539DF -> 0x00621040` | `Name:*` country | `(385,436,200,20)` | `(540,310,200,20)` | `g_GAME_FNT`; right (`flags & 2`) | loading-side color; measured text bounds expanded 4 px, black alpha `0x9F` |
| 2 / `0x00553D01 -> 0x00621040` | uppercased country special unit | `(16,72,200,20)` | `(20,90,200,20)` | `g_GAME_FNT`; left (`flags = 0`) | black RGB bytes `0,0,0`; no measured alpha backing in this layer |
| 3 / `0x00554022 -> 0x00554280` | wrapped `LoadBrief:*` | `(16,126,318,104)` | ordinary mode `(20,158,398,130)`; Cooperative `(20,380,398,130)` | `g_GAME_FNT`; left; width-wrapped and height-clipped | loading-side color; measured wrapped bounds expanded 4 px, black alpha `0x9F` |
| 4 / `0x005540A8 -> 0x00554280` | `GUI:LoadingEx` | `(16,235,200,20)` | `(20,300,200,20)` | `g_GAME_FNT`; left | loading-side color; measured text bounds expanded 4 px, black alpha `0x9F` |

`FUN_00554100` performs the centered-800x600 viewport offset.
`FUN_00554150` constructs the final `Loading...` rectangle.
`FUN_005541C0` measures and aligns that string's alpha-backing rectangle.
`FUN_00554280` draws through `FUN_00621040` using `g_GAME_FNT`.
`BitFont__MeasureText @ 0x00433CF0` performs width-aware line measurement;
the low renderer `0x00434CD0` implements bit `1` as centered and bit `2` as
right-aligned text.

The alpha helper `0x00621B80` locks the same hidden `BSurface`, blends color
zero with opacity argument `0x9F`, and unlocks it. `0x00552D60` itself performs
composition; this report does not rename the later hidden-to-primary transfer
as native `Present` or `Flip`.

## 4. Exact Country and Special-Unit Content

The key pointers and dispatch values were verified in live disassembly
`0x005536A0..0x00553D06`. Literal English values were extracted read-only from
the valid CSF beginning at byte offset `39346` in retail `langmd.mix`
(`" FSC"`, version 3, 5211 labels), using the CSF one's-complement UTF-16
encoding.

| Country value | Country key → English | Special-unit key → English after native uppercase pass |
|---:|---|---|
| 0 | `Name:Americans` → `America` | `Name:Para` → `PARADROP` |
| 1 | `Name:Alliance` → `Korea` | `Name:BEAGLE` → `BLACK EAGLE` |
| 2 | `Name:French` → `France` | `Name:GTGCAN` → `GRAND CANNON` |
| 3 | `Name:Germans` → `Germany` | `Name:TNKD` → `TANK DESTROYER` |
| 4 | `Name:British` → `Great Britain` | `Name:SNIPE` → `SNIPER` |
| 5 | `Name:Africans` → `Libya` | `Name:DTRUCK` → `DEMOLITION TRUCK` |
| 6 | `Name:Arabs` → `Iraq` | `Name:DESO` → `DESOLATOR` |
| 7 | `Name:Confederation` → `Cuba` | `Name:TERROR` → `TERRORIST` |
| 8 | `Name:Russians` → `Russia` | `Name:TTNK` → `TESLA TANK` |
| 9 | `Name:YuriCountry` → `Yuri` | `Name:YURI` → `YURI` |

The uppercase loop at `0x00553B4C..0x00553B86` subtracts `0x20` from UTF-16
code units in `a..z` and `0xE0..0xFE`. It applies only to the special-unit
layer, not to the country name or paragraph.

## 5. Exact LoadBrief Content

| Country value | CSF key | Retail English value |
|---:|---|---|
| 0 | `LoadBrief:USA` | The USA has the best paratroopers in the world. Build an Airforce Command Center to drop paratroopers anywhere on the battlefield. |
| 1 | `LoadBrief:Korea` | The Black Eagles are the most dangerous fighter pilots in the world. Korean forces are always well protected by these deadly air men and their lethal fighter-bombers. |
| 2 | `LoadBrief:French` | The French Grand Cannon is the ultimate defensive gun, firing at long range for massive damage. |
| 3 | `LoadBrief:Germans` | The German Tank Destroyer can easily eliminate enemy vehicles. Its advanced armor-piercing gun is weak against enemy infantry and structures. |
| 4 | `LoadBrief:British` | The British Sniper can easily eliminate enemy infantry at great ranges. |
| 5 | `LoadBrief:Lybia` | The Libyan Demolition Truck self-destructs on an enemy target, setting off a small nuclear bomb. |
| 6 | `LoadBrief:Iraq` | The Iraqi Desolator can poison land with toxic radiation or annihilate enemy troops with his powerful Rad-Cannon. |
| 7 | `LoadBrief:Cuba` | The Cuban terrorist is a fanatic for the Soviet cause and will actually carry a bomb right up to the enemy before detonating it, destroying himself and anything nearby. |
| 8 | `LoadBrief:Russia` | Russian Tesla Tanks fire a short range Tesla Bolt that can short circuit enemy vehicles and even arc over enemy walls. |
| 9 | `LoadBrief:YuriCountry` | Yuri uses mind-control and genetic mutation to corrupt the battlefield. His special units can seize the forces and structures of his enemies. |

`LoadBrief:Lybia` is the exact retail key spelling. Do not silently correct the
key to `Libya`.

At `0x00553E54..0x00553F3C`, ordinary selected modes take the vtable-`+0x08`
false branch and use Y `158` at the 800 base. Cooperative alone takes the true
branch and uses Y `380` at the 800 base. Both use Y `126` in the 640 layout.

## 6. Loading-Side Text Color

Before the marker helper, `0x00553603 CALL 0x00642B60` selects a color scheme
and `0x00553611 CALL 0x00517440` converts the scheme HSV triple at `+0x308`.
For the non-observer branch, `0x00642B60` selects:

- `AlliedLoad` when its owning load-screen side field `+0x80` is zero;
- `SovietLoad` when that field is nonzero.

Retail INI values are `AlliedLoad=164,255,255` and
`SovietLoad=0,235,255` in HSV. Before text draw the result is round-tripped
through the active DirectDraw pixel format, so the scheme identity is the
portable exact contract; a universal post-quantization 24-bit RGB claim would
require pinning the runtime pixel format. Under RGB565 the visible unpacked
triples are approximately `(0,36,248)` and `(248,20,16)`.

The special-unit layer instead reads the three-byte global initialized at
`0x004E8120..0x004E812C`; all three bytes are zero and no other writes were
found, so its text color is black.

## 7. Current Rust Mapping and Implementation Handoff

Current Rust already has every pre-map semantic input:

- `SkirmishLaunchSession.local.country` is resolved before loading;
- `SkirmishLaunchSession.mode` identifies Cooperative versus ordinary modes;
- `AppState.csf` and `AppState.bit_font` are startup-owned;
- `LoadingSession::from_request` already selects the country loading-art variant.

`build_native_loading_instances` currently emits background, backing, progress
bar, and side icon only. Its comment that the progress-row label is absent must
not be generalized into “the native loading renderer has no text”; the four
post-marker text layers above are a separate renderer responsibility.

Bounded implementation handoff:

1. Add a text-composition layer after marker/chrome composition and before each
   synchronous loading repaint, sourcing strings from CSF keys rather than
   hardcoded English.
2. Carry selected-mode Cooperative identity and resolved local country into the
   loading presentation state; no scenario parse dependency is needed.
3. Add fixtures for America/Battle at 800x600, Russia/Battle at 640x480, and a
   Cooperative 800x600 briefing-Y switch. Verify order, keys, uppercase result,
   rectangles, wrapping/clipping, and black `0x9F` backings separately from
   any pixel-parity claim.

## 8. Negative Findings / Do Not Do

- Do not use `LSLoadMessage`, `LSLoadBriefing`, or map `[Briefing]` as the source
  for these mode-5 strings.
- Do not suppress these four layers because the progress-row label pointer is
  null.
- Do not uppercase the country name or briefing paragraph.
- Do not use the Cooperative Y `380` layout for Battle/ManBattle or the other
  ordinary selected modes.
- Do not claim native/Rust glyph, blend, or final-pixel parity without a
  retail-derived capture or exhaustive renderer proof.

## 9. Remaining Uncertainty

- Final glyph rasterization, kerning, DirectDraw pixel-format quantization, and
  alpha-blend pixels were not compared against a runtime capture.
- The wrapper passes fixed trailing arguments including `0x0C`; their semantic
  names are not required for the verified alignment/wrapping contract and were
  not relabeled speculatively.
- Invalid or still-random country `-2` falls outside the normal resolved
  mode-5 launch path; the binary dispatch produces no country-specific string
  for values outside `0..9`.

## 10. Stale-Document Replacement Wording

For `LSLOADMESSAGE_SKIRMISH_LOADING_TEXT_SPLIT_GHIDRA_REPORT.md`, replace any
statement equivalent to “standard Skirmish has no map/status/briefing text”
with:

> `LSLoadMessage` and the scenario mission-briefing fields are campaign-only in
> `0x00552D60`, but mode 5 has a separate post-marker text pipeline: localized
> country name, uppercased country special-unit name, localized
> `LoadBrief:<country>`, and `GUI:LoadingEx`.

For `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`, replace any unresolved
“post-marker text helper/content unknown” wording with the ordered four-layer
contract and rectangle table in sections 3–6 of this report.

## 11. Evidence Commands Used

- live Ghidra `decompile_function(0x00552D60, program="gamemd.exe")`;
- live Ghidra `disassemble_bytes` over
  `0x00553304..0x005540AD`, `0x00554100..0x00554280`,
  `0x005C0E30..0x005C0E60`, and `0x005C4EE0..0x005C4F20`;
- live Ghidra `batch_decompile` for `0x00554100`, `0x00554150`,
  `0x005541C0`, `0x00554280`, `0x00621040`, `0x00621B80`,
  `0x00433CF0`, `0x00434CD0`, `0x00517440`, `0x00642B60`,
  and `0x00696F10`;
- live Ghidra `inspect_memory_content` for the CSF key pointers, selected-mode
  vtables, and renderer key `0x0082686C` (`GUI:LoadingEx`);
- read-only retail `langmd.mix` CSF parse and repo stock INI lookup.
