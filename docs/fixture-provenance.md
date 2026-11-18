# Fixture provenance and path redaction

The Java reference fixtures retain their recorded observations and payload checksums.
Some metadata contained private paths, account names, or machine names. This source copy replaces those values with descriptive placeholders.
The [redaction manifest](fixture-redactions.json) identifies each changed metadata file and its original and redacted SHA-256.

Historical provenance records may contain checksums for the original metadata. Those historical checksums do not describe the redacted files.
The XML fixture checker pins the redacted XML provenance file and still verifies the recorded fixture payloads.
No fixture payload was changed for this redaction.
