import 'dart:convert';

import 'package:http/http.dart' as http;

import 'chapter_window.dart';
import 'models.dart';

class ApiException implements Exception {
  ApiException(this.message);

  final String message;

  @override
  String toString() => message;
}

/// A short, human-readable reason for a failed request.
///
/// The raw exception text names sockets, hosts and URIs, which belongs in a
/// log rather than on screen.
String describeError(Object error) {
  final text = error.toString();
  final lower = text.toLowerCase();
  final status = RegExp(r'Server returned (\d{3})').firstMatch(text)?.group(1);
  if (status != null) {
    return 'The server returned error $status. Please try again.';
  }
  if (lower.contains('handshake')) {
    return 'Secure connection failed. Check your network and try again.';
  }
  if (lower.contains('failed host lookup') ||
      lower.contains('socketexception') ||
      lower.contains('clientexception')) {
    return 'Can’t reach the server. Check your connection and try again.';
  }
  if (lower.contains('timeout') || lower.contains('timed out')) {
    return 'The server took too long to respond. Try again.';
  }
  return 'Something went wrong. Please try again.';
}

class AudiobookApi {
  AudiobookApi({Uri? base, http.Client? client})
    : base = base ?? Uri.parse(defaultApiBase),
      _client = client ?? http.Client();

  static const int pageSize = 24;

  final Uri base;
  final http.Client _client;

  Future<AudiobookListPage> list({
    String? term,
    String? tag,
    int limit = pageSize,
    int offset = 0,
  }) async {
    final query = <String, String>{
      'limit': '$limit',
      'offset': '$offset',
      if (term != null && term.isNotEmpty) 'term': term,
      if (tag != null && tag.isNotEmpty) 'tag': tag,
    };
    final json = await _getJson(
      base.replace(
        path: '${base.path}/audiobooks/public/all'.replaceAll('//', '/'),
        queryParameters: query,
      ),
    );
    return AudiobookListPage.fromJson(json);
  }

  Future<AudiobookDetails> details(
    String slug, {
    int tracksOffset = 0,
    int tracksLimit = ChapterWindow.size,
  }) async {
    final json = await _getJson(
      base.replace(
        path:
            '${base.path}/audiobooks/public/s/$slug'.replaceAll('//', '/'),
        queryParameters: {
          'tracks_offset': '$tracksOffset',
          'tracks_limit': '$tracksLimit',
        },
      ),
    );
    final envelope = json['audiobook'] as Map<String, dynamic>?;
    if (envelope == null) {
      throw ApiException('Unexpected response format: no audiobook');
    }
    return AudiobookDetails.fromJson(envelope);
  }

  Future<int?> recordPlay({
    required int audiobookId,
    required int trackId,
  }) async {
    final response = await _client.post(
      base.replace(
        path:
            '${base.path}/audiobooks/id/$audiobookId/tracks/$trackId/play'
                .replaceAll('//', '/'),
      ),
    );
    if (response.statusCode == 204) return null;
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw ApiException('Server returned ${response.statusCode}');
    }
    try {
      final json = jsonDecode(response.body) as Map<String, dynamic>;
      return (json['play_count'] as num).toInt();
    } catch (_) {
      return null;
    }
  }

  Future<Map<String, dynamic>> _getJson(Uri uri) async {
    final response = await _client.get(uri);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw ApiException('Server returned ${response.statusCode}');
    }
    try {
      return jsonDecode(utf8.decode(response.bodyBytes)) as Map<String, dynamic>;
    } catch (error) {
      throw ApiException('Unexpected response format: $error');
    }
  }
}
