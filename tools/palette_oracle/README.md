# Native palette oracle

This tool executes original x86 instructions from the enrolled `gamemd.exe` in
Unicorn. It neither starts the game nor modifies the executable or Ghidra.
The executable is supplied locally and must match SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
No retail assets or executable bytes are included here; fixtures use a synthetic
palette whose channels each enumerate every byte value.

Install Python 3 and `unicorn==2.1.4`, then, from the repository root:

```powershell
python tools/palette_oracle/oracle.py --exe 'C:/path/to/gamemd.exe' --output .local/palette-oracle
python tools/palette_oracle/check.py --generated .local/palette-oracle
```

`oracle.py` validates 2,001 x87 base scales, complete scalar/CMOV/MMX
LightConvert tables, ordinary Convert tables, 832 SHP/VXL scanline samples and
6,014 CellClass common-brightness finalizations with original Cell stores.
`ground-level.json` additionally records 16 authored Ground/Level values through
all four ordinary/Ion original conversion sites, two native reset/default
roundtrips, and seven post-gather Cell finalizations. CCINI token scanning and
map gathering are explicitly outside these isolated instruction samples.
`fixtures/provenance.json` records native addresses, source hash, active runtime
mode observations, and hashes of the compact fixture subset used by Rust.
The one-row ColorScheme fixture covers AltPalette separately from a player's
53-row scheme.

Cell records supply post-gather RGB/additive/top/bottom values. Original
`004845A2 -> 005558E0` normalizes common brightness, while source additive and
top stay independent. The fixture also runs the original caller pointer setup,
Cell field stores and detail quantizer. It excludes gathering and Convert
allocation; its binary record layout is recorded in the provenance manifest.

Rust regression and real production shader readback checks:

```powershell
cargo test -p vera20k --lib render::palette_light::
cargo test -p vera20k --lib native_palette_tables_match_all_ordinary_shader_pixels -- --ignored --nocapture
cargo test -p vera20k --lib native_shp_loader_atlas_palette_indices_reach_production_pixels -- --ignored --nocapture
```

Follow the repository's compiler-slot and machine-local asset requirements
before running Cargo. The GPU test uses the actual production vertex and fragment
entrypoints for Batch, TMP, both SHP depth modes, and cached VXL. It compares
readbacks with native-generated words expanded by the existing enrolled retail
presentation profile. The second GPU check uses the production SHP decoder and
atlas page-copy helper, with test-owned texture/group creation and actual SHP
shader entrypoints. It does not exercise the complete BatchRenderer upload owner.
These isolated fixtures do not certify a retail scene,
non-clear A-buffer composition, alpha blending, or every palette producer.

See the [native evidence and boundaries](../../docs/research/LIGHTCONVERT_ROW_RGB565_ORACLE_2026_09_09.md).
