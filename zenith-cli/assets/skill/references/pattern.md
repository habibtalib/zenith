# Pattern node — procedural tiling

The `pattern` node tiles **one motif** (a single child node template) across a bounds box,
producing a repeating layout without manual node authoring. It is deterministic: same inputs →
identical bytes out on any machine. Two kinds are supported: `grid` (row-major, evenly-spaced
cells with optional jitter) and `scatter` (pseudo-random positions driven by a seed).

> **v0 caveat — visual props are inert.** The pattern accepts and validates visual properties
> (`fill`, `stroke`, `shadow`, `blur`, etc.) for token-usage tracking (so they don't trip
> `token.unused`), but they **do not paint**: only the motif instances render. The pattern's own
> visual props will become active in a future release.

---

## Attributes

| Attribute  | Type      | Required / default                           | Notes                                                              |
| ---------- | --------- | -------------------------------------------- | ------------------------------------------------------------------ |
| `id`       | string    | **Required**                                 | Stable unique id. Subject to the normal `id.duplicate` check.     |
| `kind`     | string    | **Required**                                 | `"grid"` or `"scatter"`. Any other value → `pattern.unknown_kind`. |
| `x`        | dimension | Optional, default `(px)0`                   | Left edge of the bounds box.                                       |
| `y`        | dimension | Optional, default `(px)0`                   | Top edge of the bounds box.                                        |
| `w`        | dimension | **Required** (must resolve to positive px)  | Width of the bounds box. The bounds box **clips** instances.       |
| `h`        | dimension | **Required** (must resolve to positive px)  | Height of the bounds box. Both `w` and `h` must be positive or nothing renders. |
| `spacing`  | dimension | **Required when `kind="grid"`**             | Cell pitch as a **literal** `(px)N` dimension — **not** a token ref (a `(token)"…"` value is ignored and fires `pattern.grid_missing_spacing`). ≤0 → `pattern.invalid_spacing`. Missing on grid → `pattern.grid_missing_spacing`. |
| `count`    | i64       | **Required when `kind="scatter"`**           | Number of instances. ≤0 → `pattern.invalid_count`. Missing on scatter → `pattern.scatter_missing_count`. |
| `seed`     | i64       | Optional, default `0`                        | Pins jitter/scatter layout deterministically. Change to get a different-but-repeatable arrangement. |
| `jitter`   | f64       | Optional, default `0.0`, range `0.0..=1.0`  | **Grid only.** Positional noise as a fraction of `spacing` per axis (x/y uncorrelated, seed-derived). Out-of-range → WARNING `pattern.jitter_out_of_range` (clamped; still renders). Ignored by scatter. |

Visual/dimension props (`fill`, `stroke`, `shadow`, `filter`, `blur`, `border-*`, `radius`,
`opacity`, `rotate`, etc.) are accepted and validated for token usage but are **inert in v0** —
see the caveat above.

---

## The motif — a template, not a real node

The single child of a `pattern` is a **template**. It defines the shape, geometry, and styling
of each instance; it is **not** collected in the document's id registry and cannot be addressed
directly.

- Each rendered instance is a **clone** with a synthetic id `<pattern-id>.<index>` (0-based):
  `bg.dots.0`, `bg.dots.1`, …
