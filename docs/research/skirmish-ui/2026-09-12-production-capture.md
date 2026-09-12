# Production skirmish capture prerequisite

Scope: one ordinary steady dialog `0x102` frame at 800×600, cursor `(400,300)`.
This supplies diagnostic output for the [all-shell acceptance work](../../plans/2026-09-12-retail-shells-acceptance.md).
It does not establish retail parity or close any shell family.

The checkpoint uses the existing Main Menu → Single Player → Skirmish action
handlers only after an ordinary frame is presented. It does not assign routes
directly, clone shell state, synthesize OS input or render an alternate scene.
The renderer identifies the branch actually drawn. The session waits for
completed skirmish slide/title/game-type/map-label reveals, observes a steady
frame, and captures the next final swapchain frame after the RGB565 presenter.
Never-started reveals are distinguished from completed reveals.

The session rejects a changed surface/cursor, developer shortcut, fallback,
active modal/edit/drag/dropdown/generation, missing chrome or selected preview,
wrong return route, surviving source movie, or changed selection. A separate
manifest and validator preserve the sealed main-menu contracts. Capture startup
continues to suppress physical RA2MD.INI options loading; recorded skirmish
selection reflects the existing production initialization. Asset and profile
enrollment must precede a native comparison.

An initial hidden run exposed another `about_to_wait` callback after successful
capture and an exit request. The shell capture pump now exits immediately when
the session already has an outcome, preventing another surface acquisition and
a misleading post-success render error. This changes only the diagnostic pump.

## Evidence and validation

Final hidden run: normal route actions at frames 35 and 51; skirmish capture
at frame 107, `PcOfDune.MAP`, Battle mode, 1,920,000 BGRA8 bytes. Frame SHA-256:
`fe7d42bfebaca25a608e1be3485a2f2f576beca75867ae4961a73a546c42f8fa`.
The frame was decoded and visually inspected: complete chrome, roster,
preview, settings and buttons. This is a Rust output observation, not a native
comparison. After the pump correction, child PID 29764 exited successfully and
reproduced the first run's exact pixels. The retained log is cumulative: the
first run's error remains at 11:15:17 UTC; the final run starts at 11:18:14 UTC
and records no new error. Palette/Vulkan-layer warnings remain visible in logs.

The existing main-menu steady checkpoint also ran successfully (child PID
14436), passed its original validator and retained the typed manifest field
order. Runtime rejection checks returned failure for existing output, wrong
dimensions, wrong cursor and a developer shortcut; no new rejected bundle was
created. These checks use the final built executable.

Local reproduction artifacts are retained under the owning worktree's `.local/`:
capture manifest/pixels/PNG, child stdout/stderr, run command, executable/config
and input pre/post SHA-256 inventory. The recorded local INI and retail top-level
MIX/INI/CSF/SHP/FNT/PAL/MAP/MPR/SED/IMG files remained unchanged. This inventory
does not claim complete input enrollment. Retail bytes are not committed.

The new guard regression exercises each invalid capture boundary, including
nonfinite cursor position. Reveal regression distinguishes default, completion
and restart. The artifact tests reject changed/truncated payloads, duplicate JSON
keys, path escape, incomplete navigation, malformed selection and parity claims.
The Python certification suite ran 63 tests: 62 passed and one optional
sealed-evidence test was skipped. The full Rust library suite passed 8,688 tests
with 121 ignored and no failures. Clippy completed successfully with 1,147
warnings. Documentation links, the system-map check and diff whitespace checks
passed. Fresh independent read-only criticism found and rechecked the manifest
serialization and strict cursor-type corrections; no implementation findings
remain.

## Native evidence and remaining scope

The diagnostic route invokes existing handlers and adds no gamemd behavior or
new reverse-engineering conclusion. Independent read-only review confirmed no
new Ghidra labels/comments are warranted for this prerequisite; existing native
identities remain with their production owners. Speculative annotations would
not improve evidence.

The separate skirmish schema explicitly reports `parity_certification: NONE` and
unenrolled inputs. No native skirmish reference frame was captured. This work
does not validate physical input, focus, audio, transition timing, any child
dialog, other resolution, persisted selection parity, launch, or return/reentry.
Those remain required by the full-scope acceptance document.
