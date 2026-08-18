# Skirmish 0x102 Player-Table Visual Chrome and Spacing Recheck

Date: 2026-07-27  
Target: active Yuri's Revenge `gamemd.exe`  
System: `GSI-03.10`  
Loop: `LOOP-001-SKIRMISH-LAUNCH`  
Status: active-binary recheck complete for the bounded player-table slice  
Confidence: high for control rectangles, combo-face composition, flag composition, and current-Rust comparison; runtime pixel equivalence remains unchecked

## Exhaustive slice declaration

This pass exhaustively covers the visible player-table controls in skirmish
dialog `0x102`:

- the player-name edit;
- the eight flag statics;
- the AI-type, side, color, start, and team collapsed combo faces;
- combo bevels, arrows, swatches, text reserve, disabled presentation, and row
  spacing;
- the distinction between fixed chrome/geometry and the dynamic values shown in
  the supplied screenshots.

This pass does not claim exhaustive coverage of:

- the complete common shell/right-rail composition;
- dropdown popup interaction;
- skirmish setting persistence or gameplay semantics;
- the lower checkbox/trackbar block;
- exact final framebuffer pixels.

Those are separate slices. No Rust production file was changed during this
investigation.

## Player-visible conclusion

The supplied screenshots do not show a broad player-table spacing failure.
After accounting for the original screenshot's 800x600 surface being stretched
to a 1440x1080 center region, the current Rust table is within the expected
logical positions for the original dialog resource.

Most of the immediately visible disagreement is dynamic content/state:

- `Player` versus `[New Player]`;
- explicit or clipped start labels (`1`, `Ra`) versus the original `???`;
- different selected player colors;
- different active/disabled row contents.

Changing the verified control rectangles to compensate for those content
differences would introduce new geometry drift.

One bounded rendering mismatch is verified in current Rust: the collapsed color
swatches are derived from loaded `[Colors]` schemes, while the native skirmish UI
uses a fixed eight-entry packed-RGB table. That should be corrected in a later
implementation slice.

## Active paint path

### Dialog owner

Fresh decompilation of `0x006AE3F0` confirms that the skirmish dialog delegates
common shell handling first and then dispatches dialog-specific work:

- initialization message `0x497` routes to `0x006AE6E0`;
- `WM_PAINT` adds the start-position overlay after common and child control
  painting;
- `WM_COMMAND` routes to `0x006ACEE0`.

The player table is therefore primarily composed by child owner-draw controls,
not by ad hoc painting in the dialog's final `WM_PAINT` arm.

### Combo owner

Fresh decompilation and disassembly of `0x00617250`, including the exact paint
block `0x00617870..0x00617BD0`, confirms the collapsed combo contract:

1. The face is always 24 pixels high.
2. The primitive bevel call uses a two-pixel border.
3. Twenty pixels of the client width are reserved for the arrow/text-fit
   boundary.
4. The arrow origin is `(client_width - 19, 1)`.
5. The text origin is two pixels from the left.
6. The text-fit width is `client_width - 20`.
7. A color swatch is inset two pixels on every side of the non-arrow region.
8. The arrow is selected from released/pressed and normal/grey PCX variants.
9. The disabled/overlay state selects a separate text color.
10. The label is shortened until it fits the reserved width before drawing.

Key fresh instruction evidence:

- `0x0061788D` pushes border `2`, followed by the primitive frame call at
  `0x00617893`;
- `0x006178C1` subtracts `0x14` (20) from the client width;
- `0x006178DC` subtracts `0x13` (19) for arrow X and `0x006178DF` increments Y;
- `0x0061791D` calls the arrow-PCX helper;
- `0x00617A5D` reads the swatch-mode byte;
- `0x00617ABC` supplies the two-pixel swatch inset;
- `0x00617B4E` subtracts 20 for text fitting;
- `0x00617B71..0x00617BAD` performs the fit loop;
- `0x00617BC6` adds two to the text X origin.

Fresh decompilation of the arrow helper `0x00620720` confirms the retail asset
family:

- `dnarrowr.pcx`;
- `dnarrowp.pcx`;
- `gdnarrowr.pcx`;
- `gdnarrowp.pcx`.

