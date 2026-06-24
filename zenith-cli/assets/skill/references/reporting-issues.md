# Reporting a Zenith bug or feature request

Use this workflow when you hit a **genuine engine limitation** during real work — wrong/incorrect
rendered output, a crash, a determinism violation, or a capability only reachable via multiple
long/fragile workarounds. This is an AI-native feedback loop: no CLI command; you use `gh` directly.

## When NOT to file

First rule out your own authoring errors:

1. Re-read `zenith schema node <kind>` — confirm the attribute exists and you have the right type.
2. Read the `zenith validate` diagnostic in full — it tells you the fix ("did you mean?",
   raw-literal hints, fit font-size suggestions). Act on it before concluding it's a bug.
3. Run `zenith schema token <type>` to verify token value forms (KDL typed literals, not CSS strings).

If any of those point to a usage error, fix it — don't file.

## Severity triage

Judge from the hands-on failure, not from description alone:

| Severity   | Condition                                                                                      |
| ---------- | ---------------------------------------------------------------------------------------------- |
| `critical` | Wrong output / crash / data loss / determinism break (same source → different bytes).          |
| `high`     | Blocks a real task; the only workarounds are multi-step and fragile.                           |
| `medium`   | Works but needs a workaround, or a clearly-missing convenience.                                |
| `low`      | Papercut / cosmetic / nice-to-have.                                                            |

## Flow (follow this order strictly)

**1. Reproduce minimally.** Confirm it's a real engine gap — not a usage error — using
`zenith schema` and `zenith validate`. Reduce to the smallest `.zen` that triggers it.

**2. Check the version.** The bug may already be fixed:

```bash
zenith --version
```

If the version is old, suggest `zenith update` instead of filing.

**3. Ask the human's permission.** Summarize: what you'd file, the severity, and the minimal
repro. Proceed only if they agree AND the GitHub CLI is authenticated:

```bash
gh auth status
```

**4. Deduplicate.** Search open AND closed issues before creating a new one:

```bash
gh issue list --repo <zenith-repo> --state all --search "<keywords>"
```

- Open match → **comment** your minimal repro there instead of opening a new issue:
  `gh issue comment <number> --body "…"`
- Closed/fixed match → likely resolved upstream. Tell the human; don't file.

Confirm the correct repo slug with the human if unsure — never guess or file against the wrong repo.

**5. File the issue.** Only after steps 1–4:

```bash
gh issue create \
  --repo <zenith-repo> \
  --title "<concise description>" \
  --label "type:bug,severity:critical" \
  --body "$(cat <<'BODY'
## What I tried
<brief description of the authoring intent>

## Expected
<what the engine should produce>

## Actual
<what it actually produced — wrong output, crash, mismatch>

## Minimal repro
\`\`\`kdl
// Smallest .zen that triggers the issue
\`\`\`

## Workaround
<if any; otherwise "none">

## Version / OS
<output of \`zenith --version\`>
BODY
)"
```

Labels: `type:bug` or `type:feature`; `severity:critical`, `severity:high`, `severity:medium`, or `severity:low`.

## Critical: no project data in the repro

The repro **must** be a minimal **synthetic** example. Never include:

- The user's real document content, real text copy, or token names/values.
- Asset paths, file paths, or any project-identifying data.

Reduce to the smallest generic `.zen` that still triggers the bug. If you cannot reproduce
without sensitive content, describe it abstractly and ask the human to provide a sanitized repro.
