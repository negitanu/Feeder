## Design

Handle keydown on the detail panel, not globally. Only unmodified, non-repeating left/right keydown from a non-editable target navigates. Ignore already handled or composing events, interactive controls which use arrow keys, and all keys while loading. At sequence boundaries leave the key untouched. Invoke the existing detail navigation function so visits, error recovery, captured order, and request generation checks remain shared. Announce the shortcut in visible detail help.

Browser tests use synthetic articles and verify both directions, boundaries, modified and repeated keys, input controls, and behavior when focus is outside the detail. Existing detail tests cover failed requests and pending/dismissed visits.
