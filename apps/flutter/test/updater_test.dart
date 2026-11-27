import 'dart:convert';

import 'package:audiobooks/screens/about_screen.dart';
import 'package:audiobooks/updater.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

Map<String, Object?> _payload({
  String tag = 'v0.11.0',
  String asset = 'Audiobooks-0.11.0.app.zip',
  String? digest =
      'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
  int? size = 680000,
  String? body = '\n\n  What changed  \n\n',
}) {
  return {
    'tag_name': tag,
    'html_url': 'https://github.com/lhuthng/my-page/releases/tag/$tag',
    'body': body,
    'assets': [
      {
        'name': 'checksums.txt',
        'browser_download_url': 'https://example.com/checksums.txt',
      },
      {
        'name': asset,
        'browser_download_url': 'https://example.com/$asset',
        if (digest != null) 'digest': 'sha256:$digest',
        if (size != null) 'size': size,
      },
    ],
  };
}

UpdateController _controller(String currentVersion, http.Client client) {
  return UpdateController(
    currentVersion: currentVersion,
    client: client,
    latestReleaseUri: Uri.parse('https://example.test/releases/latest'),
  );
}

void main() {
  test('release payload reads the app zip and its digest', () {
    final release = UpdateController.releaseFromJson(
      jsonEncode(_payload()),
    );

    expect(release.version, '0.11.0');
    expect(
      release.downloadUrl.toString(),
      'https://example.com/Audiobooks-0.11.0.app.zip',
    );
    expect(release.digest, List.filled(64, 'a').join());
    expect(release.size, 680000);
    expect(release.notes, 'What changed');
    expect(
      release.pageUrl.toString(),
      'https://github.com/lhuthng/my-page/releases/tag/v0.11.0',
    );
  });

  test('release payload rejects a release without an app zip', () {
    final payload = _payload();
    payload['assets'] = [
      {
        'name': 'checksums.txt',
        'browser_download_url': 'https://example.com/checksums.txt',
      },
    ];

    expect(
      () => UpdateController.releaseFromJson(jsonEncode(payload)),
      throwsA(isA<UpdateException>()),
    );
  });

  test('release payload rejects a missing or malformed digest', () {
    expect(
      () => UpdateController.releaseFromJson(
        jsonEncode(_payload(digest: null)),
      ),
      throwsA(isA<UpdateException>()),
    );
    expect(
      () => UpdateController.releaseFromJson(
        jsonEncode(_payload(digest: 'not-hex')),
      ),
      throwsA(isA<UpdateException>()),
    );
  });

  test('version helpers normalise tags and compare numerically', () {
    expect(UpdateController.normalised('v0.2.0'), '0.2.0');
    expect(UpdateController.normalised(' 0.2.0\n'), '0.2.0');
    expect(UpdateController.normalised('release-2026'), isNull);
    expect(UpdateController.isNewer('0.10.0', '0.9.0'), isTrue);
    expect(UpdateController.isNewer('0.5.0', '0.5.0'), isFalse);
  });

  test('check reports an available release', () async {
    final updater = _controller(
      '0.10.0',
      MockClient((_) async => http.Response(jsonEncode(_payload()), 200)),
    );
    addTearDown(updater.dispose);

    await updater.check();

    expect(updater.phase, UpdatePhase.available);
    expect(updater.release?.version, '0.11.0');
  });

  test('check reports an up-to-date app', () async {
    final updater = _controller(
      '0.11.0',
      MockClient((_) async => http.Response(jsonEncode(_payload()), 200)),
    );
    addTearDown(updater.dispose);

    await updater.check();

    expect(updater.phase, UpdatePhase.upToDate);
    expect(updater.release, isNull);
  });

  test('check surfaces a GitHub failure', () async {
    final updater = _controller(
      '0.10.0',
      MockClient((_) async => http.Response('missing', 404)),
    );
    addTearDown(updater.dispose);

    await updater.check();

    expect(updater.phase, UpdatePhase.failed);
    expect(updater.message, contains('404'));
  });

  testWidgets('about shows an available update', (tester) async {
    tester.view.physicalSize = const Size(384, 520);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final updater = _controller(
      '0.10.0',
      MockClient((_) async => http.Response(jsonEncode(_payload()), 200)),
    );
    addTearDown(updater.dispose);
    await updater.check();

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AboutScreen(updater: updater, onBack: () {}),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.drag(find.byType(ListView), const Offset(0, -500));
    await tester.pumpAndSettle();

    expect(find.text('Version 0.11.0 is available'), findsOneWidget);
    expect(find.text('Install and restart'), findsOneWidget);
    expect(find.textContaining('to download'), findsOneWidget);
    expect(find.text('What changed'), findsOneWidget);
  });
}
