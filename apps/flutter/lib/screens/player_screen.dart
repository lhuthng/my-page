import 'package:flutter/material.dart';

import '../models.dart';
import '../player_controller.dart';
import '../theme.dart';
import '../widgets/controls.dart';

class PlayerScreen extends StatelessWidget {
  const PlayerScreen({super.key, required this.player, required this.onBack});

  final PlayerController player;
  final VoidCallback onBack;

  static const rates = [0.75, 1.0, 1.25, 1.5, 1.75, 2.0];

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: player,
      builder: (context, _) {
        return Stack(
          children: [
            Positioned.fill(
              child: Container(color: AppTheme.page),
            ),
            SafeArea(
              child: AnimatedSwitcher(
                duration: const Duration(milliseconds: 240),
                switchInCurve: Curves.easeOut,
                switchOutCurve: Curves.easeIn,
                transitionBuilder: (child, animation) => FadeTransition(
                  opacity: animation,
                  child: ScaleTransition(
                    scale: Tween<double>(begin: 0.96, end: 1).animate(animation),
                    child: child,
                  ),
                ),
                child: KeyedSubtree(
                  key: ValueKey(_stateKey),
                  child: _body(context),
                ),
              ),
            ),
            Positioned.fill(
              child: AnimatedSwitcher(
                duration: const Duration(milliseconds: 200),
                child: player.hasResumePrompt
                    ? KeyedSubtree(
                        key: const ValueKey('prompt'),
                        child: _resumeQuestion(context),
                      )
                    : const SizedBox.shrink(key: ValueKey('no-prompt')),
              ),
            ),
          ],
        );
      },
    );
  }

  String get _stateKey {
    if (player.error != null && player.book == null) return 'error';
    if (player.book != null) return 'card';
    if (player.opening) return 'skeleton';
    return 'empty';
  }

  Widget _body(BuildContext context) {
    final book = player.book;
    if (player.error != null && book == null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(30),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 280),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  Icons.error_outline_rounded,
                  size: 26,
                  color: AppTheme.dark.withValues(alpha: 0.6),
                ),
                const SizedBox(height: 10),
                Text(
                  player.error ?? 'Could not open this book. Try again.',
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    fontSize: 13,
                    height: 1.35,
                    color: AppTheme.dark.withValues(alpha: 0.7),
                  ),
                ),
                const SizedBox(height: 14),
                PillButton(title: 'Try again', onPressed: player.retry),
              ],
            ),
          ),
        ),
      );
    }
    if (book == null) {
      if (player.opening) return _skeleton();
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.menu_book_rounded,
              size: 30,
              color: AppTheme.dark.withValues(alpha: 0.45),
            ),
            const SizedBox(height: 10),
            Text(
              'Pick an audiobook from the library',
              style: TextStyle(
                fontSize: 13,
                color: AppTheme.dark.withValues(alpha: 0.6),
              ),
            ),
          ],
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.all(10),
      child: Container(
        clipBehavior: Clip.antiAlias,
        decoration: BoxDecoration(
          color: Colors.white,
          borderRadius: BorderRadius.circular(14),
          border: Border.all(color: AppTheme.cardBorder, width: 3),
          boxShadow: AppTheme.panelShadow,
        ),
        child: Column(
          children: [
            _header(context, book),
            Expanded(child: _chapterList()),
          ],
        ),
      ),
    );
  }

  Widget _skeleton() {
    return Padding(
      padding: const EdgeInsets.all(10),
      child: Container(
        decoration: BoxDecoration(
          color: Colors.white,
          borderRadius: BorderRadius.circular(14),
          border: Border.all(color: AppTheme.cardBorder, width: 3),
          boxShadow: AppTheme.panelShadow,
        ),
        child: const Center(
          child: CircularProgressIndicator(
            strokeWidth: 3,
            color: AppTheme.dark,
          ),
        ),
      ),
    );
  }

  Widget _header(BuildContext context, AudiobookDetails book) {
    final track = player.currentTrack;
    return Stack(
      children: [
        Positioned.fill(child: _CoverBackdrop(url: book.coverUrl)),
        Column(
          children: [
            Padding(
              padding: const EdgeInsets.all(14),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        MarqueeLine(
                          text: book.title,
                          height: 19,
                          style: const TextStyle(
                            fontSize: 15,
                            fontWeight: FontWeight.w600,
                            color: AppTheme.dark,
                          ),
                        ),
                        const SizedBox(height: 3),
                        MarqueeLine(
                          text: track == null
                              ? ''
                              : '${track.number} - ${track.title}',
                          height: 15,
                          style: TextStyle(
                            fontSize: 12,
                            fontWeight: FontWeight.w500,
                            color: AppTheme.dark.withValues(alpha: 0.9),
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 10),
                  CircleButton(
                    icon: Icons.skip_previous_rounded,
                    size: 36,
                    iconSize: 15,
                    disabled: !player.hasPrevious,
                    onPressed: player.previous,
                  ),
                  const SizedBox(width: 8),
                  PlayPauseButton(
                    isPlaying: player.isPlaying,
                    size: 46,
                    onPressed: player.togglePlayPause,
                  ),
                  const SizedBox(width: 8),
                  CircleButton(
                    icon: Icons.skip_next_rounded,
                    size: 36,
                    iconSize: 15,
                    disabled: !player.hasNext,
                    onPressed: player.next,
                  ),
                  const SizedBox(width: 8),
                  CircleButton(
                    icon: Icons.chevron_right_rounded,
                    size: 28,
                    iconSize: 13,
                    tooltip: 'Back to the library',
                    onPressed: onBack,
                  ),
                ],
              ),
            ),
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                _skipButton(15, forward: false),
                const SizedBox(width: 12),
                _rateMenu(),
                const SizedBox(width: 12),
                _skipButton(30, forward: true),
              ],
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(14, 10, 14, 10),
              child: Row(
                children: [
                  SizedBox(
                    width: 60,
                    child: Text(formatClock(player.position), style: _timeStyle()),
                  ),
                  Expanded(
                    child: Scrubber(
                      value: player.position,
                      duration: player.duration,
                      barHeight: 10,
                      onPreview: player.scrubPreview,
                      onCommit: (value) => player.scrubCommit(value),
                    ),
                  ),
                  SizedBox(
                    width: 60,
                    child: Text(
                      formatClock(player.duration),
                      textAlign: TextAlign.right,
                      style: _timeStyle(),
                    ),
                  ),
                ],
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
              child: Row(
                children: [
                  const Text(
                    'Chapters',
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color: AppTheme.dark,
                    ),
                  ),
                  const SizedBox(width: 6),
                  Text(
                    formatCount(player.chapterCount),
                    style: TextStyle(
                      fontSize: 11,
                      fontFeatures: const [FontFeature.tabularFigures()],
                      color: AppTheme.dark.withValues(alpha: 0.65),
                    ),
                  ),
                  const Spacer(),
                  CircleButton(
                    icon: Icons.refresh_rounded,
                    size: 22,
                    iconSize: 11,
                    tooltip: 'Check for new chapters',
                    onPressed: player.refreshChapters,
                  ),
                ],
              ),
            ),
          ],
        ),
      ],
    );
  }

  TextStyle _timeStyle() {
    return TextStyle(
      fontSize: 12,
      fontFeatures: const [FontFeature.tabularFigures()],
      color: AppTheme.dark.withValues(alpha: 0.7),
    );
  }

  Widget _skipButton(int seconds, {required bool forward}) {
    return Tooltip(
      message: forward ? 'Skip forward $seconds seconds' : 'Skip back $seconds seconds',
      child: Material(
        color: AppTheme.dark.withValues(alpha: 0.1),
        shape: const CircleBorder(),
        child: InkWell(
          customBorder: const CircleBorder(),
          onTap: () => player.skip(forward ? seconds.toDouble() : -seconds.toDouble()),
          child: SizedBox(
            width: 30,
            height: 30,
            child: Stack(
              alignment: Alignment.center,
              children: [
                Transform.flip(
                  flipX: forward,
                  child: const Icon(
                    Icons.replay_rounded,
                    size: 18,
                    color: AppTheme.dark,
                  ),
                ),
                Padding(
                  padding: EdgeInsets.only(top: forward ? 1 : 0),
                  child: Text(
                    '$seconds',
                    style: const TextStyle(
                      fontSize: 7,
                      fontWeight: FontWeight.w700,
                      color: AppTheme.dark,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _rateMenu() {
    final label = _rateLabel(player.rate);
    return PopupMenuButton<double>(
      tooltip: '',
      onSelected: player.setRate,
      itemBuilder: (context) => [
        for (final rate in rates)
          PopupMenuItem(
            value: rate,
            child: Text(rate == 1 ? 'Normal' : '${_rateLabel(rate)}×'),
          ),
      ],
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
        decoration: BoxDecoration(
          color: AppTheme.dark.withValues(alpha: 0.1),
          borderRadius: BorderRadius.circular(20),
        ),
        child: Text(
          '$label×',
          style: const TextStyle(
            fontSize: 12,
            fontWeight: FontWeight.w600,
            fontFeatures: [FontFeature.tabularFigures()],
            color: AppTheme.dark,
          ),
        ),
      ),
    );
  }

  static String _rateLabel(double rate) {
    final text = rate.toStringAsFixed(2);
    return text
        .replaceAll(RegExp(r'0+$'), '')
        .replaceAll(RegExp(r'\.$'), '');
  }

  Widget _chapterList() {
    final store = player.chapters;
    return Container(
      color: Colors.white,
      child: ListView.builder(
        padding: const EdgeInsets.only(bottom: 8),
        itemCount: player.chapterCount,
        itemBuilder: (context, index) {
          final track = store?.track(index);
          if (store == null) return const SizedBox.shrink();
          if (track == null) {
            WidgetsBinding.instance.addPostFrameCallback((_) {
              store.ensure(index);
            });
            return ChapterSkeleton(index: index);
          }
          return _chapterRow(track, index);
        },
      ),
    );
  }

  Widget _chapterRow(AudiobookTrack track, int index) {
    final current = index == player.trackIndex;
    final subtitle = AppTheme.dark.withValues(alpha: 0.55);
    return InkWell(
      onTap: () => player.playTrack(index),
      child: Container(
        color: current ? AppTheme.primary.withValues(alpha: 0.1) : Colors.transparent,
        padding: const EdgeInsets.only(right: 10, top: 7, bottom: 7),
        child: Row(
          children: [
            Container(
              width: 2,
              height: 18,
              color: current ? AppTheme.primary : Colors.transparent,
            ),
            const SizedBox(width: 8),
            SizedBox(
              width: 28,
              child: Text(
                '${track.number}',
                textAlign: TextAlign.right,
                maxLines: 1,
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: FontWeight.w600,
                  fontFeatures: const [FontFeature.tabularFigures()],
                  color: current ? AppTheme.primary : AppTheme.dark.withValues(alpha: 0.5),
                ),
              ),
            ),
            const SizedBox(width: 8),
            if (current) ...[
              Icon(
                Icons.volume_up_rounded,
                size: 10,
                color: AppTheme.primary,
              ),
              const SizedBox(width: 4),
            ],
            Expanded(
              child: Text(
                track.title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: current ? FontWeight.w600 : FontWeight.w400,
                  color: current ? AppTheme.primary : AppTheme.dark,
                ),
              ),
            ),
            if ((track.playCount ?? 0) > 0) ...[
              Icon(Icons.play_arrow_rounded, size: 10, color: subtitle),
              const SizedBox(width: 2),
              Text(
                '${track.playCount}',
                style: TextStyle(
                  fontSize: 11,
                  fontFeatures: const [FontFeature.tabularFigures()],
                  color: subtitle,
                ),
              ),
              const SizedBox(width: 8),
            ],
            if (track.durationSeconds != null)
              Text(
                formatClock(track.durationSeconds!.toDouble()),
                style: TextStyle(
                  fontSize: 11,
                  fontFeatures: const [FontFeature.tabularFigures()],
                  color: subtitle,
                ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _resumeQuestion(BuildContext context) {
    final time = player.resumeTime ?? 0;
    final chapter = player.resumeChapter ?? 1;
    return Stack(
      children: [
        Positioned.fill(
          child: GestureDetector(
            onTap: player.dismissResumePrompt,
            child: Container(color: Colors.black.withValues(alpha: 0.12)),
          ),
        ),
        Center(
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 300),
                child: Material(
                  color: Colors.white,
                  borderRadius: BorderRadius.circular(14),
                  child: Container(
                    padding: const EdgeInsets.all(14),
                    decoration: BoxDecoration(
                      color: Colors.white,
                      borderRadius: BorderRadius.circular(14),
                      border: Border.all(color: AppTheme.cardBorder, width: 3),
                      boxShadow: [
                        BoxShadow(
                          color: Colors.black.withValues(alpha: 0.24),
                          blurRadius: 14,
                          offset: const Offset(0, 6),
                        ),
                      ],
                    ),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const Text(
                          'Pick up where you left off?',
                          style: TextStyle(
                            fontSize: 14,
                            fontWeight: FontWeight.w600,
                            color: AppTheme.dark,
                          ),
                        ),
                        const SizedBox(height: 3),
                        Text(
                          'Chapter $chapter · ${formatClock(time)}',
                          style: TextStyle(
                            fontSize: 12,
                            fontFeatures: const [FontFeature.tabularFigures()],
                            color: AppTheme.dark.withValues(alpha: 0.7),
                          ),
                        ),
                        const SizedBox(height: 12),
                        Row(
                          children: [
                            Expanded(
                              child: PillButton(
                                title: 'Resume at ${formatClock(time)}',
                                filled: true,
                                onPressed: player.resumeFromSavedPoint,
                              ),
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              child: PillButton(
                                title: 'Start over',
                                onPressed: player.restartFromBeginning,
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}

class _CoverBackdrop extends StatelessWidget {
  const _CoverBackdrop({required this.url});

  final String? url;

  @override
  Widget build(BuildContext context) {
    return Stack(
      fit: StackFit.expand,
      children: [
        Container(color: Colors.white),
        if (url != null)
          Image.network(
            url!,
            fit: BoxFit.cover,
            errorBuilder: (context, error, stack) => const SizedBox.shrink(),
          ),
        const DecoratedBox(
          decoration: BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topCenter,
              end: Alignment.bottomCenter,
              colors: [
                Color(0xCCFFFFFF),
                Color(0xCCFFFFFF),
                Color(0xFFFFFFFF),
              ],
              stops: [0, 0.4, 1],
            ),
          ),
        ),
      ],
    );
  }
}