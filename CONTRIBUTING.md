# Contributing

Bug reports, ideas and pull requests are welcome. Open an issue first for anything larger than a small fix, so the
change can be discussed before you spend time on it.

## Terms for contributions

The project is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE.md), and its author keeps the right to
offer it under other terms, including commercial ones. To keep that possible, every contribution is accepted on these
terms:

- You confirm that you wrote the contribution yourself, or otherwise have the right to submit it under these terms.
- You grant the project's author (tarpetos) a perpetual, worldwide, non-exclusive, royalty-free, irrevocable license
  to use, copy, modify, publish, distribute, sublicense and relicense your contribution, under any terms, including
  commercial ones.
- Your contribution is published in this repository under the project's license.

Opening a pull request means you agree to these terms.

## Ground rules

- Everything in English: code, comments, documentation, UI text and commit messages.
- **A render must repeat exactly.** The same scene and options give the same pixels on every platform and every
  thread count. Read [docs/determinism.md](docs/determinism.md) before you touch `fractique-core`. A change that
  alters what existing scenes render to is a breaking change: say so in the pull request.
- The scene format in [docs/scene-format.md](docs/scene-format.md) is the source of truth: change it together with
  the code. A scene file that loaded before must keep loading.
- `fractique-core` stays free of files, terminals, networks and UI. Everything else wraps it.
- Do not copy code from flam3, Apophysis, Fractorium, JWildfire or other renderers: their licenses do not fit this
  project. Formulas from published papers are fine.
- Before you open a pull request: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --release`.

## Comments

Code explains itself through names, types and structure. A comment is the exception, written only when it is
fully justified: when a reader would otherwise get something wrong and the code cannot say it.

- **Write one** for a reason the code cannot show: a workaround for a bug or a platform quirk, a constraint from
  outside (determinism, a file format, a thread), or where a number comes from when it was measured or tuned (one
  line, with the number).
- **Do not write one** that repeats what the code or a name already says, describes a field or a function whose
  signature is clear, narrates history ("used to", "now", "since 0.3"), or explains the feature to a user.
  Architecture and behaviour belong in `docs/`, not in module headers.
- Keep it short: one or two lines.
- Doc comments (`///`, `//!`) only on items whose contract is not obvious from the signature.
- When in doubt, leave it out.

## Commit messages

Pull requests are squash-merged: the pull request's title becomes the one commit on `master` (the pull request number
is added to it), and its description becomes the commit body. A check (`PR title`) refuses a title that does not
follow this convention ([Conventional Commits](https://www.conventionalcommits.org/)):

```
type(scope): summary
```

- **type**, one of:
  - `feat`: something new the user can do;
  - `fix`: something that did not work and now does;
  - `perf`: the same, faster or lighter;
  - `refactor`: code changed, behaviour not;
  - `docs`: documentation only;
  - `build`: installers, build scripts, dependencies;
  - `ci`: GitHub workflows;
  - `chore`: the rest (version bumps, housekeeping);
  - `revert`: undoes an earlier commit.
- **scope**, optional, one of:
  - `core`: the renderer (`fractique-core`);
  - `cli`: the command line tool;
  - `mcp`, `server`, `app`: the MCP server, the HTTP server, the apps;
  - `setup`: `setup.sh` / `setup.cmd`;
  - `build`, `ci`, `docs`, `release`, `deps`.
- **summary**: what changes, as seen by a user where there is one. It starts in lower case, has no full stop at the
  end. Keep it short; details belong in the description.
- **Breaking change**: `!` before the colon (`feat(core)!: ...`), with the details in the description. A change to
  what existing scenes render to is one.

Examples:

```
feat(core): add the waves variation
fix(cli): refuse a size above the limit with a clear message
docs: explain how layers blend
chore(release): 0.1.1
```

Direct commits to `master` (maintainers only) follow the same convention.
