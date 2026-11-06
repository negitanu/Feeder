## 1. Baseline and design

- [ ] 1.1 Inspect current Web implementation, API, schema, data and operational contracts without reading production secrets or data into fixtures.
- [ ] 1.2 Establish Feeder canonical specs and requirement-level traceability; explicitly record retired desktop and synchronization scope.
- [ ] 1.3 Evaluate and pin compatible current stable Rust/React dependencies, security posture and renderer approach.

## 2. Vertical slice and migration proof

- [x] 2.1 Implement and test a Rust service and React reader slice with synthetic data.
- [ ] 2.2 Verify read-only copies and generated schema fixtures; choose DB strategy, implement backups, reconciliation and rollback rehearsal.
- [ ] 2.3 Verify reader navigation, progressive loading, scoped bulk read, themes and accessibility.

## 3. Feature parity

- [ ] 3.1 Migrate subscriptions, collection, sidebar, settings, API authorization and background workers.
- [ ] 3.2 Migrate research, evidence, reports, notifications, recovery and job semantics.
- [ ] 3.3 Migrate SNS, AI providers, assistant, watchlists, entities, intelligence connectors and isolated observation.
- [ ] 3.4 Prove parity per capability with automated tests, PC/mobile and themes; track paid/external acceptance separately.

## 4. Cutover

- [ ] 4.1 Validate all canonical OpenSpec files strictly, dependencies, security, API compatibility and operational guides.
- [ ] 4.2 Rehearse writer quiescence, DB plus blob backup/restore, migration, data reconciliation and rollback, including writes after cutover.
- [ ] 4.3 After acceptance, rebuild/restart the intended deployment and verify health, real HTTP and required external access.
