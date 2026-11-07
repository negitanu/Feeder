## Why

The reader currently exposes four immediate bulk state buttons. A standalone dropdown gives those actions a deliberate selection and execution step while preserving the existing parent-owned bulk mutation flow.

## What Changes

Add a standalone React `ArticleBulkActions` component with a Japanese action selector, separate execute and clear buttons, busy/duplicate-action protection, accessible names, and responsive styling that uses the existing theme tokens. Selection alone never mutates articles.

## Capabilities

### New Capabilities
- `reader-bulk-dropdown`: Explicitly selected bulk article actions.

### Modified Capabilities
- Existing reader bulk state flow is represented by the `bulkState` handler in `frontend/src/App.tsx`; this change does not integrate the standalone component or edit `App.tsx`.

## Impact

Only `frontend/src/ArticleBulkActions.tsx`, `frontend/src/bulk-actions.css`, and this OpenSpec change are in scope. No API, backend, persistence, or parent reader changes.
