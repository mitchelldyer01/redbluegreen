# AGENTS.md — how to work in this repo

## Why

Agents of different sizes work here. This page is the contract they
all follow, so a handoff from one is readable by the next.

## Read first

docs/birdseye.md, then docs/STATUS.md, then the unit prompt you were
given under docs/prompts/. docs/floor.md holds the driver facts.

## Rules

1. No crates. std only. Declare C functions by hand.
2. `unsafe` only around a syscall or a raw pointer the syscall needs.
3. 80 columns everywhere: docs, code, prompts. `rustfmt.toml` sets
   `max_width = 80`, so `cargo fmt` does the wrapping for you.
4. Docs are at most 60 lines. `tools/doclint` checks them.
5. Never hard-code a value the machine can tell you (gpu_id, offsets).
6. Do not commit. Leave the working tree for review.

## Before every handoff, in order

```
cargo fmt --check
cargo clippy --all-targets   # zero warnings
tools/doclint docs/*.md docs/prompts/*.md
```

Then run the unit's verification loop and paste the real output.
Then rewrite docs/STATUS.md: position, verified, stale, next step.
If you changed what the repo can do, update "What exists today" in
docs/birdseye.md.

## When a fact in the docs is wrong

Say so in STATUS.md under "Stale", with what you observed instead.
Do not silently fix the doc and do not silently trust it.
