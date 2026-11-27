import 'package:audio_service/audio_service.dart';

import 'player_controller.dart';

class AudiobookAudioHandler extends BaseAudioHandler {
  AudiobookAudioHandler(this._controller) {
    _controller.addListener(_sync);
    _sync();
  }

  final PlayerController _controller;
  bool _stopped = false;
  String? _itemSignature;
  AudioProcessingState? _lastProcessingState;
  bool? _lastPlaying;
  double? _lastSpeed;
  int? _lastTrackIndex;
  DateTime? _lastBroadcast;

  @override
  Future<void> play() {
    _stopped = false;
    return _controller.play();
  }

  @override
  Future<void> pause() => _controller.pause();

  @override
  Future<void> stop() async {
    _stopped = true;
    await _controller.stop();
  }

  @override
  Future<void> click([MediaButton button = MediaButton.media]) {
    return switch (button) {
      MediaButton.media => _controller.togglePlayPause(),
      MediaButton.next => _controller.next(),
      MediaButton.previous => _controller.previous(),
    };
  }

  @override
  Future<void> seek(Duration position) =>
      _controller.seek(position.inMilliseconds / 1000);

  @override
  Future<void> skipToNext() => _controller.next();

  @override
  Future<void> skipToPrevious() => _controller.previous();

  @override
  Future<void> setSpeed(double speed) => _controller.setRate(speed);

  @override
  Future<void> fastForward() => _controller.skip(30);

  @override
  Future<void> rewind() => _controller.skip(-15);

  void _sync() {
    if (_controller.isPlaying) _stopped = false;
    final track = _controller.currentTrack;
    final book = _controller.book;
    if (_stopped || track == null || book == null) {
      final hadItem = _itemSignature != null;
      if (hadItem) {
        _itemSignature = null;
        mediaItem.add(null);
      }
      if (hadItem ||
          _lastProcessingState != AudioProcessingState.idle ||
          _lastPlaying != false) {
        _broadcast(
          PlaybackState(
            processingState: AudioProcessingState.idle,
            playing: false,
            controls: const [],
            systemActions: const {},
            updatePosition: Duration.zero,
            bufferedPosition: Duration.zero,
            speed: 1,
            updateTime: DateTime.now(),
          ),
        );
      }
      return;
    }

    final cover = book.coverUrl;
    final item = MediaItem(
      id: track.id.toString(),
      title: book.title,
      album: book.title,
      artist: book.translator ?? 'Audiobooks',
      duration: track.durationSeconds == null
          ? null
          : Duration(seconds: track.durationSeconds!),
      artUri: cover == null ? null : Uri.parse(cover),
      displayTitle: book.title,
      displaySubtitle: 'Chapter ${track.number} · ${track.title}',
    );
    final signature = _signature(item);
    final itemChanged = signature != _itemSignature;
    if (itemChanged) {
      _itemSignature = signature;
      mediaItem.add(item);
    }

    final controls = <MediaControl>[
      if (_controller.hasPrevious) MediaControl.skipToPrevious,
      MediaControl.rewind,
      _controller.isPlaying ? MediaControl.pause : MediaControl.play,
      MediaControl.fastForward,
      if (_controller.hasNext) MediaControl.skipToNext,
    ];
    final processingState = _controller.opening
        ? AudioProcessingState.loading
        : _controller.error != null
            ? AudioProcessingState.error
            : _controller.duration > 0 &&
                    _controller.position >= _controller.duration
                ? AudioProcessingState.completed
                : AudioProcessingState.ready;
    final state = PlaybackState(
      processingState: processingState,
      playing: _controller.isPlaying,
      controls: controls,
      androidCompactActionIndices: [
        if (_controller.hasPrevious) 0,
        2,
        if (_controller.hasNext) controls.length - 1,
      ],
      systemActions: const {
        MediaAction.seek,
        MediaAction.seekForward,
        MediaAction.seekBackward,
      },
      updatePosition: Duration(
        milliseconds: (_controller.position * 1000).round(),
      ),
      bufferedPosition: Duration.zero,
      speed: _controller.rate,
      updateTime: DateTime.now(),
      errorMessage: _controller.error,
      queueIndex: _controller.trackIndex,
    );

    final changed = itemChanged ||
        processingState != _lastProcessingState ||
        state.playing != _lastPlaying ||
        state.speed != _lastSpeed ||
        state.queueIndex != _lastTrackIndex;
    final elapsed = _lastBroadcast == null
        ? const Duration(days: 1)
        : DateTime.now().difference(_lastBroadcast!);
    if (changed || elapsed >= const Duration(seconds: 1)) {
      _broadcast(state);
    }
  }

  void _broadcast(PlaybackState state) {
    playbackState.add(state);
    _lastProcessingState = state.processingState;
    _lastPlaying = state.playing;
    _lastSpeed = state.speed;
    _lastTrackIndex = state.queueIndex;
    _lastBroadcast = DateTime.now();
  }

  String _signature(MediaItem item) =>
      '${item.id}|${item.title}|${item.album}|${item.artist}|'
      '${item.duration?.inMilliseconds}|${item.artUri}|'
      '${item.displayTitle}|${item.displaySubtitle}';
}
