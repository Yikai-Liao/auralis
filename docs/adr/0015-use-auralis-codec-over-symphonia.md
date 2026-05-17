# Use auralis-codec over Symphonia for audio codecs

Auralis audio file I/O will use `auralis-codec` as the public codec facade.
Backend crates must stay hidden behind Auralis-owned buffer, option,
diagnostic, and encode-summary types.

The default decode backend is Symphonia. Symphonia is decode-only in the
Auralis plan. WAV decode also tries Symphonia first. `hound` is allowed only as
a WAV fallback or special-case WAV path when Symphonia cannot handle an
Auralis-supported case. `hound` is not the primary decode architecture.

Do not introduce new target architecture around one crate per container such as
`auralis-flac`, `auralis-aiff`, or `auralis-au`. If such crates exist during
migration, treat them as consolidation debt behind `auralis-codec`, not as the
long-term design.

Encoding support is format-specific and must be planned explicitly. The current
target exposes WAV encode first. FLAC encode may be planned behind
`auralis-codec` with `flacenc` as the candidate backend. Other format encode
entries remain placeholders until an encoder backend and maintenance policy are
selected. Symphonia decode support does not imply that Auralis should write a
local encoder for the same format.

**Considered Options**

- Use `auralis-codec` as the facade over Symphonia, with `hound` only as WAV
  fallback: accepted because it keeps the public API small, avoids local codec
  implementations, and keeps backend swaps possible.
- Keep encode initially to WAV, plan FLAC encode through `flacenc`, and leave
  other non-WAV encode placeholders: accepted because Symphonia does not encode
  and encoder policy needs separate review.
- Build and maintain Auralis-owned codec crates for each container: rejected
  because it increases maintenance burden and duplicates mature decoder work.
- Shell out to external codec tools from normal decode paths: rejected because
  it makes local and CI behavior harder to reproduce.
