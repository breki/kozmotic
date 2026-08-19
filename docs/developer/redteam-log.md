# Red Team Findings -- Deferred backlog

Red Team findings that were **deferred** -- real, but not
fixed at review time. Fixed findings are not logged here;
their resolution lives in the commit that fixed them.

Newest first; add new entries right after the `---`. Use a
self-describing ID `rt-<YYYY-MM-DD>-<kebab-slug>` (no central
counter); a later commit acting on an item cites the ID
inline. Each entry: the ID heading, a `**Category:**` line,
and a short description.

**Threshold:** when 10+ items are open here, a full-codebase
red team review is warranted before continuing feature work.

---

## rt-2026-08-19-release-provenance

**Category:** Supply chain / trust domain

`SHA256SUMS` is uploaded to the same release, over the same
`GH_TOKEN`, as the archives it attests. Anyone who can write
to the release can rewrite the archives and the digest list
in one step, and every documented verification command still
reports `OK`. The asset defends against transport corruption
and mirror drift, not against a compromised token or a
malicious workflow change -- which is weaker than the
README's "checked against the release" reads.

Proposed: sign `SHA256SUMS` (cosign/minisign) or add
`actions/attest-build-provenance` for the archives and the
sums file, and document the signature check as the real
verification step. Also narrow `permissions: contents:
write` from the whole workflow to the `release` job -- the
reusable `ci` job inherits it today.

Deferred because it adds a signing dependency and a key
policy, and because narrowing permissions needs the reusable
CI workflow's own needs checked first. Neither is testable
without a real tag push.

## rt-2026-08-19-release-workflow-dry-run

**Category:** Untestable release path

`.github/workflows/release.yml` only ever executes on a real
tag push, so the checksum step and the create-or-publish
guard are tested in production. The file has been edited
four times in a week.

Proposed: a `workflow_dispatch` input that runs the checksum
step and the publish logic against a throwaway pre-release
tag, so the draft/published/refuse paths can be exercised
before a real release depends on them.

Deferred because it is a new workflow entry point rather
than part of publishing checksums. The checksum step's own
script was exercised locally against the five published
v2.1.1 assets in the meantime.

