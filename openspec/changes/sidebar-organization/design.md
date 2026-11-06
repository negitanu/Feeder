## Context

The current v4 database has no source positions. Library reads sort folders by name/id and feeds by title/id. Existing feed edits, discovery confirmation, OPML import and deletion also change library membership. Reference: `reference/feedastrolabe/specs/sidebar-organization/spec.md`.

## Goals / Non-Goals

**Goals:** Persist arrangement, safe atomic moves, conflict detection, accessible input and safe desktop width restoration.

**Non-Goals:** Desktop app, synchronization protocol, display timezone, other settings export, full legacy database conversion.

## Decisions

- Schema v5 stores explicit positions, using SQLite migration with a v4 consistent backup and strict schema recognition. Preserve old visible name/title/id order. New sources and moved feeds append; folder deletion appends its feeds to unfiled sources. No article/state/visit changes.
- `GET /api/library` continues returning ordered folders and feeds. `POST /api/library/order` accepts `{expected, order}` where each snapshot contains `folders: number[]` and `feeds: {id, folder_id}[]`; each array follows visible order (feeds grouped in folder order then unfiled). Both snapshots cover all sources exactly once. Compare expected with current ordered snapshot within a transaction; stale membership/order returns409. Invalid IDs, duplicate IDs or invalid membership reject the entire change. Existing same-origin protection applies. No global revision is needed because a complete expected snapshot detects concurrent source edits relevant to ordering.
- React delegates source interactions to a sidebar component. Pointer handles drag using pointer capture for mouse/touch and show insertion feedback; outside handles normal scrolling remains available. Keyboard ArrowUp/Down/Home/End reorders within a group; editing/move menu provides cross-folder parity. No optimistic committed state: save then reload authoritative library; errors reload and announce.
- Desktop width defaults250px, range200..480px additionally bounded by available workspace minus360px. Use CSS grid with a separator, pointer capture and keyboard arrows/Home/End. Browser storage is best effort, invalid values fall back; mobile<=760px ignores desktop width.

## Risks / Trade-offs

- Complete snapshots cost O(library size) → bound request body using existing Axum limits; validate before mutation.
- Concurrent changes while sorting →409 and reload, no silent overwrite.
- Pointer cancellation → do not submit until a valid pointer-up; restore normal UI on cancel.
- Unknown database schema → refuse writes; preserve migration backup and document rollback.

## Migration Plan

Run v4 fixture tests and Linux container tests; stop Web/worker before upgrading existing deployment. Startup makes v4 backup before v5 transaction. Rollback stops both services and restores the pre-upgrade backup to a separate volume with previous image.

## Open Questions

None for this bounded migration stage.
