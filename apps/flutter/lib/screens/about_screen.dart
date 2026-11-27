import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../theme.dart';
import '../widgets/controls.dart';

class AboutScreen extends StatelessWidget {
  const AboutScreen({super.key, required this.onBack});

  final VoidCallback onBack;

  static const String version = '0.1.0';
  static const String summary =
      'A cross-platform player for the audiobooks on Huu Thang\'s Blog.';
  static const String tagline =
      'Thắng\'s digital garden for software architecture, creative coding, '
      'and hands-on experiments.';

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Container(
          color: Colors.white,
          child: SafeArea(
            bottom: false,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
              child: Row(
                children: [
                  CircleButton(
                    icon: Icons.chevron_left_rounded,
                    size: 30,
                    iconSize: 13,
                    tooltip: 'Back to the library',
                    onPressed: onBack,
                  ),
                  const SizedBox(width: 10),
                  const Text(
                    'About',
                    style: TextStyle(
                      fontSize: 20,
                      fontWeight: FontWeight.w700,
                      color: AppTheme.dark,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
        Divider(
          height: 1,
          thickness: 1,
          color: AppTheme.dark.withValues(alpha: 0.15),
        ),
        Expanded(
          child: ListView(
            padding: const EdgeInsets.all(12),
            children: [
              Container(
                clipBehavior: Clip.antiAlias,
                decoration: BoxDecoration(
                  color: Colors.white,
                  borderRadius: BorderRadius.circular(14),
                  border: Border.all(color: AppTheme.cardBorder, width: 3),
                  boxShadow: AppTheme.panelShadow,
                ),
                child: Column(
                  children: [
                    Container(
                      width: double.infinity,
                      padding: const EdgeInsets.symmetric(vertical: 18),
                      decoration: const BoxDecoration(
                        gradient: LinearGradient(
                          begin: Alignment.topCenter,
                          end: Alignment.bottomCenter,
                          colors: [AppTheme.primary, AppTheme.dark],
                        ),
                      ),
                      child: Column(
                        children: [
                          const Icon(
                            Icons.headphones_rounded,
                            size: 64,
                            color: Colors.white,
                          ),
                          const SizedBox(height: 12),
                          const Text(
                            'Audiobooks',
                            style: TextStyle(
                              fontSize: 21,
                              fontWeight: FontWeight.w700,
                              color: Colors.white,
                            ),
                          ),
                          const SizedBox(height: 7),
                          Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 10,
                              vertical: 4,
                            ),
                            decoration: BoxDecoration(
                              color: Colors.white.withValues(alpha: 0.22),
                              borderRadius: BorderRadius.circular(20),
                              border: Border.all(
                                color: Colors.white.withValues(alpha: 0.35),
                              ),
                            ),
                            child: const Text(
                              version,
                              style: TextStyle(
                                fontSize: 11,
                                fontWeight: FontWeight.w600,
                                fontFeatures: [FontFeature.tabularFigures()],
                                color: Colors.white,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                    Padding(
                      padding: const EdgeInsets.all(14),
                      child: Column(
                        children: [
                          const Text(
                            summary,
                            textAlign: TextAlign.center,
                            style: TextStyle(
                              fontSize: 13,
                              fontWeight: FontWeight.w500,
                              color: AppTheme.dark,
                            ),
                          ),
                          const SizedBox(height: 6),
                          Text(
                            tagline,
                            textAlign: TextAlign.center,
                            style: TextStyle(
                              fontSize: 11,
                              fontStyle: FontStyle.italic,
                              color: AppTheme.dark.withValues(alpha: 0.65),
                            ),
                          ),
                          const SizedBox(height: 14),
                          _factChip('Written by', 'Huu Thang Le'),
                          const SizedBox(height: 8),
                          _factChip('Site', 'huuthangle.site'),
                          const SizedBox(height: 14),
                          Row(
                            children: [
                              Expanded(
                                child: _LinkChip(
                                  icon: Icons.public_rounded,
                                  title: 'Visit the blog',
                                  filled: true,
                                  url: 'https://huuthangle.site',
                                ),
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: _LinkChip(
                                  icon: Icons.code_rounded,
                                  title: 'GitHub',
                                  url: 'https://github.com/lhuthng',
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _factChip(String label, String value) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(vertical: 8),
      decoration: BoxDecoration(
        color: AppTheme.dark.withValues(alpha: 0.07),
        borderRadius: BorderRadius.circular(20),
        border: Border.all(color: AppTheme.dark.withValues(alpha: 0.14)),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Text(
            label,
            style: TextStyle(
              fontSize: 10,
              fontWeight: FontWeight.w600,
              color: AppTheme.dark.withValues(alpha: 0.6),
            ),
          ),
          const SizedBox(width: 6),
          Text(
            value,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(
              fontSize: 12,
              fontWeight: FontWeight.w600,
              color: AppTheme.dark,
            ),
          ),
        ],
      ),
    );
  }
}

class _LinkChip extends StatelessWidget {
  const _LinkChip({
    required this.icon,
    required this.title,
    required this.url,
    this.filled = false,
  });

  final IconData icon;
  final String title;
  final String url;
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
        onTap: () => launchUrl(Uri.parse(url)),
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 11),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(icon, size: 13, color: filled ? Colors.white : AppTheme.dark),
              const SizedBox(width: 6),
              Flexible(
                child: Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                    color: filled ? Colors.white : AppTheme.dark,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}