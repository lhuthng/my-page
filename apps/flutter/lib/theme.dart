import 'package:flutter/material.dart';

class AppTheme {
  static const Color dark = Color(0xFF495C83);
  static const Color primary = Color(0xFF7A86B6);
  static const Color page = Color(0xFFC8B6E2);
  static const Color accentGreen = Color(0xFF68B87E);
  static const Color accentGreenDark = Color(0xFF42A05C);
  static const Color accentRed = Color(0xFFB86872);
  static const Color accentRedDark = Color(0xFFC44A58);
  static const Color cardBorder = Color(0x80495C83);

  static List<BoxShadow> get cardShadow => [
    BoxShadow(
      color: Colors.black.withValues(alpha: 0.14),
      blurRadius: 5,
      offset: const Offset(0, 2),
    ),
  ];

  static List<BoxShadow> get panelShadow => [
    BoxShadow(
      color: Colors.black.withValues(alpha: 0.2),
      blurRadius: 12,
      offset: const Offset(0, 5),
    ),
  ];

  static ThemeData build() {
    final scheme = ColorScheme.fromSeed(
      seedColor: primary,
      brightness: Brightness.light,
    ).copyWith(
      primary: dark,
      secondary: accentGreen,
      surface: Colors.white,
    );
    return ThemeData(
      colorScheme: scheme,
      scaffoldBackgroundColor: page,
      useMaterial3: true,
      textTheme: const TextTheme(
        headlineSmall: TextStyle(fontWeight: FontWeight.w700, color: dark),
        titleMedium: TextStyle(fontWeight: FontWeight.w600, color: dark),
        bodyMedium: TextStyle(color: dark),
        labelLarge: TextStyle(fontWeight: FontWeight.w600, color: dark),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: Colors.white,
        hintStyle: TextStyle(color: dark.withValues(alpha: 0.5)),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(17),
          borderSide: BorderSide(color: dark.withValues(alpha: 0.35)),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(17),
          borderSide: BorderSide(color: dark.withValues(alpha: 0.35)),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(17),
          borderSide: const BorderSide(color: dark, width: 1.5),
        ),
      ),
    );
  }
}
