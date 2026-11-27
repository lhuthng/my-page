// ignore_for_file: avoid_print

import 'dart:io';

import 'package:audiobooks/api.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  final enabled = Platform.environment['LIVE_SMOKE'] == '1';

  test(
    'the production API serves a list and a book window',
    () async {
      final api = AudiobookApi();

      final page = await api.list(limit: 5);
      print('library: ${page.audiobooks.length} books, hasMore=${page.hasMore}');
      expect(page.audiobooks, isNotEmpty);
      final first = page.audiobooks.first;
      print('first: ${first.title} (${first.trackCount} chapters)');

      final details = await api.details(first.slug, tracksLimit: 3);
      print(
        'open "${details.title}": total=${details.totalTracks}, '
        'window=${details.tracks.length}',
      );
      expect(details.tracks, isNotEmpty);
      expect(details.tracks.first.streamUrl, startsWith('https://'));
      expect(details.totalTracks, greaterThanOrEqualTo(details.tracks.length));
    },
    skip: enabled ? false : 'set LIVE_SMOKE=1 to hit the production API',
    timeout: const Timeout(Duration(seconds: 30)),
  );
}
