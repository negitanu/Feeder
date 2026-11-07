## Design

`ArticleBulkActions` accepts `{ count, busy, onApply, onClear }`. A local select value represents one of four states and maps to `{ read: true }`, `{ read: false }`, `{ starred: true }`, or `{ starred: false }` only when the user presses 実行. The placeholder value maps to no action. The separate clear button calls `onClear` and resets the local choice.

The component guards execution with both the parent `busy` prop and a synchronous ref lock, so repeated clicks cannot dispatch a second callback before React renders. It awaits `onApply`; on fulfillment it resets the choice. A rejection propagates and leaves the choice selected for retry. Since the existing parent handler catches its own errors and resolves, the component treats that as successful completion, as required by its current callback contract.

Use native labelled controls and buttons, `aria-busy` on the toolbar, disabled states while busy, and an accessible selected count. CSS uses the app's `--surface`, `--surface-soft`, `--text`, `--muted`, `--line`, and `--accent` tokens and 8px rounded controls. A wrapping flex layout adapts to narrow screens; no theme-specific hard-coded palette is added.

## Existing Web correspondence

- The current list toolbar in `frontend/src/App.tsx` calls `bulkState` immediately from four buttons and exposes `selectedIds.length`; this component models the same four state patches and count as a reusable explicit-apply control.
- `bulkState` owns API calls, errors, notices, item updates, and the parent `bulkBusy` flag. This component does not duplicate those responsibilities.
- Existing theme controls use the CSS variables in `frontend/src/style.css` and `frontend/src/ui-craft.css`; the new stylesheet follows those tokens and control radius.
- This change records the new Feeder behavior independently; it does not use historical migration snapshots as specification.

## Acceptance contract

- Initial render shows 操作を選択 and a disabled 実行 button; changing the select alone never calls `onApply`.
- Each of the four options dispatches only its corresponding state object when 実行 is activated.
- 実行 and clear are unavailable while `busy`; repeated activation during a pending callback invokes `onApply` once.
- Fulfilled `onApply` resets the choice; rejected `onApply` preserves it for retry.
- Clear invokes `onClear` and resets the choice without applying a state.
- Controls have accessible Japanese names, remain operable with keyboard, and wrap without horizontal overflow on narrow screens in both app themes.

The component is intentionally not mounted by `App.tsx` in this change; browser behavior is specified by this contract and the component's native control semantics.
