#!/usr/bin/env bash
#
# Rebuilds LAUNCHER.EXE from game_launcher.c.
#
# The launcher is a freestanding Win32 binary: no CRT, entry point is
# WinMainCRTStartup, and it only calls plain Win95-era APIs so it runs on the
# shared Windows 95 base image. It is committed as a binary because the build
# needs a Windows cross-compiler that CI does not have.
#
# Requires mingw-w64 (brew install mingw-w64).
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

CC="${CC:-i686-w64-mingw32-gcc}"
command -v "$CC" >/dev/null || {
	echo "error: $CC not found (brew install mingw-w64)" >&2
	exit 1
}

# -nostdlib/-nostartfiles: freestanding, no CRT to link against.
# -fno-builtin:            stops gcc rewriting the hand-rolled string helpers
#                          into libc calls (strlen) that would not resolve.
# -mno-stack-arg-probe:    avoids a __chkstk_ms reference from libgcc.
# -mwindows:               GUI subsystem, so no console window appears.
"$CC" \
	-Os -s -mwindows \
	-nostdlib -nostartfiles -fno-builtin -mno-stack-arg-probe \
	-Wl,-e,_WinMainCRTStartup \
	game_launcher.c -o LAUNCHER.EXE \
	-lkernel32 -luser32

echo "built LAUNCHER.EXE ($(wc -c < LAUNCHER.EXE) bytes)"
echo "imported DLLs:"
i686-w64-mingw32-objdump -p LAUNCHER.EXE 2>/dev/null | grep "DLL Name" || true