### Flag-static owner

Fresh decompilation of `0x006153E0` and a targeted assembly check around
`0x00615861..0x00615911` confirms the kind-2 PCX-static path:

- when the PCX is smaller than the destination, the signed difference is
  divided by two and added to the destination origin;
- when it is larger, the destination extent is clamped and the source is
  cropped;
- there is no fit scaling;
- the final blit is issued through `0x006BA580`;
- the configured transparent color participates in the blit.

The fresh checks agree with the existing flag-static report: native flags use
native size, center only when smaller, crop when larger, and use magenta
transparency.

## Exact logical geometry at 800x600

The original dialog resource is converted with the shell's established
dialog-unit mapping. The relevant current/native logical rectangles are:

| Element | Logical rectangle or rule |
|---|---:|
| Player-name edit | `(58, 59, 151, 23)` |
| Flag row 0 | `(225, 59, 48, 20)` |
| AI combo width | `150` |
| Side combo width | `117` |
| Color combo width | `44` |
| Start combo width | `38` |
| Team combo width | `38` |
| Combo visible height | `24` |
| Row stride | `26` |
| Gap below a combo face | `2` |
| Combo arrow reserve | `20` |
| Arrow origin | `x = width - 19`, `y = 1` |
| Text left inset | `2` |
| Swatch inset | `2` on each side |
| Color swatch fill in a 44px combo | `20x20` |

The supplied original image is 1920x1080, with the game surface occupying the
central 1440x1080 region. This is consistent with a 1.8x stretch of an 800x600
surface. For example, the verified logical side combo `(286,59,117,24)` maps to
approximately `(755,106,211,43)` in that reference capture, which agrees with
the visible control. The supplied Rust capture is approximately the native
800x600 surface. This explains why direct raw-pixel comparison of the two PNGs
is misleading.

## Visual composition ledger

| Layer | Native composition | Current Rust status | Verdict |
|---|---|---|---|
| Player edit bounds | Dialog-resource rect, with verified one-pixel binding adjustment | `layout.rs` produces `(58,59,151,23)` | MATCH |
| Player edit frame | Two-pixel primitive bevel | Current renderer emits a two-ring bevel | MATCH at source-color/geometry level |
| Row positions | Fixed dialog-resource rows, 26px stride | Current layout uses the resource rows | MATCH |
| Combo face size | Fixed 24px visible face | `COMBO_FACE_H = 24` | MATCH |
| Combo bevel | Two-pixel primitive frame | Baked face entries use the verified two-pixel colors/order | MATCH at source-color/geometry level |
| Arrow reserve/origin | 20px reserve; `(w-19,1)` | Current layout uses these exact constants | MATCH |
| Text inset/fit | `x+2`; fit to `w-20` | Current text renderer follows both rules | MATCH |
| Disabled arrow | Grey retail PCX variant | Current renderer selects grey released/pressed entries | MATCH |
| Disabled text | Separate native packed-color path | Current renderer uses the recovered dark-red source color | MATCH at source-color level |
| Flag layout | 48x20 statics | Current layout uses 48x20 | MATCH |
| Flag scaling | Native-size center-or-crop, no scaling | Current renderer uses native clipped/centered helper | MATCH |
| Flag transparency | Magenta color key | Current asset conversion applies the color key | MATCH |
| Color swatches | Fixed native eight-entry RGB table | Current renderer resolves loaded `[Colors]` schemes | DRIFT |
| Collapsed start label | Localized random-symbol string for auto state | Current renderer supports it, but screenshot state is explicit position/clipped fallback | STATE/INITIALIZATION DRIFT, not spacing |
| Player display name | Persisted/default native value | Screenshot shows Rust fallback `Player` | STATE/INITIALIZATION DRIFT, not chrome |

## Current Rust evidence

The current renderer already incorporates the important native chrome rules:

- `src/ui/skirmish_shell/layout.rs` defines the 24px face, 20px reserve,
  two-pixel text/swatch insets, resource-derived row rectangles, and flag
  rectangles.
- `src/app_skirmish_shell_render/controls.rs` composes face, swatch, and arrow in
  the native order and selects grey arrows for disabled sibling controls.
