# Doc conventions

## Why

Docs here are read by people and by agents with small context windows.
A doc that fits one screen gets read. A doc that does not gets skimmed.
So every doc is short, and a tool enforces the shape.

## The rules

1. 80 columns. No line is longer.
2. 60 lines. No doc is longer. Split it if it grows.
3. Story shape: beginning (why), middle (what), end (what next).
4. Concise. Say it once. Do not explain what the reader can ask.
5. No repeats across docs. Link to the page that owns a fact.
6. Every repo has one bird's-eye doc: `docs/birdseye.md`. It is a
   rollup of the compiled software in the repo and how the parts
   relate. Every task that changes the software updates it.

Reports from agents follow the same rules.

## The tool

`tools/doclint FILE...` checks rules 1 and 2. It needs only sh and
awk. It exits 1 on any failure and names the line or the file.

```
tools/doclint docs/*.md
DOCLINT_LINES=40 tools/doclint README.md   # tighter, per call
```

Rules 3 to 6 are reviewed by a person.

## What next

Run doclint before you hand off a doc. When a doc fails rule 2, find
the second story inside it and give it a page of its own.
