<div align="center">

# Zenith

<h3>A design-document format and engine built for the age of AI agents.</h3>

<p>
Plain-text <strong>.zen</strong> design files that you can read, diff, review, validate, and let an agent safely edit — compiled <strong>deterministically</strong> to pixel-exact PNG and print-ready PDF.
</p>

<p>
  <a href="#quick-start"><strong>Quick start</strong></a> 
  <a href="#what-it-does"><strong>Features</strong></a> 
  <a href="#how-it-works"><strong>How it works</strong></a>
  <a href="#command-surface"><strong>Commands</strong></a>
  <a href="#showcase"><strong>Showcase</strong></a>
</p>

<p>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="License: Apache-2.0"></a>
  <a href="https://doc.rust-lang.org/edition-guide/rust-2024/index.html"><img src="https://img.shields.io/badge/edition-2024-purple" alt="Edition 2024"></a>
  <img src="https://img.shields.io/badge/unsafe-forbidden-green" alt="Unsafe: forbidden">
  <img src="https://img.shields.io/badge/deps-C--free-green" alt="C-free dependencies">
  <a href="#status"><img src="https://img.shields.io/badge/status-pre--0.1-yellow" alt="Status: pre-0.1"></a>
</p>

</div>

---

Zenith is a plain-text format and engine for design files — posters, decks, books, social graphics, diagrams, and more. The idea is simple: **design should work the way code does.** You should be able to read it, diff it, review it, test it, and let an agent safely edit it.

