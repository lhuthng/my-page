import 'package:flutter/material.dart';

import '../models.dart';
import '../player_controller.dart';
import '../theme.dart';
import 'controls.dart';

class MiniPlayerBar extends StatelessWidget {
  const MiniPlayerBar({
    super.key,
    required this.player,
    required this.onOpenPlayer,
  });

  final PlayerController player;
  final VoidCallback onOpenPlayer;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: player,
      builder: (context, _) {
        final book = player.book;
        if (book == null) return const SizedBox.shrink();
        return Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          child: card(context, book),
        );
      },
    );
  }

  Widget card(BuildContext context, AudiobookDetails book) {
    final track = player.currentTrack;
    return Material(
      color: Colors.white,
      borderRadius: BorderRadius.circular(12),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onOpenPlayer,
        child: Container(
          decoration: BoxDecoration(
            color: Colors.white,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: AppTheme.cardBorder, width: 3),
            boxShadow: AppTheme.panelShadow,
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(12, 6, 12, 0),
                child: Scrubber(
                  value: player.position,
                  duration: player.duration,
                  onPreview: player.scrubPreview,
                  onCommit: (value) => player.scrubCommit(value),
                ),
              ),
              Padding(
                padding: const EdgeInsets.all(10),
                child: Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          MarqueeLine(
                            text: book.title,
                            height: 16,
                            style: const TextStyle(
                              fontSize: 13,
                              fontWeight: FontWeight.w600,
                              color: AppTheme.dark,
                            ),
                          ),
                          const SizedBox(height: 2),
                          MarqueeLine(
                            text: track == null
                                ? ''
                                : '${track.number} - ${track.title}',
                            height: 14,
                            style: TextStyle(
                              fontSize: 11,
                              color: AppTheme.dark.withValues(alpha: 0.8),
                            ),
                          ),
                          Text(
                            '${formatClock(player.position)} / '
                            '${formatClock(player.duration)}',
                            style: TextStyle(
                              fontSize: 11,
                              fontFeatures: const [
                                FontFeature.tabularFigures(),
                              ],
                              color: AppTheme.dark.withValues(alpha: 0.5),
                            ),
                          ),
                        ],
                      ),
                    ),
                    CircleButton(
                      icon: Icons.skip_previous_rounded,
                      size: 34,
                      iconSize: 15,
                      disabled: !player.hasPrevious,
                      onPressed: player.previous,
                    ),
                    const SizedBox(width: 8),
                    PlayPauseButton(
                      isPlaying: player.isPlaying,
                      size: 40,
                      onPressed: player.togglePlayPause,
                    ),
                    const SizedBox(width: 8),
                    CircleButton(
                      icon: Icons.skip_next_rounded,
                      size: 34,
                      iconSize: 15,
                      disabled: !player.hasNext,
                      onPressed: player.next,
                    ),
                    const SizedBox(width: 8),
                    CircleButton(
                      icon: Icons.chevron_left_rounded,
                      size: 28,
                      iconSize: 13,
                      tooltip: 'Open the player',
                      onPressed: onOpenPlayer,
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}