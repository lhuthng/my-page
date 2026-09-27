// Throwaway probe (not part of the package, like the rest of .probe). Checks the
// only real logic in Updater.swift: reading a release tag as a version, and
// deciding whether it beats the running build. The network and the file swap are
// not exercised here — this is the part that silently ships a wrong answer.
//
//   ./Scripts/toolchain-env.sh swiftc -o /tmp/updateprobe .probe/updates/main.swift
//   /tmp/updateprobe

import Foundation

enum Check {
    static var failures = 0

    static func equal(_ a: String?, _ b: String?, _ what: String) {
        guard a == b else {
            print("FAIL \(what): got \(a ?? "nil"), want \(b ?? "nil")")
            failures += 1
            return
        }
    }

    static func isTrue(_ value: Bool, _ what: String) {
        guard value else {
            print("FAIL \(what)")
            failures += 1
            return
        }
    }

    static func isFalse(_ value: Bool, _ what: String) {
        isTrue(!value, what)
    }
}

// Copy of the two functions under test, so the probe needs no app sources.
func normalised(_ raw: String) -> String? {
    let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
    let body = trimmed.hasPrefix("v") ? String(trimmed.dropFirst()) : trimmed
    guard !body.isEmpty, body.allSatisfy({ $0.isNumber || $0 == "." }) else { return nil }
    return body
}

func isNewer(_ candidate: String, than current: String) -> Bool {
    let parts: (String) -> [Int] = { $0.split(separator: ".").map { Int($0) ?? 0 } }
    let (a, b) = (parts(candidate), parts(current))
    for index in 0..<max(a.count, b.count) {
        let left = index < a.count ? a[index] : 0
        let right = index < b.count ? b[index] : 0
        if left != right { return left > right }
    }
    return false
}

// Tags as GitHub actually hands them over.
Check.equal(normalised("v0.2.0"), "0.2.0", "strips the v")
Check.equal(normalised("0.10.0"), "0.10.0", "accepts a bare version")
Check.equal(normalised(" v1.2.3 "), "1.2.3", "trims whitespace")
Check.equal(normalised("nightly"), nil, "rejects a word")
Check.equal(normalised("v0.2.0-rc1"), nil, "rejects a prerelease suffix")
Check.equal(normalised("v"), nil, "rejects an empty tag")

// Numeric compare, not lexicographic: this is the bug a string compare ships.
Check.isTrue(isNewer("0.2.0", than: "0.1.0"), "0.2.0 beats 0.1.0")
Check.isTrue(isNewer("0.10.0", than: "0.9.0"), "0.10.0 beats 0.9.0")
Check.isTrue(isNewer("1.0.0", than: "0.99.99"), "1.0.0 beats 0.99.99")
Check.isTrue(isNewer("0.1.1", than: "0.1.0"), "patch bump wins")
Check.isFalse(isNewer("0.1.0", than: "0.1.0"), "same version is not newer")
Check.isFalse(isNewer("0.1.0", than: "0.2.0"), "older loses")
Check.isFalse(isNewer("0.1", than: "0.1.0"), "shorter is not newer")

if Check.failures == 0 {
    print("ok — all version checks passed")
} else {
    print("\(Check.failures) failed")
    exit(1)
}
