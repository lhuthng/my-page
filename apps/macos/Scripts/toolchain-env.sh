#!/bin/sh
# Pins the environment to a working Swift toolchain on this machine, then execs
# the given command. See README "Toolchain notes".
#
# Xcode's toolchain with its own macOS 27 SDK is the good path: it builds this
# package with no linker warnings at all, so prefer it whenever Xcode is
# installed.
XCODE="/Applications/Xcode.app/Contents/Developer"
if [ -d "$XCODE" ]; then
	export DEVELOPER_DIR="$XCODE"
	exec "$@"
fi

# Fallback for a machine with no Xcode: the macOS 27 Command Line Tools ship
# without libSwiftUIMacros.dylib, so SwiftUI cannot compile against the 27 SDK.
# The swift.org toolchain (brew install swift) plus the CLT's 26.5 SDK — whose
# SwiftUI still uses plain property wrappers — does compile. Expect linker
# warnings about search paths and about linking the system's macOS 27 Swift
# runtime; they are harmless, and installing Xcode removes them.
TC=$(ls -d /opt/homebrew/opt/swift/Swift-*.xctoolchain 2>/dev/null | head -1)
if [ -n "$TC" ] && [ -x "$TC/usr/bin/swift" ]; then
	export PATH="$TC/usr/bin:$PATH"
	export DEVELOPER_DIR="/Library/Developer/CommandLineTools"
	export SDKROOT="/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk"
fi
exec "$@"
