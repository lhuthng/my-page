import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:media_kit/media_kit.dart';

import 'api.dart';
import 'chapter_store.dart';
import 'models.dart';
import 'progress_store.dart';

class PlayerController extends ChangeNotifier {
  PlayerController({required this.api, required this.progress});

  static const List<double> rates = [0.75, 1.0, 1.25, 1.5, 1.75, 2.0];
  static const double _reportIntervalSeconds = 10;
  static const double _saveIntervalSeconds = 4;

  final AudiobookApi api;
  final ProgressStore progress;

  Player? _playerInstance;

  Player get player => _playerInstance ??= _createPlayer();

  bool get _playerReady => _playerInstance != null;

  Player _createPlayer() {
    final player = Player();
    _subscriptions.add(
      player.stream.playing.listen((playing) {
        _isPlaying = playing;
        _syncListeningTimer();
        if (!playing) _saveProgress();
        notifyListeners();
      }),
    );
    _subscriptions.add(
      player.stream.position.listen((value) {
        if (_scrubbing) return;
        _position = value.inMilliseconds / 1000;
        _maybeSaveProgress();
        notifyListeners();
      }),
    );
    _subscriptions.add(
      player.stream.duration.listen((value) {
        if (value > Duration.zero) {
          _duration = value.inMilliseconds / 1000;
          notifyListeners();
        }
      }),
    );
    _subscriptions.add(
      player.stream.completed.listen((completed) {
        if (completed) _trackDidEnd();
      }),
    );
    _subscriptions.add(
      player.stream.error.listen((message) {
        _error = 'Playback failed. Try again.';
        notifyListeners();
      }),
    );
    return player;
  }

  final List<StreamSubscription<dynamic>> _subscriptions = [];

  AudiobookSummary? _summary;
  AudiobookDetails? _book;
  ChapterStore? _chapters;
  bool _opening = false;
  String? _error;
  int _trackIndex = 0;
  bool _isPlaying = false;
  double _position = 0;
  double _duration = 0;
  double _rate = 1;
  double _lastSaved = 0;
  double _listened = 0;
  Timer? _listeningTimer;
  int _lastBeaconTrackId = 0;
  double? _resumeTime;
  int? _resumeChapter;
  bool _scrubbing = false;
  double _scrubTarget = 0;

  AudiobookSummary? get summary => _summary;
  AudiobookDetails? get book => _book;
  ChapterStore? get chapters => _chapters;
  bool get opening => _opening;
  String? get error => _error;
  int get trackIndex => _trackIndex;
  bool get isPlaying => _isPlaying;
  double get position => _position;
  double get duration => _duration;
  double get rate => _rate;
  double? get resumeTime => _resumeTime;
  int? get resumeChapter => _resumeChapter;
  bool get hasResumePrompt => _resumeTime != null;
  int get chapterCount => _chapters?.total ?? 0;
  AudiobookTrack? get currentTrack => _chapters?.track(_trackIndex);
  bool get hasPrevious => _book != null && _trackIndex > 0;
  bool get hasNext => _book != null && _trackIndex < chapterCount - 1;

  Future<void> open(AudiobookSummary summary) async {
    if (_summary?.id == summary.id) return;
    _opening = true;
    _error = null;
    _summary = summary;
    _position = 0;
    _duration = 0;
    _clearResumePrompt();
    notifyListeners();

    try {
      final details = await api.details(summary.slug);
      if (_summary?.id != summary.id) return;
      _book = details;
      _chapters?.removeListener(notifyListeners);
      final store = ChapterStore(
        slug: details.slug,
        total: details.totalTracks,
        api: api,
        seed: details.tracks,
      );
      store.addListener(notifyListeners);
      _chapters = store;

      var index = 0;
      double? resumeAt;
      final saved = progress.get(details.slug);
      if (saved != null) {
        final match = details.tracks.indexWhere((t) => t.id == saved.trackId);
        index = match >= 0
            ? match
            : store.total == 0
            ? 0
            : saved.trackIndex.clamp(0, store.total - 1);
        resumeAt = saved.time > 3 ? saved.time : null;
        if (saved.rate >= 0.5 && saved.rate <= 3) _rate = saved.rate;
      }
      _trackIndex = index;
      _opening = false;
      await _load(autoplay: false, resumeAt: resumeAt);
      if (resumeAt != null) {
        _resumeTime = resumeAt;
        _resumeChapter = _chapterNumberAt(index);
        notifyListeners();
      }
    } catch (error) {
      if (_summary?.id != summary.id) return;
      _opening = false;
      _error = describeError(error);
      notifyListeners();
    }
  }

  int _chapterNumberAt(int index) {
    final number = _chapters?.track(index)?.number;
    return number ?? index + 1;
  }

  Future<void> resumeFromSavedPoint() async {
    final time = _resumeTime;
    if (time == null) return;
    _clearResumePrompt();
    await seek(time);
    await player.play();
    notifyListeners();
  }

  Future<void> restartFromBeginning() async {
    if (_resumeTime == null) return;
    _clearResumePrompt();
    _trackIndex = 0;
    await _load(autoplay: false);
  }

