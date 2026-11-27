import 'dart:io';

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../theme.dart';
import '../updater.dart';
import '../widgets/controls.dart';

class AboutScreen extends StatelessWidget {
  const AboutScreen({
    super.key,
    required this.updater,
    required this.onBack,
  });

  final UpdateController updater;
  final VoidCallback onBack;

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
                            child: Text(
                              updater.currentVersion,
                              style: const TextStyle(
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
              if (Platform.isMacOS) ...[
                const SizedBox(height: 12),
                _UpdateRow(updater: updater),
              ],
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

class _UpdateRow extends StatelessWidget {
  const _UpdateRow({required this.updater});

  final UpdateController updater;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: updater,
      builder: (context, _) {
        final release = updater.release;
        final available = updater.phase == UpdatePhase.available;
        final failed = updater.phase == UpdatePhase.failed;
        return Container(
          width: double.infinity,
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
          decoration: BoxDecoration(
            color: Colors.white,
            borderRadius: BorderRadius.circular(10),
            border: Border.all(color: AppTheme.dark.withValues(alpha: 0.14)),
          ),
          child: Column(
            children: [
              Text(
                _status(release),
                textAlign: TextAlign.center,
                style: const TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                  color: AppTheme.dark,
                ),
              ),
              if (available && release != null) ...[
                if (release.size != null) ...[
                  const SizedBox(height: 4),
                  Text(
                    '${_sizeLabel(release.size!)} to download',
                    style: TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.w500,
                      color: AppTheme.dark.withValues(alpha: 0.55),
                    ),
                  ),
                ],
                if (release.notes != null) ...[
                  const SizedBox(height: 8),
                  Container(
                    width: double.infinity,
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 6,
                    ),
                    decoration: BoxDecoration(
                      color: AppTheme.dark.withValues(alpha: 0.06),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Text(
                      release.notes!,
                      maxLines: 6,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 10,
                        height: 1.35,
                        color: AppTheme.dark.withValues(alpha: 0.65),
                      ),
                    ),
                  ),
                ],
              ],
              if (updater.phase == UpdatePhase.downloading) ...[
                const SizedBox(height: 8),
                ProgressBar(fraction: updater.progress, height: 5),
              ],
              const SizedBox(height: 10),
              if (available)
                PillButton(
                  title: 'Install and restart',
                  filled: true,
                  onPressed: () => updater.install(release!),
                )
              else if (!updater.isBusy)
                PillButton(
                  title: 'Check for updates',
                  onPressed: () => updater.check(),
                ),
              if (failed && updater.message != null) ...[
                const SizedBox(height: 8),
                Text(
                  updater.message!,
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    fontSize: 10,
                    height: 1.35,
                    color: AppTheme.dark.withValues(alpha: 0.6),
                  ),
                ),
              ],
            ],
          ),
        );
      },
    );
  }

  String _status(UpdateRelease? release) {
    return switch (updater.phase) {
      UpdatePhase.checking => 'Checking…',
      UpdatePhase.upToDate =>
        'Audiobooks ${updater.currentVersion} is up to date',
      UpdatePhase.available => 'Version ${release?.version ?? ''} is available',
      UpdatePhase.downloading =>
        'Downloading… ${(updater.progress * 100).round()}%',
      UpdatePhase.installing => 'Installing…',
      UpdatePhase.failed => 'Update check failed',
      UpdatePhase.idle => 'Audiobooks ${updater.currentVersion}',
    };
  }

  String _sizeLabel(int bytes) {
    if (bytes >= 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    if (bytes >= 1024) return '${(bytes / 1024).round()} KB';
    return '$bytes B';
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
              Icon(icon,
                  size: 13, color: filled ? Colors.white : AppTheme.dark),
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
