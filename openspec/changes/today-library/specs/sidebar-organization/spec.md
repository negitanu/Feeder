## ADDED Requirements
### Requirement: Today and folder unread totals
Today SHALL show articles published on the current calendar day in the configured display timezone, with the same scope for listing and bulk actions. Today SHALL show its unread count for the current day in the configured timezone, excluding muted articles consistently with global unread. Each folder SHALL show its unread count under the same mute policy as global unread and refresh after read or membership changes.
#### Scenario: Day boundary and read state
- **WHEN** the calendar day changes or an article is read
- **THEN** Today updates to the new day and the Today and affected folder unread totals update.
### Requirement: Simplified search
The reader SHALL retain detailed search and remove the digest, saved search and shared search interfaces. Existing stored data SHALL be preserved.