  void dismissResumePrompt() {
    if (_resumeTime == null) return;
    _clearResumePrompt();
    notifyListeners();
  }

  void _clearResumePrompt() {
    _resumeTime = null;
    _resumeChapter = null;
  }

  Future<void> retry() async {
    final summary = _summary;
    if (summary == null) return;
    _summary = null;
    await open(summary);
  }

  Future<void> playTrack(int index) async {
    if (_book == null || index < 0 || index >= chapterCount) return;
    _clearResumePrompt();
    _trackIndex = index;
    await _load(autoplay: true);
  }

  Future<void> togglePlayPause() async {
    if (currentTrack == null) return;
    if (_isPlaying) {
      await player.pause();
    } else {
      _clearResumePrompt();
      await player.play();
    }
  }

  Future<void> previous() async {
    if (!hasPrevious) return;
    _trackIndex -= 1;
    await _load(autoplay: true);
  }

  Future<void> next() async {
    if (!hasNext) return;
    _trackIndex += 1;
    await _load(autoplay: true);
  }

  Future<void> skip(double delta) async {
    await seek(_position + delta);
  }

  Future<void> seek(double seconds) async {
    final double clamped = seconds
        .clamp(0, _duration > 0 ? _duration : 0)
        .toDouble();
    _position = clamped;
    _lastSaved = clamped;
    notifyListeners();
    await player.seek(Duration(milliseconds: (clamped * 1000).round()));
    _saveProgress();
  }

  void scrubPreview(double seconds) {
    _scrubbing = true;
    _scrubTarget = seconds
        .clamp(0, _duration > 0 ? _duration : 0)
        .toDouble();
    _position = _scrubTarget;
    notifyListeners();
  }

  Future<void> scrubCommit(double seconds) async {
    _scrubbing = false;
    await seek(seconds);
  }

  Future<void> setRate(double rate) async {
    _rate = rate;
    await player.setRate(rate);
    _saveProgress();
    notifyListeners();
  }

  Future<void> cycleRate() async {
    final index = rates.indexOf(_rate);
    final nextRate = rates[(index < 0 ? 1 : index + 1) % rates.length];
    await setRate(nextRate);
  }

  void refreshChapters() {
    _chapters?.refresh();
  }

  Future<void> _load({required bool autoplay, double? resumeAt}) async {
    if (autoplay) _clearResumePrompt();
    final store = _chapters;
    if (store == null) return;
    await store.ensure(_trackIndex);
    final track = store.track(_trackIndex);
    final url = track?.streamUrl;
    if (track == null || url == null) {
      _error = 'This chapter is not available yet.';
      notifyListeners();
      return;
    }

    _error = null;
    _position = resumeAt ?? 0;
    _lastSaved = _position;
    _duration = track.durationSeconds?.toDouble() ?? 0;
    _lastBeaconTrackId = track.id;
    _listened = 0;
    notifyListeners();

    await player.open(
      Media(
        url,
        start: resumeAt == null
            ? null
            : Duration(milliseconds: (resumeAt * 1000).round()),
      ),
      play: false,
    );
    await player.setRate(_rate);
    if (autoplay) await player.play();

    store.prefetch(_trackIndex);
    _saveProgress();
    notifyListeners();
  }

  void _trackDidEnd() {
    if (_book == null) return;
    if (hasNext) {
      _trackIndex += 1;
      unawaited(_load(autoplay: true));
    } else {
      _isPlaying = false;
      _position = _duration;
      _saveProgress();
      notifyListeners();
    }
  }

  void _syncListeningTimer() {
    if (_isPlaying) {
      _listeningTimer ??= Timer.periodic(const Duration(seconds: 1), (_) {
        _listened += 1;
        if (_listened >= _reportIntervalSeconds) {
          _listened -= _reportIntervalSeconds;
          _reportPlay();
        }
      });
    } else {
      _listeningTimer?.cancel();
      _listeningTimer = null;
    }
  }

  void _reportPlay() {
    final book = _book;
    final track = currentTrack;
    if (book == null || track == null) return;
    if (track.id != _lastBeaconTrackId) return;
    api
        .recordPlay(audiobookId: book.id, trackId: track.id)
        .then((total) {
          if (total != null) _chapters?.setPlayCount(track.id, total);
        })
        .ignore();
  }

  void _maybeSaveProgress() {
    if ((_position - _lastSaved).abs() < _saveIntervalSeconds) return;
    _lastSaved = _position;
    _saveProgress();
  }

  void _saveProgress() {
    final book = _book;
    final track = currentTrack;
    if (book == null || track == null) return;
    progress.save(
      book.slug,
      PlaybackProgress(
        trackId: track.id,
        trackIndex: _trackIndex,
        time: _position,
        rate: _rate,
        updatedAt: DateTime.now(),
      ),
    );
  }

  @override
  void dispose() {
    _saveProgress();
    for (final subscription in _subscriptions) {
      subscription.cancel();
    }
    _chapters?.removeListener(notifyListeners);
    _listeningTimer?.cancel();
    if (_playerReady) _playerInstance!.dispose();
    super.dispose();
  }
}
