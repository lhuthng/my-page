import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';

class PlaybackProgress {
  const PlaybackProgress({
    required this.trackId,
    required this.trackIndex,
    required this.time,
    required this.rate,
    required this.updatedAt,
  });

  factory PlaybackProgress.fromJson(Map<String, dynamic> json) {
    return PlaybackProgress(
      trackId: (json['track_id'] as num).toInt(),
      trackIndex: (json['track_index'] as num).toInt(),
      time: (json['time'] as num).toDouble(),
      rate: (json['rate'] as num?)?.toDouble() ?? 1,
      updatedAt:
          DateTime.tryParse(json['updated_at'] as String? ?? '') ??
          DateTime.now(),
    );
  }

  final int trackId;
  final int trackIndex;
  final double time;
  final double rate;
  final DateTime updatedAt;

  Map<String, dynamic> toJson() => {
    'track_id': trackId,
    'track_index': trackIndex,
    'time': time,
    'rate': rate,
    'updated_at': updatedAt.toIso8601String(),
  };
}

class ProgressStore {
  static const String _key = 'playback_progress_v1';

  final Map<String, PlaybackProgress> _progress = {};
  SharedPreferences? _prefs;

  Future<void> load() async {
    _prefs = await SharedPreferences.getInstance();
    final raw = _prefs?.getString(_key);
    if (raw == null) return;
    try {
      final decoded = jsonDecode(raw) as Map<String, dynamic>;
      for (final entry in decoded.entries) {
        final value = entry.value as Map<String, dynamic>;
        _progress[entry.key] = PlaybackProgress.fromJson(value);
      }
    } catch (_) {
      // A corrupt payload costs the saved positions, not the app.
    }
  }

  PlaybackProgress? get(String slug) => _progress[slug];

  Future<void> save(String slug, PlaybackProgress progress) async {
    _progress[slug] = progress;
    final prefs = _prefs ??= await SharedPreferences.getInstance();
    await prefs.setString(
      _key,
      jsonEncode(_progress.map((key, value) => MapEntry(key, value.toJson()))),
    );
  }
}
