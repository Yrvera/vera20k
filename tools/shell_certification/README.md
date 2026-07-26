# Shell certification tooling

This package validates the one-shot production `main-menu-0xe2-steady` capture
bundle and compares four physical presentation crops with the sealed native
guard. It never writes to the guard and never performs desktop input or native
capture.

The Rust artifact is a tight logical 800x600 BGRA8 frame. Guard hashes are not
logical-crop hashes: they cover half-open `presentation_rect` crops in the
1920x1080 DDrawCompat surface. The comparator reconstructs each presentation
crop from the full logical frame using the verified point mapping
`source=floor((2*destination_offset+1)*5/18)` relative to the content rectangle
`[240,0,1680,1080)`, then hashes the resulting tight BGRA8 bytes. It never
compares a direct `logical_rect` crop with a guard digest.

Compare an existing bundle:

```powershell
python -m tools.shell_certification compare `
  --capture C:\path\to\new-run `
  --guard C:\Users\enok\AppData\Local\VERA20k\oracle\guards\sha256\fe\32\fe32f218137b76a91dc3bac07bc96372a61c22e48ea083519f1ecbdbd97d601c.shell-guard.json `
  --output C:\path\to\new-run\comparison.json
```

Run the explicit hidden capture child and compare it:

```powershell
python -m tools.shell_certification capture-and-compare `
  --executable C:\path\to\vera20k.exe `
  --working-directory C:\Users\enok\Documents\ra2-rust-game `
  --guard C:\Users\enok\AppData\Local\VERA20k\oracle\guards\sha256\fe\32\fe32f218137b76a91dc3bac07bc96372a61c22e48ea083519f1ecbdbd97d601c.shell-guard.json `
  --run-dir C:\path\to\brand-new-run-directory
```

Derive the enrolled RGB565 presentation codebooks from all three source frames
named by the sealed guard:

```powershell
python -m tools.shell_certification derive-presentation-profile `
  --guard C:\Users\enok\AppData\Local\VERA20k\oracle\guards\sha256\fe\32\fe32f218137b76a91dc3bac07bc96372a61c22e48ea083519f1ecbdbd97d601c.shell-guard.json `
  --oracle-runs C:\Users\enok\AppData\Local\VERA20k\oracle\runs `
  --output C:\path\to\brand-new-presentation-profile.json
```

Profile derivation validates the guard's sealed SHA-256, accepts exactly three
guarded source frames, and resolves every source only beneath the supplied
Oracle runs root. It rejects traversal, links, non-files, concurrent mutation,
wrong byte length or digest, non-opaque alpha, source-table disagreement,
non-32/64/32 channel cardinality, and different blue/red five-bit tables. The
canonical `vera20k.shell-presentation-profile.v1` JSON is written once and
never replaces an existing path. Neither the guard nor any source run is
modified. Keys and source-derived values are deterministic and sorted; the
documented `generated_at_utc` field is the only intentionally variable value.

The derived tables describe only the enrolled active
retail/AMD/DDrawCompat/DXGI presentation environment sealed by that guard. They
are not a universal `gamemd.exe` RGB565 expansion claim and do not certify
other shell states, resolutions, adapters, display pipelines, or blended
packed-domain behavior. The artifact therefore records
`parity_certification` as `NONE`; it is executable input evidence, not a
completion or parity certificate.

Generate the title-specific RED differential from an existing immutable Rust
capture and all three native source frames named by the guard:

```powershell
python -m tools.shell_certification title-differential `
  --capture C:\path\to\immutable-rust-capture `
  --guard C:\Users\enok\AppData\Local\VERA20k\oracle\guards\sha256\fe\32\fe32f218137b76a91dc3bac07bc96372a61c22e48ea083519f1ecbdbd97d601c.shell-guard.json `
  --oracle-runs C:\Users\enok\AppData\Local\VERA20k\oracle\runs `
  --output C:\path\to\brand-new-title-differential.json
```

The title report validates stable source reads and hashes, collapses the sealed
point-scaled crop only after proving every replica of each logical pixel
agrees, searches every in-bounds glyph-mask translation, and derives the
terminal Path-A tint through native-source RGB565 codebooks. It never
overwrites an output or mutates the guard/run tree. A RED report exits with
status 1 because the current capture is still drift; an invalid evidence input
exits with 2.

The working directory is required because VERA20k loads `config.toml` and
other relative resources from its process working directory. The wrapper
requires an absolute non-link directory and a regular non-link `config.toml`,
passes that directory as the child `cwd`, and records the config path plus
pre/post SHA-256 without serializing its contents. A changed config forces
`INVALID`.

The wrapper uses no shell and, on timeout, kills only the exact child PID it
created. It retains partial diagnostics and refuses to overwrite a run
directory or evidence file.

Exit codes are `0` for `MATCH`, `1` for `DRIFT`, and `2` for `INVALID` or a
tooling error. A region `MATCH` applies only to that named point-scaled
presentation crop. It does not certify the whole frame, movie phase, OS
compositor/display behavior, cursor, route transitions, input, audio, another
dialog, or another resolution.

Focused tests:

```powershell
python -m unittest discover -s tools/shell_certification/tests -p 'test_*.py' -v
```
