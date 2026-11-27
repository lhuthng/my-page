import 'dart:convert';
import 'dart:io';

import 'package:audiobooks/chapter_window.dart';
import 'package:audiobooks/models.dart';
import 'package:flutter_test/flutter_test.dart';

Map<String, dynamic> _fixture(String name) {
  final file = File('test/fixtures/$name');
  return jsonDecode(file.readAsStringSync()) as Map<String, dynamic>;
}

void main() {
  group('list payload', () {
    test('parses the public feed', () {
      final page = AudiobookListPage.fromJson(_fixture('list.json'));

      expect(page.audiobooks, isNotEmpty);
      expect(page.hasMore, isA<bool>());

      final book = page.audiobooks.first;
      expect(book.id, greaterThan(0));
      expect(book.title, isNotEmpty);
      expect(book.slug, isNotEmpty);
      expect(book.trackCount, greaterThan(0));
      expect(book.coverUrl, startsWith('https://'));
      expect(book.coverUrl, contains('/media/i/'));
    });

    test('media urls pass absolute paths through', () {
      expect(mediaUrl('https://cdn.example/a.mp3'), 'https://cdn.example/a.mp3');
      expect(mediaUrl(null), isNull);
      expect(mediaUrl(''), isNull);
      expect(
        mediaUrl('media/i/x', base: 'https://api.example.test'),
        'https://api.example.test/media/i/x',
      );
    });
  });

  group('details payload', () {
    test('parses the windowed book and its tracks', () {
      final envelope = _fixture('details.json');
      final book = AudiobookDetails.fromJson(
        envelope['audiobook'] as Map<String, dynamic>,
      );

      expect(book.tracks, isNotEmpty);
      expect(book.tracks.length, lessThanOrEqualTo(ChapterWindow.size));
      expect(book.totalTracks, greaterThanOrEqualTo(book.tracks.length));
      expect(book.tracks.first.streamUrl, startsWith('https://'));
      expect(book.tracks.first.number, 1);
      expect(book.tracks.first.durationSeconds, isNotNull);
      expect(book.tags, isNotEmpty);
    });

    test('a window short of the total reports more chapters', () {
      final envelope = _fixture('details.json');
      final book = AudiobookDetails.fromJson(
        envelope['audiobook'] as Map<String, dynamic>,
      );
      if (book.tracks.length < book.totalTracks) {
        expect(book.hasMoreChapters, isTrue);
      }
    });
  });

  group('chapter windows', () {
    test('windows start on multiples of the size', () {
      expect(ChapterWindow.start(0), 0);
      expect(ChapterWindow.start(19), 0);
      expect(ChapterWindow.start(20), 20);
      expect(ChapterWindow.start(39), 20);
      expect(ChapterWindow.start(40), 40);
    });

    test('starts cover a book without gaps', () {
      expect(ChapterWindow.starts(0), isEmpty);
      expect(ChapterWindow.starts(20), [0]);
      expect(ChapterWindow.starts(21), [0, 20]);
      expect(ChapterWindow.starts(61), [0, 20, 40, 60]);
    });

    test('the last window is clamped to the book', () {
      expect(ChapterWindow.count(0, 45), 20);
      expect(ChapterWindow.count(40, 45), 5);
      expect(ChapterWindow.count(40, 40), 0);
      expect(ChapterWindow.count(0, 7), 7);
    });
  });

  group('formatting', () {
    test('clocks mirror the web format', () {
      expect(formatClock(0), '0:00');
      expect(formatClock(65), '1:05');
      expect(formatClock(3661), '1:01:01');
      expect(formatClock(double.nan), '--:--');
    });

    test('durations read as english labels', () {
      expect(formatDurationLabel(0), '');
      expect(formatDurationLabel(30), 'under a minute');
      expect(formatDurationLabel(60 * 12), '12 min');
      expect(formatDurationLabel(3600), '1 hr');
      expect(formatDurationLabel(3600 + 60 * 5), '1 hr 5 min');
    });

    test('counts group thousands', () {
      expect(formatCount(0), '0');
      expect(formatCount(999), '999');
      expect(formatCount(1152), '1,152');
      expect(formatCount(1234567), '1,234,567');
    });
  });
}
