import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../theme.dart';

class Scrubber extends StatefulWidget {
  const Scrubber({
    super.key,
    required this.value,
    required this.duration,
    required this.onPreview,
    required this.onCommit,
    this.barHeight = 8,
  });

  final double value;
  final double duration;
  final double barHeight;
  final ValueChanged<double> onPreview;
  final ValueChanged<double> onCommit;

  @override
  State<Scrubber> createState() => _ScrubberState();
}

class _ScrubberState extends State<Scrubber> {
  double? _preview;

  double _ratioAt(double x, double width) {
    if (width <= 0 || widget.duration <= 0) return 0;
    return (x / width).clamp(0.0, 1.0);
  }

  void _previewAt(double x, double width) {
    final time = _ratioAt(x, width) * widget.duration;
    setState(() => _preview = time);
    widget.onPreview(time);
  }

  void _commit() {
    final target = _preview;
    if (target == null) return;
    setState(() => _preview = null);
    widget.onCommit(target);
  }

  @override
  Widget build(BuildContext context) {
    final shown = _preview ?? widget.value;
    final bar = widget.barHeight;
    return SizedBox(
      height: 16,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final width = constraints.maxWidth;
          final ratio = widget.duration <= 0
              ? 0.0
              : (shown / widget.duration).clamp(0.0, 1.0);
          final filled = bar + math.max(width - bar, 0) * ratio;
          return GestureDetector(
            behavior: HitTestBehavior.opaque,
            onPanDown: (details) =>
                _previewAt(details.localPosition.dx, width),
            onPanUpdate: (details) =>
                _previewAt(details.localPosition.dx, width),
            onPanEnd: (_) => _commit(),
            onPanCancel: _commit,
            child: Stack(
              clipBehavior: Clip.none,
              children: [
                Positioned.fill(
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: Container(
                      height: bar,
                      decoration: BoxDecoration(
                        color: AppTheme.dark.withValues(alpha: 0.15),
                        borderRadius: BorderRadius.circular(bar),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  left: 0,
                  top: (16 - bar) / 2,
                  child: Container(
                    width: filled,
                    height: bar,
                    decoration: BoxDecoration(
                      color: AppTheme.primary.withValues(alpha: 0.6),
                      borderRadius: BorderRadius.circular(bar),
                    ),
                  ),
                ),
                Positioned(
                  left: filled - bar,
                  top: (16 - bar) / 2,
                  child: Container(
                    width: bar,
                    height: bar,
                    decoration: const BoxDecoration(
                      color: AppTheme.primary,
                      shape: BoxShape.circle,
                    ),
                    child: Center(
                      child: Container(
                        width: math.max(bar - 4, 1),
                        height: math.max(bar - 4, 1),
                        decoration: const BoxDecoration(
                          color: Colors.white,
                          shape: BoxShape.circle,
                        ),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}

class CircleButton extends StatelessWidget {
  const CircleButton({
    super.key,
    required this.icon,
    required this.onPressed,
    this.size = 34,
    this.iconSize = 14,
    this.disabled = false,
    this.tooltip,
  });

  final IconData icon;
  final VoidCallback? onPressed;
  final double size;
  final double iconSize;
  final bool disabled;
  final String? tooltip;

  @override
  Widget build(BuildContext context) {
    final button = Opacity(
      opacity: disabled ? 0.3 : 1,
      child: Material(
        color: AppTheme.dark.withValues(alpha: 0.1),
        shape: const CircleBorder(),
        child: InkWell(
          customBorder: const CircleBorder(),
          onTap: disabled ? null : onPressed,
          child: SizedBox(
            width: size,
            height: size,
            child: Icon(icon, size: iconSize, color: AppTheme.dark),
          ),
        ),
      ),
    );
    if (tooltip == null) return button;
    return Tooltip(message: tooltip!, child: button);
  }
}

class PlayPauseButton extends StatelessWidget {
  const PlayPauseButton({
    super.key,
    required this.isPlaying,
    required this.onPressed,
    this.size = 42,
  });

  final bool isPlaying;
  final VoidCallback onPressed;
  final double size;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: isPlaying ? AppTheme.accentRed : AppTheme.accentGreen,
      shape: const CircleBorder(),
      child: InkWell(
        customBorder: const CircleBorder(),
        onTap: onPressed,
        child: SizedBox(
          width: size,
          height: size,
          child: Icon(
            isPlaying ? Icons.pause_rounded : Icons.play_arrow_rounded,
            color: Colors.white,
            size: size * 0.52,
          ),
        ),
      ),
    );
  }
}

class PillButton extends StatelessWidget {
  const PillButton({
    super.key,
    required this.title,
    required this.onPressed,
    this.filled = false,
  });

  final String title;
  final VoidCallback onPressed;
  final bool filled;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: filled ? AppTheme.dark : Colors.white,
      shape: StadiumBorder(
        side: filled
            ? BorderSide.none
            : BorderSide(color: AppTheme.dark.withValues(alpha: 0.35)),
      ),
      child: InkWell(
        customBorder: const StadiumBorder(),
        onTap: onPressed,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
          child: Text(
            title,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            textAlign: TextAlign.center,
            style: TextStyle(
              fontSize: 12,
              fontWeight: FontWeight.w600,
              color: filled ? Colors.white : AppTheme.dark,
            ),
          ),
        ),
      ),
    );
  }
}

class ProgressBar extends StatelessWidget {
  const ProgressBar({super.key, required this.fraction, this.height = 6});

  final double fraction;
  final double height;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: height,
      child: LayoutBuilder(
        builder: (context, constraints) {
          return Stack(
            children: [
              Positioned.fill(
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: AppTheme.dark.withValues(alpha: 0.15),
                    borderRadius: BorderRadius.circular(height),
                  ),
                ),
              ),
              Container(
                width: constraints.maxWidth * fraction.clamp(0.0, 1.0),
                height: height,
                decoration: BoxDecoration(
                  color: AppTheme.primary,
                  borderRadius: BorderRadius.circular(height),
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class MarqueeLine extends StatefulWidget {
  const MarqueeLine({
    super.key,
    required this.text,
    required this.style,
    this.height = 16,
  });

  final String text;
  final TextStyle style;
  final double height;

  @override
  State<MarqueeLine> createState() => _MarqueeLineState();
}

class _MarqueeLineState extends State<MarqueeLine>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
  );
  double _overflow = 0;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _configure(double overflow) {
    if ((overflow - _overflow).abs() <= 1) return;
    _overflow = overflow;
    if (overflow <= 1) {
      _controller.stop();
      _controller.value = 0;
      return;
    }
    final seconds = (6 + overflow / 25) * 2;
    _controller.duration = Duration(milliseconds: (seconds * 1000).round());
    if (!_controller.isAnimating) _controller.repeat();
  }

  double _offsetFor(double overflow, double t) {
    const hold = 0.18;
    const travel = 0.32;
    double ease(double v) {
      final c = v.clamp(0.0, 1.0);
      return c * c * (3 - 2 * c);
    }

    if (t < hold) return 0;
    if (t < hold + travel) return overflow * ease((t - hold) / travel);
    if (t < 2 * hold + travel) return overflow;
    return overflow * (1 - ease((t - 2 * hold - travel) / travel));
  }

  @override
  Widget build(BuildContext context) {
    final style = widget.style.copyWith(height: 1.2);
    return SizedBox(
      height: widget.height,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final painter = TextPainter(
            text: TextSpan(text: widget.text, style: style),
            maxLines: 1,
            textDirection: TextDirection.ltr,
          )..layout();
          final overflow = math.max(0.0, painter.width - constraints.maxWidth);
          _configure(overflow);
          if (overflow <= 1) {
            return Align(
              alignment: Alignment.centerLeft,
              child: Text(
                widget.text,
                maxLines: 1,
                overflow: TextOverflow.clip,
                style: style,
              ),
            );
          }
          return ClipRect(
            child: AnimatedBuilder(
              animation: _controller,
              builder: (context, child) {
                return Transform.translate(
                  offset: Offset(
                    -_offsetFor(overflow, _controller.value),
                    0,
                  ),
                  child: child,
                );
              },
              child: Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  widget.text,
                  maxLines: 1,
                  style: style,
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}

class CoverImage extends StatelessWidget {
  const CoverImage({
    super.key,
    required this.url,
    this.radius = 7,
    this.bordered = true,
  });

  final String? url;
  final double radius;
  final bool bordered;

  @override
  Widget build(BuildContext context) {
    return Container(
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.circular(radius),
        border: bordered
            ? Border.all(color: AppTheme.cardBorder, width: 3)
            : null,
      ),
      child: url == null
          ? const _CoverGlyph()
          : Image.network(
              url!,
              fit: BoxFit.cover,
              errorBuilder: (context, error, stack) => const _CoverGlyph(),
            ),
    );
  }
}

class _CoverGlyph extends StatelessWidget {
  const _CoverGlyph();

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Icon(
        Icons.menu_book_rounded,
        size: 26,
        color: Color(0x73595C83),
      ),
    );
  }
}

class VnFlagBadge extends StatelessWidget {
  const VnFlagBadge({super.key, this.height = 18});

  final double height;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: height * 1.5,
      height: height,
      decoration: BoxDecoration(
        color: const Color(0xFFDA251D),
        borderRadius: BorderRadius.circular(3),
        border: Border.all(color: Colors.black.withValues(alpha: 0.1)),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.15),
            blurRadius: 1,
            offset: const Offset(0, 0.5),
          ),
        ],
      ),
      child: Center(
        child: CustomPaint(
          size: Size(height * 0.62, height * 0.62),
          painter: _StarPainter(),
        ),
      ),
    );
  }
}

class _StarPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final path = Path();
    final center = Offset(size.width / 2, size.height / 2);
    final outer = math.min(size.width, size.height) / 2;
    final inner = outer * 0.382;
    for (var index = 0; index < 10; index++) {
      final angle = index * math.pi / 5 - math.pi / 2;
      final radius = index.isEven ? outer : inner;
      final point = Offset(
        center.dx + math.cos(angle) * radius,
        center.dy + math.sin(angle) * radius,
      );
      if (index == 0) {
        path.moveTo(point.dx, point.dy);
      } else {
        path.lineTo(point.dx, point.dy);
      }
    }
    path.close();
    canvas.drawPath(path, Paint()..color = const Color(0xFFFFFF00));
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class ChapterSkeleton extends StatelessWidget {
  const ChapterSkeleton({super.key, this.index = 0});

  final int index;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(right: 10),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 7),
        child: Row(
          children: [
            const SizedBox(width: 2),
            _bar(18, 9),
            const SizedBox(width: 8),
            _bar(index.isEven ? 132 : 168, 9),
            const Spacer(),
            _bar(26, 9),
          ],
        ),
      ),
    );
  }

  Widget _bar(double width, double height) {
    return Container(
      width: width,
      height: height,
      decoration: BoxDecoration(
        color: AppTheme.dark.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(height / 2),
      ),
    );
  }
}