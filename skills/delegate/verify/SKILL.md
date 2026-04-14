---
name: delegate-verify
description: Verification gate for delegate tasks. Check whether a worker's output actually meets the doneWhen criteria before proceeding.
allowed-tools: ["Read", "Grep", "Glob", "Bash"]
---

# delegate-verify

Use this as a verification gate in the delegate workflow.

This is NOT an execution skill. It verifies that work already done meets the stated completion criteria.

## When to use

After a delegate task worker completes, use this skill to verify the output before:
- Marking the task as completed in TodoWrite
- Proceeding to dependent tasks in the next wave
- Claiming the task is done in the report

## Verification process

### Step 1: Parse the doneWhen criteria

Extract each concrete criterion from the task's `doneWhen` field. Each criterion must be independently verifiable.

### Step 2: Check files changed

For each file the worker reported changing:
1. Read the file to confirm the change exists
2. Verify the change matches the task scope
3. Check for obvious breakage (syntax errors, missing imports, unclosed brackets)

### Step 3: Verify each criterion

For each `doneWhen` criterion:
- **Code change required**: check the diff or read the relevant file sections
- **Tests must pass**: check that tests were actually run and report counts
- **Build must succeed**: verify no compilation errors in output
- **No regressions**: spot-check files the worker didn't touch but that depend on changed code

### Step 4: Classify result

| Classification | Criteria |
|----------------|----------|
| `VERIFIED` | All `doneWhen` criteria are met. Changes are present and correct. |
| `VERIFIED_WITH_CONCERNS` | Criteria met, but quality issues found (see concerns). |
| `NOT_VERIFIED` | One or more criteria are NOT met. Specific gaps listed below. |

### Step 5: Report

```text
## Verification — Task #<id>

Status: VERIFIED | VERIFIED_WITH_CONCERNS | NOT_VERIFIED

### Criteria check
- [x] <criterion 1> — met (evidence: <file:line> or <test output>)
- [x] <criterion 2> — met
- [ ] <criterion 3> — NOT met (gap: <specific description>)

### Concerns (if any)
- <specific concern with file path and description>

### Recommendation
PROCEED | RETRY (with specific feedback) | ESCALATE
```

## Stop-on-failure discipline

If verification returns `NOT_VERIFIED`:
1. Do NOT proceed to dependent tasks
2. Do NOT mark the task as completed
3. Retry the task with specific feedback on which criteria failed
4. After 2 failed retries, mark as failed and escalate to user

## What NOT to verify

- Do NOT re-execute the work yourself
- Do NOT verify things outside the `doneWhen` criteria
- Do NOT check for perfect code quality — only check for correctness and completeness
- Do NOT expand scope beyond what the task specified
