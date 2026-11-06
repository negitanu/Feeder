## Context

Current schema v5 stores ordered sources and collection settings. Article dates use the browser timezone, while date filtering is UTC. The legacy `preferences.py` supports UTC and Asia/Tokyo only; `configuration_export.py` exports explicit fields and omits credentials. The legacy configuration export specification is export-only.

## Goals / Non-Goals

**Goals:** Consistent displayed/calendar dates, persisted defaults, three browser layouts and safe read-only export from the existing collection/settings hub.

**Non-Goals:** Configuration import, arbitrary IANA zones, retired desktop/sync, AI/connector settings not yet migrated, authentication deployment and real-data cutover.

## Decisions

- Schema v6 adds singleton `reader_settings` (display_timezone: UTC or Asia/Tokyo; default_view: all/unread/starred), defaults UTC/all. Strict recognition retains v1..v5 and backs up known old versions before migration.
- GET/PATCH `/api/reader/settings` reads/updates validated shared values. PATCH returns complete settings and checks Origin. Dates in SQL compare julianday against calendar midnight with explicit UTC/+09:00 offset, end exclusive next day; no host timezone dependency or new library. Lists and scoped mutations use the same shared timezone.
- React initially loads shared reader settings before the first list; initial default view applies once without overriding an explicit user action. Saving timezone reloads the active scope and clears selected article IDs. Browser layouts standard/line/tiles change only presentation, preserving loaded cards, filters and selection. Theme/layout/width/disclosure storage is best effort.
- POST `/api/configuration/export` accepts `{presentation:{appearance,width,layout,collapsed_folders}}`. Values are enums, width integer200..480 and at most2000 distinct positive folder IDs. It returns a UTF-8 JSON attachment, no-store, named `feeder-settings.json`; same-origin checks apply. Export format `feeder-settings`, format_version1, app/schema version, timestamp, explicit settings sections collection/reader/folders/subscriptions/presentation and omissions/unsupported_sections. Queries use a single read transaction and serialize explicit allowed fields only. No environment, filesystem or credentials are read; no writes/network work are initiated.
- Export source URLs only when HTTP(S), no userinfo, query or fragment, and free of obvious credential markers in path. Credential-like titles/names are omitted as null. Conservative omissions include field paths and reason only, never the original value. Do not copy etag, modified, errors, checked timestamps, leases, article bodies/states/visits or arbitrary DB columns. Unsupported features are named so the file does not imply full restore support. Omission handling is conservative and cannot recognize arbitrary user-supplied secrets with no distinguishing pattern; documents describe the export scope.
- The existing collection hub receives a display/settings section and an export button; no separate feature page or shared settings import is introduced.

## Risks / Trade-offs

- UTC is the default rather than implicit browser time → intentional alignment with the previous Feeder date-filter default and explicit selection in UI.
- Changes from another browser while a tab remains open → shared settings refresh on window focus; settings saves use explicit complete values. Cross-tab transactional filter versioning is deferred with continuation work.
- Conservative URL omission limits portable subscriptions → show omission count; OPML remains the existing supported subscription interchange. Do not claim export is a backup.
- Browser storage denied → current session still works and export uses current in-memory presentation.

## Migration Plan

Stop Web and worker for deployment; migration creates a consistent v5 backup before v6 transaction. Test fixtures include state/order/history/settings preservation and reopen. Rollback stops both services, restores pre-upgrade backup into a separate volume and runs the prior image.

## Open Questions

None for this bounded stage.
