import 'dart:convert';

import 'package:audiobooks/api.dart';
import 'package:audiobooks/player_controller.dart';
import 'package:audiobooks/progress_store.dart';
import 'package:audiobooks/screens/library_screen.dart';
import 'package:audiobooks/theme.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('library renders books from the feed', (tester) async {
    SharedPreferences.setMockInitialValues({});
    final progress = ProgressStore();
    await progress.load();

    final client = MockClient((request) async {
      return http.Response(
        jsonEncode({
          'audiobooks': [
            {
              'id': 1,
              'title': 'Test Book One',
              'slug': 'test-book-one',
              'description': '',
              'status': 'published',
              'url': 'media/i/cover-one',
              'track_count': 12,
              'total_duration_seconds': 3600,
              'tags': ['Tien Hiep'],
              'tag_slugs': ['tien-hiep'],
              'created_at': '2026-01-01T00:00:00Z',
            },
            {
              'id': 2,
              'title': 'Test Book Two',
              'slug': 'test-book-two',
              'description': '',
              'status': 'published',
              'url': 'media/i/cover-two',
              'track_count': 3,
              'total_duration_seconds': 600,
              'tags': [],
              'tag_slugs': [],
              'created_at': '2026-01-02T00:00:00Z',
            },
          ],
          'has_more': false,
        }),
        200,
        headers: {'content-type': 'application/json'},
      );
    });

    final api = AudiobookApi(
      base: Uri.parse('https://api.example.test'),
      client: client,
    );
    final player = PlayerController(api: api, progress: progress);
    addTearDown(player.dispose);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.build(),
        home: Scaffold(
          backgroundColor: AppTheme.page,
          body: LibraryScreen(
            api: api,
            player: player,
            onOpenBook: (_) {},
            onOpenPlayer: () {},
            onOpenAbout: () {},
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Audiobooks'), findsOneWidget);
    expect(find.text('Test Book One'), findsOneWidget);
    expect(find.text('Test Book Two'), findsOneWidget);
    expect(find.text('12 chapters'), findsOneWidget);
    expect(find.textContaining('#Tien Hiep'), findsOneWidget);
  });

  testWidgets('library shows the failure state and retries', (tester) async {
    SharedPreferences.setMockInitialValues({});
    final progress = ProgressStore();
    await progress.load();

    var calls = 0;
    final client = MockClient((request) async {
      calls += 1;
      if (calls == 1) return http.Response('down', 503);
      return http.Response(
        jsonEncode({'audiobooks': [], 'has_more': false}),
        200,
      );
    });
    final api = AudiobookApi(
      base: Uri.parse('https://api.example.test'),
      client: client,
    );
    final player = PlayerController(api: api, progress: progress);
    addTearDown(player.dispose);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.build(),
        home: Scaffold(
          backgroundColor: AppTheme.page,
          body: LibraryScreen(
            api: api,
            player: player,
            onOpenBook: (_) {},
            onOpenPlayer: () {},
            onOpenAbout: () {},
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('503'), findsOneWidget);

    await tester.tap(find.text('Try again'));
    await tester.pumpAndSettle();

    expect(find.text('No published audiobooks yet.'), findsOneWidget);
  });
}