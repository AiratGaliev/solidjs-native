## Linear issue

<!-- Link the corresponding issue and its milestone parent, if applicable. -->

## Goal and scope

<!-- What problem does this change solve? What is explicitly out of scope? -->

## Implementation

<!-- Summarize the architectural and behavioral changes. Mention changed contracts/ADRs. -->

## Verification

| Command / scenario | Result |
|---|---|
| `pnpm check` | <!-- passed / failed / not run --> |
| `pnpm smoke` (when runtime integration is affected) | <!-- passed / failed / not run --> |
| Platform-specific checks | <!-- target OS/device and result, or not run --> |
| Security-sensitive checks | <!-- boundary/permission/failure tests, or not applicable --> |
| Performance benchmarks | <!-- workload, baseline/candidate SHA and result, or not applicable --> |

**Unrun checks and limitations:**

<!-- State exactly what was not executed and why. Never mark an unrun check as passed. -->

## Acceptance checklist

- [ ] PR target follows the branch policy: topic/dependabot branch → `dev`; `dev` → `test`; `test` or `hotfix/*` → `main`.
- [ ] Scope matches the linked Linear issue and its acceptance criteria.
- [ ] Tests cover the changed behavior, including relevant failure/edge cases.
- [ ] Documentation, ADRs, examples and compatibility notes are updated where required.
- [ ] No unrelated changes, generated build output, secrets or local artifacts are included.
- [ ] Material architecture/API/dependency changes follow the ADR and dependency policies.
- [ ] Security, lifecycle, threading and resource-ownership implications are reviewed where affected.
- [ ] CI results, unrun checks and remaining risks are recorded without overstating acceptance.
