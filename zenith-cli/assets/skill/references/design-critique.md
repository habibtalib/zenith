# Design critique — judging and improving a rendered `.zen`

`validate` tells you the document is **correct**; it cannot tell you it is **good**. This pack is
the judgment layer: how to read a render and its measurements, decide what's weak, and fix it —
**without enforcing a style.** It pairs with the core loop's step 4 ("actually LOOK") and
`agentic-workflow.md` §3 (self-critique).

## The boundary: facts vs judgment vs style

- **`validate` = is it broken.** Defects only: illegible contrast (`contrast.low`), clipped/overflowing
  text (`text.overflow`). Fix every Error before critiquing aesthetics.
- **`inspect --json` = what are its numbers.** Resolved per-node geometry (`x/y/w/h` in px, with
  `(token)` refs already resolved) plus each node's `role`. These are the raw facts you compute
  balance/alignment/consistency from. The CLI states the numbers; it never says "this is wrong."
- **You = is it good, for this intent.** Aggregates and verdicts live here, not in the tool.
- **Style is an input, never a default.** Flat, minimal, brutalist, maximalist — all can be
  excellent and all can fail. A weak slide is rarely weak _because_ it's flat; it's weak because a
  principle below is violated. **Never reach for gradients/shadows/icons as a reflex "fix" — name
  the violated principle and address that.**

## Order: coarse → fine

Fix in this order; reversing it means polishing things you later move or delete.

1. **Intent & primitives** — is the right primitive used (a labeled box is a `shape`, a comparison
   is not an arrow, data is a `chart`)? Does the medium's range fit the goal?
2. **Composition & balance** — one dominant element; weight distributed, no large dead zones.
3. **Consistency** — same-role elements share treatment across pages.
4. **Noise** — every mark earns its place; remove redundant ink.
5. **Semantic accuracy** — every visual is a claim; does it match the truth?

## The critique pass — questions, not rules

Read each rendered page and ask:

- **Hierarchy** — is there one obvious focal point, or do elements compete?
- **Balance** — does content sit centered/intentional, or drift with a dead band opposite?
- **Alignment** — are edges that _should_ line up actually equal, not "almost"?
- **Rhythm** — are gaps in a series even?
- **Consistency** — does any single element quietly break the system (a missing accent rule, a
  lowercase heading among sentence-case ones, an off-size card)?
- **Noise** — does every shape carry meaning or deliberate depth? What can be deleted?
- **Semantic accuracy** — does the visual metaphor match the relationship? (An arrow asserts
  direction/causation; a comparison wants a neutral divider/"VS", not an arrow. A loop should read
  as a circle, a pipeline as a line.)
- **Legibility** — safe contrast on its actual background (cross-check `contrast.low`).

"Valid but flat/hollow/cramped/half-empty/misaligned" is a fail. Re-render until it passes.

## Turning `inspect` facts into a verdict

`zenith inspect <file> --json` gives resolved geometry + `role` per node. Compute aggregates from it
(the CLI deliberately won't — deciding _which nodes form a set_ is intent, which is your call):

- **Alignment** — group the nodes you consider a set (e.g. a column of cards) and compare their
  `x` (or `y`). Equal = aligned; a 1–3px spread is almost-certainly an accidental misalignment to
  unify. (But an _intentional_ offset — a deliberately "fragile" stack — is fine; you decide.)
- **Spacing/rhythm** — sort a series by `y`, diff consecutive edges; uneven gaps in a meant-to-be-even
  set are a defect.
- **Margins** — per page, take the min/max content edges vs page `w/h`; the left/top margins should
  match your grid and be consistent across pages.
- **Balance** — sum node areas weighted by center to get a rough center-of-mass; far from page
  center with emptiness opposite ⇒ rebalance (recenter, or add an anchoring element).
- **Consistency (role-divergence)** — group nodes by `role` across pages. Every `role="heading"`
  should share font/size/position; if one page's heading differs, or is missing a sibling
  `role="accent-rule"` that its peers have, that's the outlier to fix. This is the cheapest
  high-value check and the one the eye punishes hardest.

Tag nodes with a `role` (`role="heading"`, `role="accent-rule"`, `role="footer"`, `role="card"`)
precisely so these cross-page checks are mechanical rather than eyeballed.

## Worked micro-example

Four cards in a column look "off." `inspect --json` shows their `x` = 980, 982, 980, 981 and `y`
gaps = 124, 124, 124. The gaps are even (rhythm fine); the `x` spread of 2px is the culprit — unify
to 980 (or, if the wobble is a deliberate "fragile handoff," leave it and move on). The tool gave
the numbers; you made the call.

## Report

After a critique pass, say briefly: which principle was weak, which node ids changed, the re-render
path, and that `validate` is clean. The render and the source are the artifacts.
