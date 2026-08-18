# Slice 5a-ii Grounding — Lane D: SIDEBTTN.SHP / SIDEBAR.PAL / SDBTNANM.SHP dims

Read-only asset grounding for the in-game Options (0xBBB) owner-draw button right-edge
anchoring. The native child-resize helper anchors the three owner-draw buttons (Back 0x686,
Keyboard 0x52C, Sound 0x52D) off the **SIDEBTTN canvas width**, so 5a-ii needs the exact
on-disk SHP dimensions. All paths absolute. All numbers below are read out of the retail
files via running in-repo code this session (not guessed).

Asset root: `<ra2-install>/`

---

## How the numbers were obtained (load-time mechanism, reproducible)

Wrote a one-shot inspector bin `src/bin/inspect-sidebttn.rs`
(declared in `Cargo.toml` as `[[bin]] name = "inspect-sidebttn"`), then:
`cargo run -p vera20k --bin inspect-sidebttn`. It uses the same path every consumer uses:

1. `AssetManager::new(ra2_dir)` — `src/assets/asset_manager.rs:103`. Loads `ra2.mix`, extracts
   nested archives by brute-force (`extract_all_nested`, `:202`), then `ra2md.mix`,
   `expandmd01.mix` (`YR_EXPANSION_MIXES`, `:61`), and the `OPTIONAL_TOP_LEVEL` list (`:48`).
   Search order = first-match-wins, expansion/`md` archives searched BEFORE `ra2.mix`.
2. `AssetManager::get_with_source(name)` — `:285` — returns `(bytes, "ra2.mix -> sidec01.mix")`
   so the source archive is reported.
3. `ShpFile::from_bytes(&data)` — `src/assets/shp_file.rs:91`. Reads the 8-byte SHP(TS) file
   header: `u16 zero=0`, `u16 width` (offset 2), `u16 height` (offset 4), `u16 frame_count`
   (offset 6) — these are the canvas (max) WxH and frame count. Then each 24-byte frame header
   (`frame_x`@+0, `frame_y`@+2, `frame_width`@+4, `frame_height`@+6, `format`@+8,
   `data_offset`@+20) gives per-frame dims.
4. `Palette::from_bytes(&data)` — `src/assets/pal_file.rs` — 768-byte 6-bit VGA palette → 256
   RGB colors (×4 scaled to 8-bit on read).

The inspector bin is temporary scaffolding for this grounding; safe to delete (it and its
`Cargo.toml` stanza) once 5a-ii is wired.

---

## (1) SIDEBTTN.SHP — the button canvas the anchoring keys off

**Source archive:** `ra2.mix -> sidec01.mix` (sidec01 is nested INSIDE ra2.mix on this install;
there is NO standalone `sidec01.mix` / `sidec01md.mix` file on disk — only `ra2.mix`, `ra2md.mix`,
`expandmd01.mix` exist as top-level mixes). No `md`/expandmd override wins for this file: the
search-order winner resolves to the ra2.mix sidec01 copy, so RA2 base == YR for SIDEBTTN.

**File size:** 9464 bytes.

**Canvas (file-header max) WxH = 125 x 25. frame_count = 3.**

| frame | x | y | w | h | pixels |
|------:|--:|--:|----:|---:|-------:|
| 0 | 0 | 0 | 125 | 25 | 3125 |
| 1 | 0 | 0 | 125 | 25 | 3125 |
| 2 | 0 | 0 | 125 | 25 | 3125 |

All 3 frames are full-canvas 125×25 (uniform; no sub-rect frames). These are the standard
owner-draw push-button states: frame 0 = up/normal, frame 1 = down/pressed, frame 2 =
disabled (conventional SHP button ordering; visual-state confirmation is Lane-B/render scope,
not measured here — only dims were the ask).

**The load-bearing number for 5a-ii: SIDEBTTN canvas width = 125 px** (height 25 px). The
right-edge button anchor (Back/Keyboard/Sound in the 0xBBB template, each 108×23 DLU in the
template per the already-verified Lane-A/template transcription) re-anchors against this
runtime canvas width in the native child-resize helper family.

---

## (2) SIDEBAR.PAL — palette for the button frames

**Source archive:** `ra2.mix -> sidec01.mix` (same nested archive as SIDEBTTN.SHP — they ship
together in the sidebar content mix). **768 bytes**, parses to 256 colors.

- idx0 = (255, 0, 255) — magenta, the conventional transparent/key index.
- idx1 = (243, 255, 255), idx255 = (255, 255, 255).

