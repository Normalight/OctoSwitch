---
name: route-activate
description: Activate a specific OctoSwitch group member, e.g. /route-activate executor sonnet.
allowed-tools: Bash(python scripts/octoswitch_routing.py*), Read
disable-model-invocation: true
argument-hint: <group> <member>
---

# /route-activate

Use this skill when the user wants to set the active member for any OctoSwitch routing group.

## Goal

Provide a direct route-control command for switching the active member of any existing OctoSwitch group.

## Command shape

```text
/route-activate <group> <member>
```

Examples:

```text
/route-activate executor sonnet
/route-activate planner opus
```

## Direct execution behavior

When invoked as a project-local command:

1. Parse arguments as `<group> <member>`.
2. Run:

```bash
python scripts/octoswitch_routing.py activate <group> <member>
```

3. Parse the returned JSON and reply with a short confirmation.

## Output contract

Confirm the exact route that was activated and avoid unnecessary explanation.

Preferred output:

```text
Activated <group>/<member>.
Group '<group>' now uses <member> as its active member.
```

If required arguments are missing, explain the expected syntax instead of guessing.

## Failure handling

If the group is missing:

- report that the group does not exist
- do not suggest a guessed replacement group

If the member is missing:

- report that the member does not exist within that group
- do not silently activate a different member
