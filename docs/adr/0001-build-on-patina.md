# 0001. Port MD-Chat onto Patina

Date: 2026-10-11

## Status

Accepted

## Context

MD-Chat is a small native client for OpenAI-compatible APIs, including
[Reservoir](https://github.com/Sector-F-Labs/reservoir), that shows replies as rendered markdown.
It was built on eframe 0.31 with the desktop's title bar, a sun/moon light/dark toggle, and a
toolbar holding Refresh, a model picker and the vim-style mode indicator.

The other Sector F desktop apps (Notebook, Calendar, Photograph, UnAmp) now share
[Patina](https://github.com/Sector-F-Labs/patina): one window frame, one skin format and one
title-bar layout. MD-Chat looked and behaved unlike them, and its egui was four versions behind.

## Decision

We will build MD-Chat on Patina (`patina-ui`), with egui and eframe at Patina's version (0.35),
and egui_commonmark at the release made for it (0.24):

- Patina's window frame, with a "System Title Bar" setting in View as the escape hatch.
- Title bar per Patina's ADR-0006: word menus on the left (File, View, Skins). On the right, by
  the window buttons, goes the model picker, since the model is what writes the app's output.
- A status bar across the bottom holds the vim-style mode (NORMAL/INSERT) and what the app is
  waiting on, and rounds both bottom corners.
- Skins from Patina's built-ins and the shared `~/.config/patina/skins/`, starting on Default,
  which follows the desktop's light/dark setting. The light/dark toggle goes; a skin replaces it.
- The skin and title-bar settings are saved in `config.toml` beside the API settings.
- The start-up history fetch from Reservoir moves off the UI thread, so a slow or missing
  Reservoir no longer holds up the window.

## Consequences

- MD-Chat looks and behaves like the rest of the Patina family and takes the same skins.
- It moves to a new egui version only together with Patina, and only once egui_commonmark has a
  release for that version.
- The mode indicator's colours come from the skin (its selection colours) instead of fixed
  black and white.
