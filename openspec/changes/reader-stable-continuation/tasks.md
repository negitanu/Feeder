## 1. Baseline and contracts

- [x] 1.1 Check the dev CI result or record unavailable access with local baseline results; finalize initial cursor-mode/error contracts in the design and API docs before edits.
- [x] 1.2 Add deterministic fixture cases demonstrating current insertion/deletion skips, date ties/NULLs/offsets and history ordering.

## 2. Implementation

- [x] 2.1 Implement canonical scope fingerprints and bounded versioned authenticated tokens with explicit expiry/restart behavior.
- [x] 2.2 Implement consistent first-batch high-water reads and keyset traversal while retaining offset/public v1 compatibility.
- [x] 2.3 Validate optional expected timezone atomically in scoped mutations and submit it from the browser.
- [x] 2.4 Separate continuation error/retry from initial reload, add keyboard load without observer, preserve loaded reader state and reject stale responses.

## 3. Acceptance

- [x] 3.1 Test tampering/expiry/mismatched filters/timezone/source membership, insert/delete traversal, unchanged outside state and old API contracts.
- [x] 3.2 Run Linux browser/regression/restore acceptance including missing observer, failed second batch, explicit retry and mobile/themes.
- [ ] 3.3 Update canonical specs and verification records, run strict validation and commit purpose-specific changes on dev.
