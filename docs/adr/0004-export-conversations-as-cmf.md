# 0004. Export conversations as CMF

Date: 2026-10-11

## Status

Accepted

## Context

A conversation in MD-Chat lived only in the window (or in Reservoir). The user wants to save it
in [CMF](https://sectorflabs.com/cmf/), Sector F's Conversational Markdown Format: user messages
are blockquotes, and everything else is the assistant's. A CMF file is plain CommonMark that
other CMF tools can read.

## Decision

- An **Export** button sits at the far right of the title bar, by the window buttons. Per Patina's
  ADR-0006 that side holds the app's output, and an exported conversation is MD-Chat's output.
  File → Export as CMF… (Ctrl+E) does the same.
- It asks where to save with the desktop's save dialog (rfd), suggesting a name made from the
  first question, e.g. `whats-a-monad.cmf`.
- The file is written with the reference `cmf` crate's `Document::to_cmf`, so MD-Chat's output
  matches the reference parser and CLI. Before that:
  - system messages (the prompt, errors) are left out, since CMF has no place for them;
  - consecutive replies are joined;
  - reply lines starting with `>` are escaped by indenting them one space, as CMF specifies;
  - replies before the first user message become a preamble, which readers skip.
- The status bar reports where the file went, or why it couldn't be written.

## Consequences

- Conversations leave MD-Chat as readable, greppable files that other CMF tools understand.
- The `cmf` crate brings its CLI's dependencies (clap, colored, pulldown-cmark) into the build.
- The reference writer leaves two blank lines between turns. That is valid CMF, and it changes
  if the crate changes.
- Export is one-way for now; opening a `.cmf` file to continue a chat would be a later decision.
