import 'package:flutter/foundation.dart';

import 'api.dart';
import 'chapter_window.dart';
import 'models.dart';

class ChapterStore extends ChangeNotifier {
  ChapterStore({
    required this.slug,
    required this.total,
    required this.api,
    List<AudiobookTrack> seed = const [],
  }) {
    for (var i = 0; i < seed.length; i++) {
      _byIndex[i] = seed[i];
    }
  }

  final String slug;
  final AudiobookApi api;
  int total;

  final Map<int, AudiobookTrack> _byIndex = {};
  final Map<int, Future<bool>> _inflight = {};

  AudiobookTrack? track(int index) => _byIndex[index];

  bool isLoaded(int index) => _byIndex.containsKey(index);

  Future<bool> ensure(int index) {
    if (index < 0 || index >= total) return Future.value(false);
    if (_byIndex.containsKey(index)) return Future.value(true);
    final start = ChapterWindow.start(index);
    return _inflight.putIfAbsent(start, () => _fetchWindow(start));
  }

  void prefetch(int index) {
    for (var ahead = 1; ahead <= ChapterWindow.prefetchAhead; ahead++) {
      final target = index + ahead;
      if (target < total && !_byIndex.containsKey(target)) {
        ensure(target);
      }
    }
  }

  Future<void> refresh() async {
    try {
      final details = await api.details(slug, tracksOffset: 0);
      total = details.totalTracks;
      for (var i = 0; i < details.tracks.length; i++) {
        _byIndex[i] = details.tracks[i];
      }
      notifyListeners();
    } catch (_) {
      // A refresh that fails leaves the loaded chapters in place.
    }
  }

  void setPlayCount(int trackId, int count) {
    for (final track in _byIndex.values) {
      if (track.id == trackId) {
        track.playCount = count;
        notifyListeners();
        return;
      }
    }
  }

  Future<bool> _fetchWindow(int start) async {
    try {
      final details = await api.details(
        slug,
        tracksOffset: start,
        tracksLimit: ChapterWindow.size,
      );
      total = details.totalTracks;
      for (var i = 0; i < details.tracks.length; i++) {
        _byIndex[start + i] = details.tracks[i];
      }
      notifyListeners();
      return true;
    } catch (_) {
      return false;
    } finally {
      _inflight.remove(start);
    }
  }
}
