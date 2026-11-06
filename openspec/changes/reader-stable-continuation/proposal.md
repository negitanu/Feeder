## Why

The internal reader currently combines offset pagination with client deduplication. Inserting or deleting earlier articles can shift offsets and silently skip matching articles; a failed continuation also lacks a dedicated keyboard retry path. Resolve this before expanding the reader into research and external sources.

## What Changes

- Add bounded, authenticated continuation tokens tied to normalized filters, order, timezone and relevant source membership.
- Use deterministic keyset traversal with an initial article-ID high-water mark, retaining the existing date/visit ordering and NULL handling.
- Keep loaded cards and selection on continuation errors; provide explicit load/retry controls including without IntersectionObserver.
- Reject stale-timezone scoped browser mutations before changing article state.
- Retain the existing internal offset contract for compatibility and leave public v1 ID pagination unchanged.

## Capabilities

### New Capabilities

- `reader-stable-continuation`: Stable scoped continuation and recoverable progressive reading.

### Modified Capabilities

None. Existing requirements remain; this capability adds the missing stronger continuation contract.

## Impact

`backend/src/reader.rs`, shared reader settings, `frontend/src/App.tsx`, browser tests and API documentation. This is an implementation-ready handoff proposal; no runtime change is included in its preparation. Legacy no-script navigation, floating detail and research filters are separate work.
