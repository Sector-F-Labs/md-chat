# MD-Chat

A lightweight, fast desktop client for experimenting with OpenAI-compatible APIs. Built with [egui](https://github.com/emilk/egui) for native performance and [CommonMark](https://commonmark.org/) support.

![Screenshot](docs/screenshot.png)

## Features

- 🚀 Fast native performance with egui
- 📝 Full Markdown rendering support
- 🌓 Light/Dark mode toggle
- 💾 Persistent window position
- 🔄 Compatible with OpenAI and similar APIs (like [Reservoir](https://github.com/Sector-F-Labs/reservoir))
- 🎨 Clean, minimal interface

![Screenshot](docs/screenshot.png)
## Configuration

MD-Chat uses a TOML config file for settings such as your OpenAI API key and API URL. **Environment variables are no longer required.**

### Config File Location
- **macOS:** `~/Library/Application Support/MD-Chat/config.toml`
- **Linux:** `~/.config/MD-Chat/config.toml`

The config file is created automatically on first run if it does not exist.

### Example `config.toml`
```toml
openai_api_key = "sk-...yourkey..."
api_url = "https://api.openai.com/v1/chat/completions"
```
- If `openai_api_key` is missing or empty, requests will fail unless the API does not require a key.
- You can edit this file to change your API key or use a different API URL.

## Building and Running

```bash
# Build the application
cargo build --release

# Run the application
cargo run --release
```

## Usage

1. Launch the application
2. Type your message in the input box
3. Press Enter or click Send to submit
4. View the markdown-formatted response

## Development

The codebase is organized into two main components:

- `src/main.rs`: UI and application logic
- `src/openai.rs`: API client implementation

## Dependencies

- eframe: Egui framework for native applications
- egui_commonmark: Markdown rendering support
- reqwest: HTTP client
- tokio: Async runtime
- serde: Serialization/deserialization

## License

Licensed under the Apache License, Version 2.0 (the "License"); you may not use this code except in compliance with the License. You may obtain a copy of the License at

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License. 
