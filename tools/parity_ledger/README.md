# VERA20k parity ledger

The parity ledger is VERA20k's standalone engine-facing evidence ledger. It validates
tracked parity records and their cited evidence; it does not launch retail Yuri's
Revenge, capture runtime state, or upgrade evidence to verified parity by prose.

Run the unit suite and tracked-corpus check with:

```powershell
python -m unittest discover -s tools/parity_ledger/tests -p "test_*.py" -v
python -m tools.parity_ledger check --ci
```

The ledger has no source or documentation dependency on an external game runner.
