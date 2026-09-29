APP_NAME := Audiobooks

# Toolchain selection + SDK pinning live in Scripts/toolchain-env.sh (see
# README "Toolchain notes" for why this machine needs it).

.PHONY: run debug test app open clean

# Dev iteration against the local backend:
#   ./Scripts/toolchain-env.sh env AUDIOBOOKS_API_BASE=http://127.0.0.1:3000 swift run
run:
	./Scripts/toolchain-env.sh swift run

debug:
	./Scripts/toolchain-env.sh swift build

test:
	./Scripts/toolchain-env.sh swift test

app:
	./Scripts/toolchain-env.sh ./Scripts/bundle.sh

open: app
	open build/$(APP_NAME).app

clean:
	./Scripts/toolchain-env.sh swift package clean
	rm -rf build
