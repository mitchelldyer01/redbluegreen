# Diagram conventions

## Why, and the frame

Every wiki page carries one diagram. These rules keep the set one
family, readable on a phone, correct in both colour themes. One
diagram shows ONE mechanism and carries a one-sentence
`<figcaption>` with the claim; repeat that claim in `aria-label`.

- `viewBox="0 0 800 H"`. Width is always 800. Pick H for content.
- `role="img"`. No `<script>`, `<style>`, no external image.
- Width and height come from `docs/site.css`. Set neither.

## Colour

No hex in a page file. Colour comes from the variables in
`docs/site.css`, as `fill="var(--ink-ours)"` or `stroke="var(--ink-hw)"`,
so a diagram follows the theme: `--fg`, `--muted`, `--ink-arrow`, and
`--ink-X`/`--fill-X` for `ours`, `none`, `kern` and `hw`.

## The four treatments

| means | shape | fill | stroke |
|---|---|---|---|
| our code, exists | rect, square | `--fill-ours` | `--ink-ours`, 2 |
| not yet | rect, square | `--fill-none` | `--ink-none`, 2, dash `7 5` |
| kernel, GPU ISA | rect, square | `--fill-kern` | `--ink-kern`, 2 |
| hardware, driver | rect, `rx="10"` | `--fill-hw` | `--ink-hw`, 2 |

Dash is `stroke-dasharray`. Label a built or hardware box in `--fg`;
label a not-yet or kernel box in its own ink, for readers without
colour vision.

## Strokes, text, arrows, bytes

Stroke 2 for boxes and arrows, 1.5 for ticks. Text 16 inside a box,
14 for notes and legends, never under 14. Notes in `--muted`. Give
every arrow a verb: `writes`, `polls`, `signals`. Draw a byte range
as a row of rects, width proportional to size; put the hex offset
above the left edge and the size in bytes below it, both centred at
`font-size="14"` in `--muted`.

## Example, copy the marker

```svg
<defs><marker id="arw" viewBox="0 0 10 10" refX="9" refY="5"
  markerWidth="7" markerHeight="7" orient="auto">
  <path d="M0,0 L10,5 L0,10 z" fill="var(--ink-arrow)"/></marker></defs>
<rect x="8" y="20" width="200" height="44" fill="var(--fill-ours)"
  stroke="var(--ink-ours)" stroke-width="2"/>
<text x="108" y="48" fill="var(--fg)" font-size="16">host</text>
<line x1="216" y1="42" x2="330" y2="42" stroke="var(--ink-arrow)"
  stroke-width="2" marker-end="url(#arw)"/>
<text x="273" y="32" fill="var(--muted)" font-size="14">writes</text>
```

## What next

Grep your page for `#` colours before you hand it off. A hex there
is a bug: fix the variable in `docs/site.css` instead.
