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
7. Never create a GPU queue from inside a pi session: do not run
   `rbg queue` or any command that calls CREATE_QUEUE. The model
   server shares this GPU. A bad queue resets the GPU, kills the
   server, and ends your own session. Build, clippy, `rbg probe`
   and `rbg mem` are fine. Write `ready for GPU test` in STATUS
   and stop; a person runs the queue loop with the server down.
8. Context is the scarce resource. Read files by line range, pipe
   build and test output through `tail -20`, and never paste a
   file back into the conversation.

## Before every handoff, in order

```
cargo fmt --check
cargo clippy --all-targets   # zero warnings
tools/doclint docs/*.md docs/prompts/*.md
```

Then run the unit's verification loop and paste the real output.
Then rewrite docs/STATUS.md: position, verified, stale, next step.
The first line under "## Position" starts with exactly one of
`Unit NNN: done`, `Unit NNN: in progress`, `Unit NNN: ready for
GPU test`, or `Unit NNN: blocked`, so tools/drive can read it.
Use "ready for GPU test" when only rule 7 steps remain, and
"blocked" when you need a decision; say what, in the same line.
If you changed what the repo can do, update "What exists today" in
docs/birdseye.md.

## When a fact in the docs is wrong

Say so in STATUS.md under "Stale", with what you observed instead.
Do not silently fix the doc and do not silently trust it.
