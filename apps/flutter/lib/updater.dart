import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;

enum UpdatePhase {
  idle,
  checking,
  upToDate,
  available,
  downloading,
  installing,
  failed
}

class UpdateRelease {
  const UpdateRelease({
    required this.version,
    required this.downloadUrl,
    required this.digest,
    this.pageUrl,
    this.size,
    this.notes,
  });

  final String version;
  final Uri downloadUrl;
  final String digest;
  final Uri? pageUrl;
  final int? size;
  final String? notes;
}

class UpdateException implements Exception {
  const UpdateException(this.message);

  final String message;

  @override
  String toString() => message;
}

class UpdateController extends ChangeNotifier {
  UpdateController({
    required this.currentVersion,
    http.Client? client,
    Uri? latestReleaseUri,
  })  : _client = client ?? http.Client(),
        _latestReleaseUri = latestReleaseUri ??
            Uri.parse(
              'https://api.github.com/repos/$repository/releases/latest',
            );

  static const String repository = 'lhuthng/my-page';
  static const String assetSuffix = '.app.zip';

  final String currentVersion;
  final http.Client _client;
  final Uri _latestReleaseUri;

  UpdatePhase _phase = UpdatePhase.idle;
  UpdateRelease? _release;
  double _progress = 0;
  String? _message;

  UpdatePhase get phase => _phase;
  UpdateRelease? get release => _release;
  double get progress => _progress;
  String? get message => _message;
  bool get isBusy =>
      _phase == UpdatePhase.checking ||
      _phase == UpdatePhase.downloading ||
      _phase == UpdatePhase.installing;

  Future<void> check() async {
    if (isBusy) return;
    _phase = UpdatePhase.checking;
    _release = null;
    _message = null;
    _progress = 0;
    notifyListeners();

    try {
      final response = await _client.get(
        _latestReleaseUri,
        headers: const {'Accept': 'application/vnd.github+json'},
      ).timeout(const Duration(seconds: 15));
      if (response.statusCode < 200 || response.statusCode >= 300) {
        throw UpdateException('GitHub returned ${response.statusCode}');
      }
      final release = releaseFromJson(response.body);
      if (isNewer(release.version, currentVersion)) {
        _release = release;
        _phase = UpdatePhase.available;
      } else {
        _release = null;
        _phase = UpdatePhase.upToDate;
      }
    } on UpdateException catch (error) {
      _phase = UpdatePhase.failed;
      _message = error.message;
    } catch (_) {
      _phase = UpdatePhase.failed;
      _message =
          'Could not check for updates. Check your network and try again.';
    }
    notifyListeners();
  }

  Future<void> install(UpdateRelease release) async {
    if (isBusy) return;
    if (!Platform.isMacOS) {
      throw const UpdateException(
          'Updates install automatically on macOS only.');
    }

    _release = release;
    _phase = UpdatePhase.downloading;
    _progress = 0;
    _message = null;
    notifyListeners();

    Directory? temp;
    Directory? installed;
    try {
      if (release.size != null && release.size! <= 0) {
        throw const UpdateException('The release archive is empty.');
      }
      temp = await Directory.systemTemp.createTemp('audiobooks-update-');
      final archive = File('${temp.path}/update.zip');
      await _download(release, archive);
      _phase = UpdatePhase.installing;
      notifyListeners();
      await _verify(archive, release.digest);
      installed = await _installMacOS(archive);
      await temp.delete(recursive: true);
      temp = null;
      await _relaunch(installed);
    } on UpdateException catch (error) {
      _phase = UpdatePhase.failed;
      _message = error.message;
      notifyListeners();
    } catch (_) {
      _phase = UpdatePhase.failed;
      _message = 'The update could not be installed. Try again later.';
      notifyListeners();
    } finally {
      if (temp != null) {
        try {
          await temp.delete(recursive: true);
        } catch (_) {}
      }
    }
  }

  Future<void> _download(UpdateRelease release, File destination) async {
    final request = http.Request('GET', release.downloadUrl);
    final response =
        await _client.send(request).timeout(const Duration(seconds: 60));
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw UpdateException('Download returned ${response.statusCode}');
    }

