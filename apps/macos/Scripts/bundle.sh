#!/bin/sh
# Build a release binary and wrap it in a minimal .app bundle, ad-hoc signed.
set -eu
cd "$(dirname "$0")/.."

APP_NAME="Audiobooks"
BUNDLE_ID="site.huuthangle.audiobooks"
# CI overrides this from the release tag; local builds keep the default.
VERSION="${AUDIOBOOKS_VERSION:-0.1.0}"

swift build -c release

BIN=".build/release/$APP_NAME"
APP="build/$APP_NAME.app"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/$APP_NAME"

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key><string>$APP_NAME</string>
	<key>CFBundleDisplayName</key><string>Audiobooks</string>
	<key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
	<key>CFBundleExecutable</key><string>$APP_NAME</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
	<key>CFBundleShortVersionString</key><string>$VERSION</string>
	<key>CFBundleVersion</key><string>$VERSION</string>
	<key>LSMinimumSystemVersion</key><string>15.0</string>
	<key>LSApplicationCategoryType</key><string>public.app-category.books</string>
	<key>NSPrincipalClass</key><string>NSApplication</string>
	<key>NSHighResolutionCapable</key><true/>
	<key>NSAppTransportSecurity</key>
	<dict>
		<key>NSExceptionDomains</key>
		<dict>
			<key>localhost</key>
			<dict><key>NSExceptionAllowsInsecureHTTPLoads</key><true/></dict>
			<key>127.0.0.1</key>
			<dict><key>NSExceptionAllowsInsecureHTTPLoads</key><true/></dict>
		</dict>
	</dict>
</dict>
</plist>
EOF

codesign --force --sign - "$APP"
echo "Built $APP"
