"""VERA20k System Map v2 tooling.

The package keeps the historical GSI registry, the curated execution topology,
and live repository freshness as separate layers.  None of its outputs are
parity certifications or completion ledgers.
"""

from .model import SCHEMA_VERSION

__all__ = ["SCHEMA_VERSION"]
