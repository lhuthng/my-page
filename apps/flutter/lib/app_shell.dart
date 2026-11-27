import 'package:flutter/material.dart';

import 'api.dart';
import 'models.dart';
import 'player_controller.dart';
import 'progress_store.dart';
import 'screens/about_screen.dart';
import 'screens/library_screen.dart';
import 'screens/player_screen.dart';
import 'theme.dart';

class AppShell extends StatefulWidget {
  const AppShell({super.key, required this.api, required this.progress});

  final AudiobookApi api;
  final ProgressStore progress;

  @override
  State<AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<AppShell> {
  late final PlayerController _player;
  final PageController _pages = PageController(initialPage: 1);
  int _page = 1;

  @override
  void initState() {
    super.initState();
    _player = PlayerController(api: widget.api, progress: widget.progress);
    _player.addListener(_guardPlayerPage);
  }

  @override
  void dispose() {
    _player.removeListener(_guardPlayerPage);
    _player.dispose();
    _pages.dispose();
    super.dispose();
  }

  void _guardPlayerPage() {
    if (_page != 0) return;
    if (_player.opening || _player.book != null || _player.error != null) return;
    _go(1);
  }

  void _go(int page) {
    if (!_pages.hasClients) return;
    _pages.animateToPage(
      page,
      duration: const Duration(milliseconds: 260),
      curve: Curves.easeOutCubic,
    );
  }

  void _openBook(AudiobookSummary book) {
    _go(0);
    _player.open(book);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: AppTheme.page,
      body: PageView(
        controller: _pages,
        physics: const ClampingScrollPhysics(),
        onPageChanged: (page) {
          setState(() => _page = page);
          if (page == 0 &&
              _player.book == null &&
              !_player.opening &&
              _player.error == null) {
            _go(1);
          }
        },
        children: [
          PlayerScreen(player: _player, onBack: () => _go(1)),
          LibraryScreen(
            api: widget.api,
            player: _player,
            onOpenBook: _openBook,
            onOpenPlayer: () => _go(0),
            onOpenAbout: () => _go(2),
          ),
          AboutScreen(onBack: () => _go(1)),
        ],
      ),
    );
  }
}