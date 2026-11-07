## ADDED Requirements

### Requirement: Reconcile completed visits after detail dismissal
The Reader SHALL reflect a successful visit in the same retained article list even when its detail panel was dismissed while the response was pending, without reopening that panel.

#### Scenario: Dismiss a pending visit
- **WHEN** a visit commits, the reader closes its pending panel, and its response arrives afterward
- **THEN** the retained row reflects the read state and the detail remains closed

#### Scenario: Scope changed before completion
- **WHEN** a response belongs to an earlier list generation or search scope
- **THEN** it does not change the new list or open the old detail

#### Scenario: A later article wins
- **WHEN** the reader opens another article before the earlier response arrives
- **THEN** the earlier response does not replace the newer detail
