import 'dart:math' as math;

import 'package:flutter/material.dart';

class LogoMark extends StatelessWidget {
  const LogoMark({super.key, required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    return ExcludeSemantics(
      child: AspectRatio(
        aspectRatio: _LogoPainter.art.width / _LogoPainter.art.height,
        child: CustomPaint(painter: _LogoPainter(color)),
      ),
    );
  }
}

class _LogoPainter extends CustomPainter {
  const _LogoPainter(this.color);

  static const Size art = Size(92.09, 61.75);
  static const double strokeWidth = 4;

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.isEmpty) return;
    final scale = math.min(size.width / art.width, size.height / art.height);
    canvas.save();
    canvas.translate(
      (size.width - art.width * scale) / 2,
      (size.height - art.height * scale) / 2,
    );
    canvas.scale(scale);
    canvas.drawPath(
      _path,
      Paint()
        ..color = color
        ..style = PaintingStyle.stroke
        ..strokeWidth = strokeWidth
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round,
    );
    canvas.restore();
  }

  @override
  bool shouldRepaint(_LogoPainter oldDelegate) => oldDelegate.color != color;
}

final Path _path = () {
  final path = Path()
    ..moveTo(27.7, 24.17)
    ..cubicTo(27.14, 23.49, 24.56, 20.4, 20.78, 20.68)
    ..cubicTo(13.53, 21.23, 11.56, 26.89, 11.61, 31.43)
    ..cubicTo(11.68, 36.69, 17.26, 41.01, 22.69, 41.04)
    ..cubicTo(47.56, 41.19, 54.18, 14.41, 54.18, 14.41)
    ..lineTo(54.63, 40.7)
    ..cubicTo(57.34, 41.56, 71.09, 14.21, 71.09, 14.21)
    ..lineTo(71.29, 40.69)
    ..moveTo(87.59, 7.61)
    ..cubicTo(87.08, 6.84, 85.53, 4.53, 82.78, 4.5)
    ..cubicTo(80.2, 4.47, 77.51, 7.32, 77.51, 9.31)
    ..lineTo(77.39, 57.25)
    ..moveTo(4.5, 47.39)
    ..lineTo(87.55, 47.39);
  return path;
}();
