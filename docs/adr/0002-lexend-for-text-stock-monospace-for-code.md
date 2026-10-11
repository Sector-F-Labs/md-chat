# 0002. Lexend for text, stock monospace for code

Date: 2026-10-11

## Status

Proposed

## Context

Patina's design language asks for egui's stock fonts unless an app truly needs its own. MD-Chat
has bundled Lexend since before the port, for both proportional and monospace text, with
monochrome Noto Emoji as a fallback, because models answer with emoji that egui's fonts lack.
Lexend as the monospace font made code blocks in replies proportional, so columns didn't line up.

## Decision

We will keep Lexend for proportional text and Noto Emoji as the fallback for both families, but
leave monospace text in egui's stock monospace font (Hack), so code blocks and the mode
indicator are monospaced.

## Consequences

- Replies keep MD-Chat's look, and code in them lines up.
- The binary carries two fonts (Lexend and Noto Emoji), a few hundred KB.
- Skins can't change the text font; if Patina's skin format gains one, revisit this.