- The motif keeps its authored `x`/`y`; each clone is **translated by the instance offset**
  (the offset is added to the motif's authored x/y, not replaced).
- The motif can be any node kind that carries geometry.

> **Token-usage caveat.** Because the validator treats the motif as an opaque template and does
> not descend into it, a token referenced **only** inside the motif (e.g. the motif's `fill`)
> reads as `token.unused` (advisory). The pattern still renders correctly — the advisory is
> harmless. To silence it, also reference that token elsewhere in the document at node scope.

---

## Layout

### Grid (`kind="grid"`)

Instances are placed row-major at `col * spacing, row * spacing` for every cell whose origin
falls inside the `w × h` bounds. The bounds box clips any instance that overflows.

With `jitter > 0` each instance is displaced by `±jitter * spacing` per axis independently,
using the seed to derive per-instance offsets (x and y are uncorrelated).

### Scatter (`kind="scatter"`)

Exactly `count` instances are placed at seed-derived pseudo-random positions inside the bounds.
`jitter` is ignored.

### Determinism

Both layout functions are fully deterministic. Changing `seed` produces a different-but-stable
arrangement. Default `seed=0` is a valid, reproducible layout. Same document source → same
pixel output on every machine.

---

## Diagnostics

| Code                              | Severity | Trigger                                                       |
| --------------------------------- | -------- | ------------------------------------------------------------- |
| `pattern.unknown_kind`            | Error    | `kind` is not `"grid"` or `"scatter"`.                        |
| `pattern.invalid_spacing`         | Error    | `spacing` resolves to ≤0.                                     |
| `pattern.grid_missing_spacing`    | Error    | `kind="grid"` and no `spacing` attribute.                     |
| `pattern.invalid_count`           | Error    | `count` resolves to ≤0.                                       |
| `pattern.scatter_missing_count`   | Error    | `kind="scatter"` and no `count` attribute.                    |
| `pattern.jitter_out_of_range`     | Warning  | `jitter` is outside `0.0..=1.0` (clamped; output still produced). |

---

## Examples

### Grid — evenly-tiled dots with jitter

```kdl
tokens format="zenith-token-v1" {
  token id="color.dot" type="color" value="#22d3ee55"
}
pattern id="bg.dots" kind="grid" x=(px)0 y=(px)0 w=(px)1080 h=(px)1080 spacing=(px)48 jitter=0.15 seed=7 {
  ellipse id="dot" x=(px)0 y=(px)0 w=(px)8 h=(px)8 fill=(token)"color.dot"
}
```

Tune density by adjusting `spacing` (a `(px)` literal); tune noise by adjusting `jitter`. The
bounds box clips any partially-overlapping instances at the edges.

### Scatter — random confetti

```kdl
tokens format="zenith-token-v1" {
  token id="color.accent" type="color" value="#f59e0b99"
}
pattern id="bg.scatter" kind="scatter" x=(px)0 y=(px)0 w=(px)1080 h=(px)1080 count=25 seed=42 {
  rect id="star" x=(px)0 y=(px)0 w=(px)12 h=(px)12 fill=(token)"color.accent" rotate=(deg)45
}
```

Change `seed` to rotate the arrangement; change `count` to increase/decrease density.

---

## The `detach_pattern` transaction op

`detach_pattern` **materializes** a pattern into an editable native `group`. The pattern is
replaced in place by a group carrying the pattern's `id`, name, role, and bounds, whose children
are the cloned instances (`<id>.0`, `<id>.1`, …), each with its `(x, y)` set to the instance
offset and all other motif props preserved. The detached group renders **byte-identically** to
the live pattern — the same position function drives both.

### JSON op

A transaction file wraps its ops in an `ops` array, so `detach.json` is:

```json
{ "ops": [ { "op": "detach_pattern", "node": "<pattern-id>" } ] }
```

### Workflow

```bash
zenith tx doc.zen detach.json             # dry-run: preview the diff
zenith tx doc.zen detach.json --apply     # materialize the pattern to a group
zenith validate doc.zen --json            # confirm no hard diagnostics
```

After detaching, each instance node is individually editable (`set_fill`, `set_geometry`,
`delete_node`, etc.). The `recipes` provenance block is the right place to record what seed and
params drove the original pattern if you want to reproduce it later.

### Error codes

| Code                              | Trigger                                                     |
| --------------------------------- | ----------------------------------------------------------- |
| `tx.unknown_node`                 | `node` id does not exist in the document.                   |
| `tx.not_a_pattern`                | `node` resolves to a node that is not a `pattern`.          |
| `tx.pattern_unresolved_bounds`    | `w` or `h` is missing or resolves to ≤0 px.                |
| `tx.pattern_not_expandable`       | Layout yields no instances (unknown `kind`, or the required `spacing`/`count` is missing). |
| locked-node rejection             | Pattern or motif is locked — unlock before detaching.       |
