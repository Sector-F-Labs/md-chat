# MD-Chat

A Patina (egui) chat client for OpenAI-compatible APIs, defaulting to a local Ollama, with replies
rendered as markdown by egui_commonmark.

Significant decisions are ADRs in `docs/adr/`; read them before changing the API handling, fonts
or the window layout. Use the `adr` skill to add one.

## Commands

```bash
make check         # tests + clippy -D warnings
make run           # run it (against the user's real config: prefer a scratch one, below)
make screenshot    # docs/screenshot*.png, in a throwaway config dir
make install       # per-user install; test with PREFIX=<scratch dir>
```

Never run test instances against the user's config (`~/.config/MD-Chat/config.toml`, which may
hold an API key; don't read the key). Set `XDG_CONFIG_HOME` and `XDG_DATA_HOME` to a scratch dir,
as `make screenshot` does. The app writes its config on first run and when a setting changes.

egui_commonmark must have a release for Patina's egui version (0.24 is egui 0.35), so check it
before moving egui.

## Layout

- `src/main.rs`: viewport (Patina frame, app id `com.sectorflabs.MdChat`, icon).
- `src/app.rs`: window, header menus and model picker, conversation, input, status bar, modal
  keys, screenshot mode (`MDCHAT_SCREENSHOT`, `MDCHAT_SKIN`).
- `src/config.rs`: `config.toml`, the Ollama default and the migration from the old default.
- `src/openai.rs`: chat completions and the `/v1/models` list.
- `src/history.rs`: history from a local Reservoir (localhost:3017).
- `src/screenshot.rs`: saves the screenshot PNG.
