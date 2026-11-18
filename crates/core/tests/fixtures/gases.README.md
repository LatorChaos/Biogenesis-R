# Resource boundary observations

`gases.tsv` projects existing, immutable Java observations from `tests/fixtures/java/core.jsonl`.
The original source/source/JAR recipe and hashes remain in `tests/fixtures/java/manifest.json` and `docs/reference_oracles.md`.
This projection performs no new Java execution and changes no original fixture.

It retains all 21 `world_gas_methods` transitions and the separate negative-request observation.
Each row contains nine tab-separated fields:

```text
operation request before_O2 before_CO2 before_CH4 returned after_O2 after_CO2 after_CH4
```

Numbers are sixteen hexadecimal digits containing exact binary64 bits.
Void removal methods use `~` for the returned field.
Java hexadecimal floats convert through Python `float.fromhex`, then big-endian binary64 encoding.
Integer inputs convert exactly to binary64.
The provenance file records the input hash, selected scenarios, conversion, output hash, and executed Python version.

Rust uses each recorded before-state as an explicit input to the actual World resource method.
All returned values and gas quantities compare by exact bits.
Zero, partial, exhausted, and negative requests preserve the original behavior.
This fixture does not verify organism metabolism scheduling, full frames, or population statistics.