**Verdict: SIDEBAR.PAL is the correct palette for SIDEBTTN.SHP** — same archive, in-game
sidebar palette, and SIDEBTTN is in-game sidebar chrome. Note idx0 is magenta (the
transparent key); `ShpFile::frame_to_rgba_ui` (`src/assets/shp_file.rs:259`) is the UI path
that forces index-0 → alpha 0 without baking palette alpha, which is the path UI/shell button
art should use so the magenta key doesn't bleed.

**Where it loads in current code:** the AssetManager resolves `sidebar.pal` on demand; it is
already enumerated as a known shell asset in multiple places
(`src/render/skirmish_shell_chrome.rs:988` via `classify_shell_asset("sidebar.pal")`;
`src/bin/mix_browser_data.rs:105`; `tests/sidebar_assets.rs:82`,
`tests/sidebar_chrome_inspect.rs`, `tests/shp_export.rs:145`). For 5a-ii the wiring is:
`assets.get("SIDEBAR.PAL")` → `Palette::from_bytes` → pair with the SIDEBTTN frames via the
shell-chrome atlas loader (`load_named_palette` + `render_shp_entry` pattern in
`skirmish_shell_chrome.rs:184/219`).

---

## (3) SDBTNANM.SHP — already loaded today (contrast)

**Status: ALREADY LOADED and rendered today.** Loaded by the skirmish shell-chrome atlas:
`src/render/skirmish_shell_chrome.rs:184` loads palette `SDBTNANM.PAL`, `:219-237` bakes all
17 frames (indices 0..=16) into atlas entries labelled `sdbtnanm.shp#{frame}`; the atlas
struct exposes `right_panel_button_sdbtnanm_frames: [Option<…>; 17]` (`:44`) plus named
frame-2/3/4/10 fields (`:39-42, :416-421`). Frame-index semantics for the slide-in wave are in
`src/app_shell_transition.rs:24-29` (Group A SDBTNANM 10→5 settle 1; Group B 16→11 settle 0)
and `app_skirmish_shell_render.rs:299` / `chrome.rs:331-373`.

**Source archive:** `ra2.mix -> #0x7B512B17` — a nested mix whose id is NOT in the
`KNOWN_NESTED_MIX_NAMES` guess table (`asset_manager.rs:485`), so it logs as a raw id rather
than a friendly name; content is correct regardless. SDBTNANM.PAL ships in the SAME nested mix.

**File size:** 111800 bytes.

**Canvas (file-header max) WxH = 156 x 42. frame_count = 17.** All 17 frames are full-canvas
156×42 (uniform, x=y=0, 6552 pixels each).

**SDBTNANM.PAL:** 768 bytes, 256 colors, from the same `#0x7B512B17` nested mix. idx0 =
(255, 0, 255) magenta key; idx1 = (255, 247, 247); idx255 = (255, 255, 255). SDBTNANM is
rendered with SDBTNANM.PAL (its own palette), NOT sidebar.pal — confirmed at
`skirmish_shell_chrome.rs:184` and `inspect-pcx-palette.rs:96-100`.

---

## Contrast table (the dimensional facts 5a-ii needs)

| Asset | Source mix | Canvas WxH | Frames | Palette | Loaded today? |
|-------|-----------|-----------:|-------:|---------|---------------|
| SIDEBTTN.SHP | ra2.mix -> sidec01.mix | **125 x 25** | 3 | SIDEBAR.PAL | NO (only referenced in comments — `ui/shell/layout.rs:41`, `descriptor.rs:86`) |
| SDBTNANM.SHP | ra2.mix -> #0x7B512B17 | 156 x 42 | 17 | SDBTNANM.PAL | YES (skirmish shell-chrome atlas) |
| SIDEBAR.PAL | ra2.mix -> sidec01.mix | (768 B, 256 colors) | — | — | enumerated as known shell asset |
| SDBTNANM.PAL | ra2.mix -> #0x7B512B17 | (768 B, 256 colors) | — | — | YES |

**Key contrast:** SIDEBTTN (125×25, 3 frames) is a different, smaller button art than SDBTNANM
(156×42, 17 frames). They are NOT interchangeable. The 0xBBB in-game Options owner-draw buttons
anchor off SIDEBTTN's **125 px** canvas width; SDBTNANM is the front-end shell's slide-in button
chrome and uses its own palette. SIDEBTTN.SHP is currently only named in code comments
(`src/ui/shell/layout.rs:41`, `src/ui/shell/descriptor.rs:86`) noting that 5a-ii is where its
dimensions get wired into the right-edge anchor.

---

## Exactness note

All dims above are read from the actual retail SHP file headers this session via running
in-repo `ShpFile::from_bytes`, not inferred. No running code was needed beyond the one-shot bin;
the load-time mechanism (AssetManager → get_with_source → ShpFile::from_bytes header fields) is
documented in the "How the numbers were obtained" section so any consumer can reproduce.
