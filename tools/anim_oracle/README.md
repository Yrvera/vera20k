# Anim boundary oracle

`boundary.py` executes original gamemd.exe instructions at `0042468C..004247B1`
under Unicorn. It records ping-pong direction changes, ordinary loop decrement,
and forward/reverse stage reset. The executable SHA-256 is checked before use.
No project harness, substituted branch result, or Rust model is used.

Install Python dependencies `unicorn>=2.1.1` and `capstone`, set `RA2_DIR` to the
retail directory (or `VERA20K_GAMEMD_EXE` to the executable), then run:

```text
python tools/anim_oracle/boundary.py
```

The checked-in 13,312 rows supply runtime/type fields and the already-committed
stage. They cover stock-shaped normal/damaged bounds and explicit signed/wrapping
fixtures; the latter are not asserted to arise in retail. Entry registers ESI,
EBX and zero EDI match the preceding AI region. Observation stops after native
mutations, before subsequent gameplay consumers. No constructor, timer, complete
AI host, asset loader, or whole-animation equivalence is claimed.

`native_anim_boundary_and_reset_vectors` compares the production Rust boundary
transaction against every row. `combat_anim_shadow_endpoint_retires_in_runtime_and_survives_restore`
uses an explicit ART override through the existing combat-animation producer and
live SimRuntime scheduler, checking retirement, neighbor visitation, and save/load
continuation. The stock NAMISL Building producer and the earlier Anim Middle
callback are separate, unchanged integration work.
