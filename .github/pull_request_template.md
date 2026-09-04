## What and why

<!-- One or two sentences: what this PR changes and the problem it solves.
     Reference the issue it closes, if any (closes #N). -->

## How it was verified

<!-- Paste the checks you ran. Nothing merges red:
     cargo fmt --all -- --check
     cargo clippy --workspace --all-targets -- -D warnings
     cargo test --workspace
     bash scripts/check.sh
     Plus, when the change touches circuits/artifacts/proving:
     nargo test (circuits), artifacts check + fresh-compile determinism,
     cargo test -p crucible-tests --test ultrahonk --test real_backend
-->

## Privacy / security notes

<!-- Does this touch witness material or the verification path? Confirm no
     private value can appear in logs, errors, or outputs; cite the security
     test that pins the behavior, or explain why none is needed. -->

## Definition of done

- [ ] One logical improvement per commit, with a message explaining what and why
- [ ] fmt, clippy, and the relevant tests are green locally
- [ ] Docs updated where the change affects a documented behavior (README, docs/, crate docstrings)
- [ ] No Codebuff or other tool-generated attribution in commits

## Notes for reviewers

<!-- Design trade-offs, deferred work, or areas you want extra scrutiny. -->
