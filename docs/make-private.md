# Turning the GitHub repository private

Do this in the browser, because it needs the owner's session and I could not: the
HTTPS credential on this machine belongs to an account with read-only access to the
repository, and the API refused with 404 on `PATCH /repos/...` — the standard
response when the token lacks admin, not an indication the repository is missing.

**Settings → General → Danger Zone → Change repository visibility → Change
visibility → type the repository name → Make private.**

Two minutes. It takes effect immediately and the Pages deployment, if any is
running, stops serving.

## What becomes unavailable, and what does not

| | public | private |
|---|---|---|
| the code | readable by anyone | your account and invited collaborators only |
| GitHub Pages | served | not served |
| `cargo install` from GitHub | works for anyone | works for anyone with read access, which is nobody by default |
| the release workflow | runs on push | unchanged |
| badges in the README pointing at the repo | render | broken for outside readers, which is fine while it is private |
| issues and pull requests | open to anyone | collaborators only |

Nothing in the build depends on the repository being public. CI runs on Actions,
npm reads from the registry, and crates.io is not being published yet.

## Making it public again, when you are ready

Settings → Danger Zone → Change visibility → Public. Nothing else needs doing;
the tag, the history and the release assets are all still there.

Before you flip it, the checklist is in `docs/en/06-release-and-ci.md` and the
short version is: `node bench/harness/links.mjs --deny --for github,npm` passes,
CI is green on all three platforms, and `DOCS_URL` is filled in if you want the
Pages link on the front page.
