---
description: Type-check every target, tests included (runs none)
allowed-tools: Bash(cargo xtask:*)
---

Type-check every target, tests included, without running any.

## Usage

`/check` -- check if code compiles

## Output

**Success:** `Check OK`
**Failure:** shows compilation errors (first 10)

## When to use

After making code changes, before running tests --
catches syntax and type errors in production and test code,
in seconds on a warm build.

## Implementation

```
cargo xtask check
```
