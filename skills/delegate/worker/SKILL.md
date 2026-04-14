---
name: delegate-worker
description: Structured response protocol for delegate task workers. Ensures consistent output format across all subagents.
allowed-tools: []
---

# delegate-worker protocol

Every subagent dispatched via `/delegate` MUST follow this response protocol.

This ensures the controller can parse, verify, and report results consistently.

## Response format

Your final response MUST contain these sections in order:

### 1. Route confirmation

```text
Route: <group>
```

State the routing target you were assigned. Do NOT change this.

### 2. Status classification

Choose EXACTLY ONE:

```text
Status: DONE
```
or
```text
Status: DONE_WITH_CONCERNS
```
or
```text
Status: BLOCKED
```
or
```text
Status: NEEDS_CONTEXT
```

**Rules:**
- `DONE` only if ALL `doneWhen` criteria are actually met
- `DONE_WITH_CONCERNS` if criteria met but you identified risks
- `BLOCKED` only with a concrete blocker description
- `NEEDS_CONTEXT` only with specific information you need

### 3. Summary (3-5 bullets max)

```text
## What I did
- <bullet 1>
- <bullet 2>
- <bullet 3>
```

### 4. Files changed

```text
## Files changed
- path/to/file1.ts
- path/to/file2.rs
- (none, if no files were modified)
```

### 5. Commands run

```text
## Commands run
- npm run build
- cargo test
- (none, if no commands were executed)
```

### 6. Test results

```text
## Test results
- 12 passed, 0 failed
- (no tests run, if applicable)
```

### 7. Unresolved risks

```text
## Unresolved risks
- <risk 1: specific description>
- (none identified, if applicable)
```

**If status is BLOCKED, also include:**

```text
## Blocker
<what is blocking you and why>
```

**If status is NEEDS_CONTEXT, also include:**

```text
## Missing context
<specific information you need to proceed>
```

## Anti-patterns to avoid

- Do NOT return long reasoning dumps or chain-of-thought
- Do NOT change the routing target
- Do NOT re-plan the whole task unless truly blocked
- Do NOT broaden scope beyond what was assigned
- Do NOT claim DONE if you did not verify the `doneWhen` criteria
- Do NOT omit any of the required sections
