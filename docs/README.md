# Solidrop Docs Strategy

## Documentation Structure (Where)
PRD: /README.md
Design: /docs/design/architecture.md

Documents with an overview nature / spanning across multiple concerns should be placed in /docs/

Module specifications are placed close to the code.
Examples: 
/infra/terraform/SPEC.md
/crates/api-server/SPEC.md

Progression/handover docs should be placed in /docs/progress/
Examples:
/docs/progress/00-init-stub-report.md
They should be named as <sequential-number>-<summary-in-two-or-three-words>-report.md

Also, plan docs should be placed in /docs/progress/
They should match the naming convention above, have the same sequential-number.

## Documentation Contents (What)
Specification / architecture docs should include decision choice and have a well defined reasons that support the facts. Do not imagine decision reasons / rationales that where unclear in the first place, distinguish between "thought-through" and "tentative" decisions.
Update docs after code review. When the code is merge-ready, all related docs should be updated.
