# 0003. Ollama by default, models from the API

Date: 2026-10-11

## Status

Accepted

## Context

MD-Chat defaulted to `https://api.openai.com` and offered a fixed list of models
(gemini-2.0-flash, gpt-4.1, gpt-4o-mini, gpt-4o), or a `models` list in `config.toml`. Without
an API key the default could never work, and the list offered models the API might not have.
The user runs [Ollama](https://ollama.com) locally and wants MD-Chat to use it, showing only the
models actually installed.

## Decision

- The default `api_url` is a local Ollama, `http://localhost:11434`. A config still holding the
  old default (OpenAI's URL with no API key) is moved to Ollama when loaded. A config with a key
  keeps its URL.
- The model picker lists what the API reports at `GET <api_url>/v1/models`, OpenAI's endpoint,
  which Ollama, OpenAI and most compatible servers have. Embedding models (ids containing
  "embed") are left out, since they can't chat. The list is fetched off the UI thread at start
  and again from File → Refresh Models.
- The `models` config key is gone. The last model picked is saved as `model` and chosen again if
  the API still has it; otherwise the API's first model is chosen.
- When the list can't be fetched, the status bar says why, and Send stays disabled until a model
  is available.

## Consequences

- MD-Chat works out of the box on a machine running Ollama, and never offers a model it can't use.
- An API without `/v1/models` shows no models; that would need a fallback list again.
- Leaving out embedding models by name is a heuristic; a chat model with "embed" in its name
  would be hidden.
