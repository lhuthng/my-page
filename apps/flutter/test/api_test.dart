import 'dart:convert';

import 'package:audiobooks/api.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  final listBody = jsonEncode({
    'audiobooks': [
      {
        'id': 1,
        'title': 'Test Book',
        'slug': 'test-book',
        'description': '',
        'status': 'published',
        'url': 'media/i/cover',
        'track_count': 42,
        'total_duration_seconds': 7200,
        'tags': ['Tien Hiep'],
        'tag_slugs': ['tien-hiep'],
        'created_at': '2026-01-01T00:00:00Z',
      },
    ],
    'has_more': true,
  });

  test('list sends paging and filter params', () async {
    late Uri seen;
    final client = MockClient((request) async {
      seen = request.url;
      return http.Response(listBody, 200, headers: {'content-type': 'application/json'});
    });
    final api = AudiobookApi(
      base: Uri.parse('https://api.example.test'),
      client: client,
    );

    final page = await api.list(term: 'tu tiên', tag: 'tien-hiep', offset: 24);

    expect(seen.path, '/audiobooks/public/all');
    expect(seen.queryParameters['limit'], '${AudiobookApi.pageSize}');
    expect(seen.queryParameters['offset'], '24');
    expect(seen.queryParameters['term'], 'tu tiên');
    expect(seen.queryParameters['tag'], 'tien-hiep');
    expect(page.audiobooks.single.trackCount, 42);
    expect(page.hasMore, isTrue);
  });

  test('details unwraps the envelope and asks for a window', () async {
    late Uri seen;
    final client = MockClient((request) async {
      seen = request.url;
      return http.Response(
        jsonEncode({
          'audiobook': {
            'id': 1,
            'title': 'Test Book',
            'slug': 'test-book',
            'description': '',
            'status': 'published',
            'total_duration_seconds': 100,
            'owner_username': 'owner',
            'owner_display_name': 'Owner',
            'tags': [],
            'tracks': [],
            'track_count': 42,
            'has_more_tracks': true,
          },
        }),
        200,
      );
    });
    final api = AudiobookApi(
      base: Uri.parse('https://api.example.test'),
      client: client,
    );

    final details = await api.details('test-book', tracksOffset: 20);

    expect(seen.path, '/audiobooks/public/s/test-book');
    expect(seen.queryParameters['tracks_offset'], '20');
    expect(seen.queryParameters['tracks_limit'], '20');
    expect(details.totalTracks, 42);
    expect(details.hasMoreChapters, isTrue);
  });

  test('a 204 play beacon counts as not counted', () async {
    final client = MockClient((_) async => http.Response('', 204));
    final api = AudiobookApi(
      base: Uri.parse('https://api.example.test'),
      client: client,
    );

    expect(await api.recordPlay(audiobookId: 1, trackId: 2), isNull);
  });

  test('network failures read as plain sentences', () {
    expect(
      describeError(Exception(
        'ClientException with SocketException: Failed host lookup: api.example',
      )),
      contains('Can’t reach the server'),
    );
    expect(
      describeError(Exception('HandshakeException: Connection terminated')),
      contains('Secure connection failed'),
    );
    expect(
      describeError(ApiException('Server returned 503')),
      contains('503'),
    );
    expect(
      describeError(StateError('odd')),
      'Something went wrong. Please try again.',
    );
  });

  test('errors surface the status code', () async {
    final client = MockClient((_) async => http.Response('nope', 503));
    final api = AudiobookApi(
      base: Uri.parse('https://api.example.test'),
      client: client,
    );

    await expectLater(
      api.list(),
      throwsA(
        isA<ApiException>().having(
          (e) => e.message,
          'message',
          contains('503'),
        ),
      ),
    );
  });
}