- `src/app_skirmish_shell_render/text.rs` applies the disabled text source color,
  left inset, vertical centering, and pre-truncation.
- `src/app_skirmish_shell_render/chrome.rs` implements the native-size
  center-or-crop flag rule.
- `src/app_skirmish_shell_render.rs` emits flags through that helper for the
  human and active opponent rows.

The verified color-table mismatch is in
`src/app_skirmish_shell_render/controls.rs`: `house_color_tint` uses
`scheme_for_priority` and HSV conversion. The native skirmish UI instead uses
the fixed table already recovered in
`SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`.

## Coverage ledger

| Question | Result |
|---|---|
| Is `0x006AE3F0` the live skirmish dialog path? | Resolved by fresh decompile |
| Are player controls painted by the final dialog `WM_PAINT` arm? | No; child owner-draw paths own them |
| Exact combo face height? | Resolved: 24px |
| Exact combo row stride? | Resolved: 26px from resource positions |
| Exact arrow reserve/origin? | Resolved: 20px and `(w-19,1)` |
| Exact text inset and fit boundary? | Resolved: `+2`, `w-20` |
| Exact swatch inset? | Resolved: 2px |
| Disabled arrow source? | Resolved: grey arrow PCXs |
| Native flag fit behavior? | Resolved: native size, center-or-crop, no scale |
| Does current Rust preserve the verified geometry? | Yes for this bounded slice |
| Does current Rust preserve native swatch source values? | No |
| Are screenshot name/start/color differences caused by spacing? | No; they are dynamic state/content |
| Exact final framebuffer pixels? | Unchecked |

## Open questions and residuals

1. **Display-format conversion residual.** Native packed colors pass through the
   active DirectDraw display format. Rust uses decoded RGB values. The source
   colors and geometry are recovered, but exact final display pixels are not
   certified.
2. **Font rasterization residual.** Ghidra establishes text rectangles and fit
   rules, but final glyph pixels also depend on the retail font asset and the
   Rust raster/upload path.
3. **Initialization residual.** The supplied Rust capture's player name, player
   color, and start labels do not match the supplied original state. Their exact
   initialization/persistence contract needs a separate bounded check before a
   faithful-default implementation.
4. **Runtime capture residual.** A same-state, same-resolution capture is still
   required to judge final pixel placement. The two supplied screenshots use
   different output scaling and different UI state.

No additional chrome/spacing question inside the declared player-table slice
remained after the zero-add recheck.

## Implementation handoff

Recommended next work, in order:

1. Preserve the verified resource rectangles and row stride.
2. Replace the skirmish color-combo swatch source with the verified fixed
   eight-entry native UI table; do not source these face colors from rules INI.
3. Treat the player name, selected colors, random-start symbols, and inactive-row
   values as initialization/state restoration, not layout tuning.
4. Add a same-state 800x600 visual fixture covering:
   - human row with a concrete side and color;
   - one active AI;
   - two inactive rows;
   - random start labels;
   - normal and grey arrows;
   - flags that exercise both native centering and clipping.
5. Have the user compare a new runtime capture against the original. Record any
   remaining one-pixel or palette drift honestly rather than claiming pixel
   parity from source inspection alone.

No broad refactor is needed. The bounded production touchpoints are expected to
remain in the existing skirmish-shell layout/state/render modules.

## Evidence sources

Fresh active-binary reads:

- decompile `0x006AE3F0`;
- decompile/disassemble `0x00617250`, especially
  `0x00617870..0x00617BD0`;
- decompile `0x00620720`;
- decompile `0x006208F0`;
- decompile `0x006153E0`;
- disassemble `0x00615861..0x00615911`;
- xrefs/callee checks for `0x006153E0` and `0x006BA580`.

Existing reports re-read and reconciled:

- `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`;
- `SKIRMISH_0X102_CHILD_CONTROL_RECT_MATRIX_CURRENT_RUST_GHIDRA_REPORT.md`;
- `SKIRMISH_FLAG_STATICS_GHIDRA_REPORT.md`;
- `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`;
- `SKIRMISH_PLAYER_NAME_EDIT_0X6A0_POST_IMPLEMENTATION_AUDIT_GHIDRA_REPORT.md`.