A `.zen` file is human-readable [KDL](https://kdl.dev) text. The engine parses it, validates it against a large diagnostic set, compiles it to a backend-neutral scene, and renders the same file to the **same pixels every time** — as a PNG or a print-ready PDF.

## Why

Code got source control, types, tests, and pull requests. Design files got none of that. They're opaque blobs — you can't diff them, you can't review a change, and the same file can render differently on different machines.

That's a problem for people, and it's a bigger problem for AI. Agents can already write code and open pull requests, because code is text they can read and reason about. Drop them into a design tool and they go blind. Ask an agent to "make the heading brand red and tighten the layout" and there's nothing safe to grab onto — no stable target, no validation, no preview, no way to check the result.

Zenith fixes that. The goal is to make design files as safe to automate as code:

- **Plain text** you own — readable, diffable, yours forever.
- **Stable IDs** so every change is a reviewable patch, not a mouse drag.
- **Deterministic rendering** — the same file always produces the same pixels.
- **Real validation** — text fits, colors come from the design system, nothing falls off the page.
- **Safe edits** — every change is a typed transaction, checked and previewable before it lands, with a source diff and an audit record.

## It's not AI image generation

This is the most common mix-up, so it's worth being blunt: Zenith is the opposite of an image model like Nano Banana, ChatGPT image, or Grok Imagine.

An image generator gives you a flat picture. It's a bag of pixels — you can't open it up and move the logo, you can't force the headline to use your exact brand color, and asking for "the same thing but with a different date" gives you a different image. There's nothing to edit, review, or guarantee.

Zenith doesn't generate a picture. It generates the _design itself_ — a structured, editable document where every element is real and addressable. An agent (or a person) can change one line, swap a color token, or regenerate a hundred on-brand variants, and every render is exact and repeatable. AI writes and edits the source; Zenith guarantees what it means and how it looks.

## Agent-native first, not a tool with an API bolted on

Most design tools are built for a human dragging boxes, and an automation API gets added later as an afterthought — a thin, limited layer over a model that was never meant to be driven by software.

Zenith is built the other way around. The foundation is a programmatic, text-based, deterministic engine. Agents, scripts, and the command line drive it directly. A visual editor for humans is a client on top of that same engine — not the other way around. So automation isn't a side door; it's the front door.

People are still first-class users. "Agent-first" means the safe, scriptable core comes first, and everything else is built on it.

## How it works

A `.zen` document flows through a single deterministic pipeline. Each stage is a separate crate with a clean contract boundary, so a future GPU backend, SVG export, or visual editor consumes the same scene IR:

```text
  design.zen  (KDL plain text)
       │  parse + validate           zenith-core   →  diagnostics (Error/Warning/Advisory)
       ▼
  Document AST  ──transaction ops──▶  Document AST'  zenith-tx  →  dry-run / apply + audit record
       │  compile                    zenith-scene + zenith-layout
       │    · resolve tokens, geometry, anchors
       │    · shape text (rustybuzz), wrap, hyphenate
       ▼
  Scene IR  (backend-neutral display list)
       │  render                     zenith-render
       ├─▶ PNG   (tiny-skia, byte-identical)
       └─▶ PDF   (vector, native CMYK, bleed / trim / crop)

  local history / undo / versions    zenith-session   (off the render path; never affects pixels)
```

Everything that touches the render path is **deterministic and C-free**: no time, no randomness, no `HashMap`, no `unsafe`, no C dependencies. The same bytes in always produce the same bytes out, on any machine.

## What it does

- **Plain-text `.zen` format** — KDL v2 source with `project` / `tokens` / `styles` / `document` / `page` structure; every node carries a stable id.
- **Design tokens** — `color` (sRGB **and** native CMYK), `dimension`, `number`, `fontFamily`, `fontWeight`, `gradient` (linear/radial), `shadow`, `filter`, and `mask`, with alias chains and cycle detection.
- **A full node set** — `rect`, `ellipse`, `line`, `polygon`, `polyline`, `text`, `code`, `image`, `frame`, `group`, `shape`, `connector`, `instance`, `field`, `footnote`, `toc`, and `table`, plus lossless pass-through of unknown nodes for forward compatibility.
- **Real typography** — `rustybuzz` shaping with bundled Noto Sans / Noto Sans Mono, font fallback, Knuth–Liang hyphenation, rich inline spans, threaded text flow (chains), drop-caps, tab-leaders, text runaround, and a `font.glyph_missing` diagnostic when a glyph is unavailable.
- **Visual effects** — linear & radial gradients, layered shadows, Gaussian blur, feathered masks, 12 Porter-Duff blend modes, opacity cascade, per-corner radius, and image fit / clip-shape / object-position.
- **Anchors** — page-relative 9-point placement and safe-zone-relative anchors that materialize to explicit, deterministic geometry (absent anchor = byte-identical to hand-placed coordinates).
- **Transaction engine** — a typed op set (set fill/stroke/geometry, add/remove/reparent/group, align/distribute, page ops, token ops, find-replace, and more) applied as **dry-run by default**, with referential-integrity and id-uniqueness enforcement, a source diff, a scene diff, affected node ids, and an audit record.
- **Deterministic rendering** — pixel-exact PNG via tiny-skia and print-ready PDF (native DeviceRGB/CMYK, MediaBox/TrimBox/BleedBox, no embedded timestamps), single page, all pages, or facing-page spreads. The scene IR can be dumped to JSON.
- **Local history** — per-document identity (ULID doc-id stamped in the file, ignored by the renderer), an ephemeral session DAG for undo/redo, and a durable content-addressed version store (SHA-256 + DEFLATE) with named versions and restore — entirely off the render path.
- **Library subsystem** — embedded preset packs (`@zenith/flowchart`, `@zenith/filters`, `@zenith/masks`, `@zenith/brand-kit`); `library add` materializes an item into a self-contained document with `libraries` + `provenance` tracking.
- **Variable-data merge** — `role="data.<column>"` bindings drive CSV mail-merge across text and image columns and multi-page templates, with a per-row JSON report and a byte-reproducible manifest.

## Workspace

Zenith is a Rust workspace. Each crate owns one concern and exposes a stable contract; `zenith-core` depends on nothing else in the tree.

| Crate            | Responsibility                                                                            |
| ---------------- | ----------------------------------------------------------------------------------------- |
| `zenith-core`    | KDL parser adapter, semantic AST, canonical formatter, tokens, validation, diagnostics    |
| `zenith-layout`  | Text shaping & font metrics (`rustybuzz` + `ttf-parser`); third-party types confined here |
| `zenith-scene`   | Backend-neutral scene IR + compilation (geometry, text wrap, anchors, opacity/clip)       |
| `zenith-render`  | CPU PNG backend (tiny-skia) and vector PDF backend; determinism enforcement               |
| `zenith-tx`      | Transaction op set, apply/dry-run engine, diffs, and the audit-record contract            |
| `zenith-session` | Local-machine doc identity, session DAG, durable versions (content-addressed store)       |
| `zenith-cli`     | `zenith` command-line tool — dispatch, argument parsing, and JSON/human output            |

## Install

Zenith is pre-release and not yet published to crates.io. Build from source:

```bash
git clone --recurse-submodules https://github.com/farhan-syah/zenith
cd zenith
cargo build --release            # builds the workspace, including the `zenith` CLI
```

The release binary lands at `target/release/zenith`. No C toolchain or system libraries are required — the dependency graph is C-free and `unsafe` is forbidden workspace-wide.

## Quick start

```bash
zenith validate examples/hello.zen          # report diagnostics (add --json for machine output)
zenith fmt examples/hello.zen               # canonical, idempotent formatting
zenith tokens examples/hello.zen            # list design tokens and their resolved values
zenith inspect examples/hello.zen           # print the node tree (read-only)
zenith render examples/hello.zen --out .    # compile + render to PNG

zenith render examples/multipage.zen --all-pages out/     # one PNG per page
zenith render examples/hello.zen --pdf hello.pdf          # print-ready PDF
zenith render examples/hello.zen --scene scene.json       # dump the scene IR
```

The smallest valid document:

```kdl
zenith version=1 {
  project id="proj.hello" name="Hello Zenith"
  tokens format="zenith-token-v1" {
    token id="color.bg" type="color" value="#f8fafc"
    token id="color.ink" type="color" value="#111827"
    token id="font.body" type="fontFamily" value="Noto Sans"
    token id="size.heading" type="dimension" value=(px)42
  }
  styles {}
  document id="doc.hello" title="Hello Zenith" {
    page id="page.hello" w=(px)480 h=(px)160 {
      rect id="rect.bg" x=(px)0 y=(px)0 w=(px)480 h=(px)160 fill=(token)"color.bg"
      text id="text.hello" x=(px)24 y=(px)24 w=(px)432 h=(px)112 fill=(token)"color.ink" font-family=(token)"font.body" font-size=(token)"size.heading" { span "Hello Zenith" }
    }
  }
}
```

See [`examples/`](./examples) for runnable `.zen` files covering shapes, rich text, code blocks, images, frames/groups, multi-page documents and styles, plus the richer features — `gradient`, `shadow`, `blur`, `filter`, `mask`, `table`, `flowchart` (shapes + connectors), and `anchors`.

## Command surface

Run `zenith <command> --help` for flags. Every command supports `--json` for machine-readable output.

| Group        | Commands                                                               |
| ------------ | ---------------------------------------------------------------------- |
| **Author**   | `validate` · `fmt` · `tokens` · `inspect`                              |
| **Render**   | `render` (`--pdf` · `--scene` · `--all-pages` · `--spread` · `--page`) |
| **Edit**     | `tx` (typed transactions, dry-run by default)                          |
| **Variants** | `merge` (CSV mail-merge, `--manifest`)                                 |
| **Library**  | `library list` · `library add`                                         |
| **History**  | `history` · `undo` · `redo` · `version` · `restore` · `sync`           |

## History & versions

Zenith keeps a **local edit history per document**, entirely off the render path — it never
changes the pixels. On the first edit, a document is given a stable identity: a ULID `doc-id`
stamped into the file and ignored by the renderer. From then on, every engine edit (`tx --apply`,
`library add`, an `undo`/`redo`/`restore`) is recorded as a full **snapshot** (state, not an
op-log), so history survives even hand-edits.

```bash
zenith tx poster.zen recolor.json --apply   # an edit — recorded automatically
zenith undo poster.zen                       # step back, rewriting the file in place
zenith redo poster.zen                       # step forward again
zenith history poster.zen                    # list the timeline (--json for tooling)

zenith version poster.zen "launch-v1"        # save a NAMED version, kept indefinitely
zenith restore poster.zen launch-v1          # rewrite the doc to that version
zenith restore poster.zen @head~1            # …or to a revspec: v2, @head~1, @latest:named, a name

zenith sync poster.zen                        # capture an out-of-band change (GUI / hand-edit / git checkout)
```

There are two tiers. **Undo/redo** walk an ephemeral session timeline — fast, local, for
in-progress work. **Named versions** are durable, content-addressed checkpoints (SHA-256 +
DEFLATE) retained until you remove them; `restore` can jump to any of them. `sync` is how an edit
made _outside_ the engine gets folded back in so the history stays complete. History recording is
best-effort: if it can't write, your file still saves — the edit is never blocked.

## Works in your repo

Because a Zenith file is just text, it lives wherever your code lives. Commit it to git. Review design changes in a pull request, side by side with the diff. Render it in CI to catch a broken layout before it ships. Generate variants in a pipeline. Roll back like any other file. Design stops being a separate world you export to and from, and becomes part of the build.

## Who it's for

- **AI and agent builders** who need to generate and edit visuals reliably, not by screenshot-and-pray.
- **Engineering teams** who want design assets in the repo, reviewed in PRs, and built in CI.
- **High-volume producers** — marketing, publishing, localization — who need lots of correct variants.
- **Tool builders** who'd rather build on an open format than a closed cloud API.

## Showcase

The public showcase lives at [`farhan-syah/zenith-showcase`](https://github.com/farhan-syah/zenith-showcase) and is linked here as the [`zenith-showcase`](./zenith-showcase) submodule.

It is the place for reusable Zenith examples: `.zen` source, rendered outputs, visual recipes, actions, filters, backgrounds, posters, flyers, books, magazines, ads, diagrams, presentations, and other generated design work.

Only put files in the showcase if you have the rights to share them and you allow others to reuse the submitted source, outputs, and assets under the declared license. Private, client, portfolio-only, or custom-licensed work should be linked from the showcase's external gallery instead; licensing for external work stays with the owner.

## Install

> Releases are published from version tags. Until the first `vX.Y.Z` tag is cut, the commands below have nothing to download yet — see [Status](#status).

### Install script

The recommended way to install the `zenith` CLI is the install script, which detects your platform and downloads the matching prebuilt binary from GitHub Releases.

**Linux / macOS**

```bash
curl -fsSL https://raw.githubusercontent.com/farhan-syah/zenith/main/scripts/install.sh | sh
```

**Windows (PowerShell)**

```powershell
irm https://raw.githubusercontent.com/farhan-syah/zenith/main/scripts/install.ps1 | iex
```

Set `ZENITH_INSTALL_DIR` to change the install location (default `~/.local/bin`):

```bash
ZENITH_INSTALL_DIR=/usr/local/bin \
  curl -fsSL https://raw.githubusercontent.com/farhan-syah/zenith/main/scripts/install.sh | sh
```

### With cargo

```bash
cargo install zenith-tool    # from crates.io (installs the `zenith` binary)
cargo install --git https://github.com/farhan-syah/zenith zenith-tool   # from source
```

The library crates (`zenith-core`, `zenith-layout`, `zenith-scene`, `zenith-render`, `zenith-tx`, `zenith-session`) are published under their own names for Rust projects that want to build on the engine directly.

### Prebuilt binaries

Download directly from [GitHub Releases](https://github.com/farhan-syah/zenith/releases):

| Platform | Architecture  | Asset                                 |
| :------- | :------------ | :------------------------------------ |
| Linux    | x86_64        | `zenith-<version>-linux-x64.tar.gz`   |
| Linux    | aarch64       | `zenith-<version>-linux-arm64.tar.gz` |
| macOS    | x86_64        | `zenith-<version>-macos-x64.tar.gz`   |
| macOS    | Apple Silicon | `zenith-<version>-macos-arm64.tar.gz` |
| Windows  | x86_64        | `zenith-<version>-windows-x64.zip`    |

### Update

```bash
zenith update                   # latest stable release
zenith update --pre             # latest prerelease
zenith update --version <tag>   # a specific release tag, e.g. the ones on the Releases page
```

Verify with `zenith --version`.

## Status

🚧 **Early, pre-`0.1`.** The author → validate → edit → render pipeline works end-to-end: parsing, the diagnostic set, the transaction engine, PNG/PDF rendering, local history, the library subsystem, and variable-data merge are all implemented and tested. The format, wire types, and command surface may still change before `0.1`. Nothing here is stable yet, and there's no published build to download.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to work on the engine, and [AGENTS.md](AGENTS.md) for the binding repository conventions (the source of truth for both human and agent contributors).

## License

Apache-2.0. See [LICENSE](LICENSE).