    final sink = destination.openWrite();
    final total = response.contentLength;
    var received = 0;
    var lastReported = -1.0;
    try {
      await for (final chunk in response.stream) {
        sink.add(chunk);
        received += chunk.length;
        if (total != null && total > 0) {
          final fraction = received / total;
          if (fraction - lastReported >= 0.01) {
            lastReported = fraction;
            _progress = fraction.clamp(0, 1);
            notifyListeners();
          }
        }
      }
    } finally {
      await sink.close();
    }
    _progress = 1;
  }

  static Future<void> _verify(File archive, String expected) async {
    final digest = await sha256.bind(archive.openRead()).first;
    if (digest.toString() != expected.toLowerCase()) {
      throw const UpdateException(
        'Downloaded archive does not match the release digest.',
      );
    }
  }

  static Future<Directory> _installMacOS(File archive) async {
    final installed = _installedBundle();
    final parent = installed.parent;
    final staging = Directory(
      '${parent.path}/.audiobooks-update-${DateTime.now().microsecondsSinceEpoch}',
    );
    try {
      await staging.create();
    } on FileSystemException {
      throw UpdateException(
        'Need write access to ${parent.path} to install the update.',
      );
    }

    try {
      final extracted = await Process.run('/usr/bin/ditto', [
        '-x',
        '-k',
        archive.path,
        staging.path,
      ]);
      if (extracted.exitCode != 0) {
        throw const UpdateException(
            'The update archive could not be unpacked.');
      }

      final installedId = await _bundleIdentifier(installed);
      Directory? candidate;
      for (final entry in staging.listSync().whereType<Directory>()) {
        if (!entry.path.endsWith('.app')) continue;
        if (await _bundleIdentifier(entry) == installedId) {
          candidate = entry;
          break;
        }
      }
      if (candidate == null) {
        throw const UpdateException(
          'The downloaded app is not this Audiobooks build.',
        );
      }

      final retired = Directory(
        '${parent.path}/.audiobooks-old-${DateTime.now().microsecondsSinceEpoch}',
      );
      try {
        await installed.rename(retired.path);
      } on FileSystemException {
        throw UpdateException(
          'Need write access to ${parent.path} to install the update.',
        );
      }
      try {
        await candidate.rename(installed.path);
      } catch (_) {
        try {
          await retired.rename(installed.path);
        } catch (_) {}
        throw const UpdateException('The app bundle could not be replaced.');
      }
      try {
        await retired.delete(recursive: true);
      } catch (_) {}
      return installed;
    } finally {
      try {
        await staging.delete(recursive: true);
      } catch (_) {}
    }
  }

  static Directory _installedBundle() {
    var directory = File(Platform.resolvedExecutable).parent;
    while (true) {
      if (directory.path.endsWith('.app')) return directory;
      final parent = directory.parent;
      if (parent.path == directory.path) break;
      directory = parent;
    }
    throw const UpdateException('The installed app bundle could not be found.');
  }

  static Future<String?> _bundleIdentifier(Directory bundle) async {
    final info = File('${bundle.path}/Contents/Info.plist');
    if (!info.existsSync()) return null;
    final result = await Process.run('/usr/bin/plutil', [
      '-extract',
      'CFBundleIdentifier',
      'raw',
      info.path,
    ]);
    if (result.exitCode != 0) return null;
    return (result.stdout as String).trim();
  }

  static Future<void> _relaunch(Directory installed) async {
    await Process.start(
      '/usr/bin/open',
      ['-n', installed.path],
      mode: ProcessStartMode.detached,
    );
    exit(0);
  }

  static UpdateRelease releaseFromJson(String source) {
    final Object? decoded;
    try {
      decoded = jsonDecode(source);
    } on FormatException {
      throw const UpdateException('The release payload is not valid JSON.');
    }
    if (decoded is! Map<String, dynamic>) {
      throw const UpdateException('The release payload is not an object.');
    }

    final tag = decoded['tag_name'];
    final version = tag is String ? normalised(tag) : null;
    if (version == null) {
      throw UpdateException('Release ${tag ?? '?'} has no usable version.');
    }

    final assets = decoded['assets'];
    if (assets is! List) {
      throw UpdateException('Release $version has no $assetSuffix asset.');
    }
    Map<String, dynamic>? asset;
    for (final item in assets) {
      if (item is! Map) continue;
      final name = item['name'];
      if (name is String && name.endsWith(assetSuffix)) {
        asset = Map<String, dynamic>.from(item);
        break;
      }
    }
    if (asset == null) {
      throw UpdateException('Release $version has no $assetSuffix asset.');
    }

    final rawUrl = asset['browser_download_url'];
    final downloadUrl = rawUrl is String ? Uri.tryParse(rawUrl) : null;
    if (downloadUrl == null ||
        !downloadUrl.hasScheme ||
        !downloadUrl.hasAuthority) {
      throw UpdateException('Release $version has an invalid download link.');
    }

    final rawDigest = asset['digest'];
    final digest = rawDigest is String ? sha256Digest(rawDigest) : null;
    if (digest == null) {
      throw UpdateException(
        'Release $version publishes no sha256 digest to verify.',
      );
    }

    final rawSize = asset['size'];
    final rawNotes = decoded['body'];
    final rawPage = decoded['html_url'];
    final pageUrl = rawPage is String ? Uri.tryParse(rawPage) : null;
    final notes = rawNotes is String ? rawNotes.trim() : '';

    return UpdateRelease(
      version: version,
      downloadUrl: downloadUrl,
      digest: digest,
      pageUrl: pageUrl != null && pageUrl.hasScheme ? pageUrl : null,
      size: rawSize is num ? rawSize.toInt() : null,
      notes: notes.isEmpty ? null : notes,
    );
  }

  static String? sha256Digest(String raw) {
    final match = RegExp(
      r'^sha256:([0-9a-fA-F]{64})$',
    ).firstMatch(raw.trim());
    return match?.group(1)?.toLowerCase();
  }

  static String? normalised(String raw) {
    var body = raw.trim();
    if (body.startsWith('v')) body = body.substring(1);
    if (body.isEmpty || !RegExp(r'^[0-9.]+$').hasMatch(body)) return null;
    return body;
  }

  static bool isNewer(String candidate, String current) {
    List<int> parts(String value) => [
          for (final part in value.split('.')) int.tryParse(part) ?? 0,
        ];
    final (left, right) = (parts(candidate), parts(current));
    for (var index = 0;
        index < (left.length > right.length ? left.length : right.length);
        index++) {
      final a = index < left.length ? left[index] : 0;
      final b = index < right.length ? right[index] : 0;
      if (a != b) return a > b;
    }
    return false;
  }

  @override
  void dispose() {
    _client.close();
    super.dispose();
  }
}
