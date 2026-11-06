## 1. Baseline and design

- [ ] 1.1 Inspect current Web implementation, API, schema, data and operational contracts without reading production secrets or data into fixtures.
- [ ] 1.2 Establish Feeder canonical specs and requirement-level traceability; explicitly record retired desktop and synchronization scope.
- [ ] 1.3 Evaluate and pin compatible current stable Rust/React dependencies, security posture.

## 2. Vertical slice and migration proof

- [x] 2.1 Implement and test a Rust service and React reader slice with synthetic data.
- [ ] 2.2 Verify generated Feeder schema fixtures, backups, restore and rollback rehearsal; legacy data conversion is out of scope.
- [ ] 2.3 Verify reader navigation, progressive loading, scoped bulk read, themes and accessibility.

## 3. Feature parity

- [ ] 3.1 Migrate subscriptions, collection, sidebar, settings, API authorization and background workers.
- [ ] 3.2 Complete Reader collection recovery, retry and backup operations.
- [ ] 3.3 Migrate public SNS collection into shared reading, search and organization.
- [ ] 3.4 Prove parity per capability with automated tests, PC/mobile and themes; track paid/external acceptance separately.

## 4. Cutover

- [ ] 4.1 Validate all canonical OpenSpec files strictly, dependencies, security, API compatibility and operational guides.
- [ ] 4.2 Rehearse Feeder writer quiescence, Reader data backup/restore and rollback, including writes after an upgrade.
- [ ] 4.3 After acceptance, rebuild/restart the intended deployment and verify health, real HTTP and required external access.
