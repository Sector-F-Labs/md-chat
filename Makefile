APP_NAME := md-chat
APP_ID := com.sectorflabs.MdChat
VERSION := $(shell awk -F\" '/^version = / { print $$2; exit }' Cargo.toml)

PREFIX ?= $(HOME)/.local
BINDIR := $(PREFIX)/bin
APPSDIR := $(PREFIX)/share/applications
ICONDIR := $(PREFIX)/share/icons/hicolor

# macOS app bundle
APP_BUNDLE := target/MD-Chat.app

# A throwaway config and data dir, so screenshots and test runs never touch
# the settings of a copy the user is running.
SCRATCH_ENV := XDG_CONFIG_HOME=$(CURDIR)/target/scratch-config XDG_DATA_HOME=$(CURDIR)/target/scratch-data

.DEFAULT_GOAL := help

.PHONY: help run dev build test check screenshot install uninstall bundle open-app install-app clean

help: ## Show this help
	@echo "Usage: make <target>"
	@echo
	@awk 'BEGIN { FS = ":.*## " } /^[a-z-]+:.*## / { printf "  \033[1m%-14s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

run: ## Run the app
	cargo run

dev: ## Run with live reload (requires cargo-watch)
	@command -v cargo-watch >/dev/null 2>&1 || { echo "cargo-watch is required: cargo install cargo-watch"; exit 1; }
	cargo watch -x "run --bin $(APP_NAME)"

build: ## Build a release binary
	cargo build --release

test: ## Run the tests
	cargo test

check: test ## Run the tests and clippy, as before a commit or release
	cargo clippy --all-targets -- -D warnings

screenshot: ## Regenerate docs/screenshot-*.png in a throwaway config dir
	@rm -rf target/scratch-config target/scratch-data
	@mkdir -p docs target/scratch-config target/scratch-data
	$(SCRATCH_ENV) MDCHAT_SCREENSHOT=docs/screenshot.png MDCHAT_SKIN="Dark Orange" cargo run
	$(SCRATCH_ENV) MDCHAT_SCREENSHOT=docs/screenshot-light.png MDCHAT_SKIN=light cargo run

install: build ## Install MD-Chat and its launcher for the current user (Linux, PREFIX=~/.local)
	install -Dm755 target/release/$(APP_NAME) $(BINDIR)/$(APP_NAME)
	install -Dm644 assets/icon.iconset/icon_256x256.png $(ICONDIR)/256x256/apps/$(APP_ID).png
	install -Dm644 assets/icon.iconset/icon_128x128.png $(ICONDIR)/128x128/apps/$(APP_ID).png
	install -d $(APPSDIR)
	sed 's|^Exec=$(APP_NAME)|Exec=$(BINDIR)/$(APP_NAME)|' packaging/linux/$(APP_ID).desktop > $(APPSDIR)/$(APP_ID).desktop
	-update-desktop-database -q $(APPSDIR)
	@# GTK trusts an icon cache over the folder, so refresh one if it exists.
	-@test ! -f $(ICONDIR)/icon-theme.cache || gtk-update-icon-cache -q -t -f $(ICONDIR)

uninstall: ## Remove what make install put in place
	rm -f $(BINDIR)/$(APP_NAME) $(APPSDIR)/$(APP_ID).desktop
	rm -f $(ICONDIR)/256x256/apps/$(APP_ID).png $(ICONDIR)/128x128/apps/$(APP_ID).png
	-update-desktop-database -q $(APPSDIR)
	-@test ! -f $(ICONDIR)/icon-theme.cache || gtk-update-icon-cache -q -t -f $(ICONDIR)

bundle: build ## Build target/MD-Chat.app (macOS)
	rm -rf $(APP_BUNDLE)
	mkdir -p $(APP_BUNDLE)/Contents/MacOS $(APP_BUNDLE)/Contents/Resources
	cp target/release/$(APP_NAME) $(APP_BUNDLE)/Contents/MacOS/MD-Chat
	cp assets/icon.icns $(APP_BUNDLE)/Contents/Resources/icon.icns
	cp Info.plist $(APP_BUNDLE)/Contents/Info.plist

open-app: bundle ## Build and open the macOS app
	open $(APP_BUNDLE)

install-app: bundle ## Copy the macOS app to /Applications
	cp -R $(APP_BUNDLE) /Applications/

clean: ## Remove build output
	cargo clean
