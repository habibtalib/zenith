# Size / format variants (`zenith variant`)

Turn **one canonical page into many named target sizes** — square, story, banner, ad slots —
deterministically. This varies **dimensions** (and small per-target tweaks). It is distinct from
`zenith merge`, which varies **content** across CSV rows (see `references/variants.md`). Reach for
`variant` for "the same design at 4 sizes"; reach for `merge` for "this design for 200 people".

For the `variants` block syntax, `override` props, and all command flags, run:

```bash
zenith variant --help
```

## Why it's reliable

- **Token propagation is free.** Variants are overrides *on* the canonical page, so they inherit
  the source tokens — change a brand token once and every size re-renders on-brand.
- **Anchored nodes reflow.** Use `anchor` / `anchor-zone` (see `references/layout.md`) so logos,
  CTAs, and page numbers stay correctly placed at every size; only free-coordinate decorative
  nodes need per-variant repositioning.
- **Deterministic.** Same source → byte-identical `.zen`, PNG, and manifest across runs.

## Workflow

1. Build and `zenith validate` the canonical page first — a broken source fails every variant.
   Variant-specific diagnostics: `variant.duplicate_id`, `variant.unknown_source`,
   `variant.invalid_dimension` (non-px or ≤ 0), `variant.override_unknown_node`.
2. Generate, then open a couple of the PNGs to eyeball reflow at the widest/tallest sizes.
3. For CI, pass `--manifest` and commit it so the batch is auditable and reproducible.

Run `zenith variant --help` for exact flags.
