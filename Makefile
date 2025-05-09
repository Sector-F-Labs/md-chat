BINARY_NAME=md-chat
APP_NAME='MD-Chat'
APP_BUNDLE=target/$(APP_NAME).app
ICON=assets/icon.icns

.PHONY: all build bundle open-app clean install icon

all: bundle

build:
	cargo build --release

icon:
	bash docs/make-icon.sh

bundle: icon build
	rm -rf $(APP_BUNDLE)
	mkdir -p $(APP_BUNDLE)/Contents/MacOS
	mkdir -p $(APP_BUNDLE)/Contents/Resources
	cp target/release/$(BINARY_NAME) $(APP_BUNDLE)/Contents/MacOS/$(APP_NAME)
	cp $(ICON) $(APP_BUNDLE)/Contents/Resources/icon.icns
	cp Info.plist $(APP_BUNDLE)/Contents/Info.plist

open-app: bundle
	open $(APP_BUNDLE)

clean:
	rm -rf $(APP_BUNDLE)

install: bundle
	cp -R $(APP_BUNDLE) /Applications/ 