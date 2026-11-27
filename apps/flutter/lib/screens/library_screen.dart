import 'package:flutter/material.dart';

import '../api.dart';
import '../library_controller.dart';
import '../models.dart';
import '../player_controller.dart';
import '../theme.dart';
import '../widgets/controls.dart';
import '../widgets/mini_player.dart';

class LibraryScreen extends StatefulWidget {
  const LibraryScreen({
    super.key,
    required this.api,
    required this.player,
    required this.onOpenBook,
    required this.onOpenPlayer,
    required this.onOpenAbout,
  });

  final AudiobookApi api;
  final PlayerController player;
  final ValueChanged<AudiobookSummary> onOpenBook;
  final VoidCallback onOpenPlayer;
  final VoidCallback onOpenAbout;

  @override
  State<LibraryScreen> createState() => _LibraryScreenState();
}

class _LibraryScreenState extends State<LibraryScreen>
    with SingleTickerProviderStateMixin {
  late final LibraryController _library;
  final TextEditingController _search = TextEditingController();
  final GlobalKey _drawerKey = GlobalKey();

  late final AnimationController _drawer = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 320),
  )..addListener(() => setState(() {}));

  double _drawerHeight = 110;

  @override
  void initState() {
    super.initState();
    _library = LibraryController(api: widget.api)..reload();
  }

  @override
  void dispose() {
    _library.dispose();
    _search.dispose();
    _drawer.dispose();
    super.dispose();
  }

  double get _reveal => _drawer.value;

  void _measureDrawer() {
    final height = _drawerKey.currentContext?.size?.height ?? 0;
    if (height > 0 && (height - _drawerHeight).abs() > 0.5) {
      _drawerHeight = height;
    }
  }

  void _toggleDrawer() {
    _measureDrawer();
    _settleDrawer(open: _reveal < 0.5);
  }

  void _settleDrawer({bool? open, double velocity = 0}) {
    final target = open ?? (velocity.abs() > 200 ? velocity > 0 : _reveal > 0.5);
    _drawer.animateTo(
      target ? 1 : 0,
      duration: const Duration(milliseconds: 260),
      curve: Curves.easeOutCubic,
    );
  }

  void _dragDrawer(double dy) {
    _measureDrawer();
    _drawer.value = (_drawer.value + dy / _drawerHeight).clamp(0.0, 1.0);
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: _library,
      builder: (context, _) {
        return Column(
          children: [
            _drawerShell(context),
            Opacity(
              opacity: _reveal > 0.01 ? 1 : 0,
              child: Divider(
                height: 1,
                thickness: 1,
                color: AppTheme.dark.withValues(alpha: 0.15),
              ),
            ),
            Expanded(child: _content()),
            ListenableBuilder(
              listenable: widget.player,
              builder: (context, _) {
                return AnimatedSwitcher(
                  duration: const Duration(milliseconds: 250),
                  switchInCurve: Curves.easeOut,
                  switchOutCurve: Curves.easeIn,
                  transitionBuilder: (child, animation) => SizeTransition(
                    sizeFactor: animation,
                    alignment: Alignment.bottomCenter,
                    child: FadeTransition(opacity: animation, child: child),
                  ),
                  child: widget.player.book == null
                      ? const SizedBox.shrink(key: ValueKey('no-mini'))
                      : Column(
                          key: const ValueKey('mini'),
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Divider(
                              height: 1,
                              thickness: 1,
                              color: AppTheme.dark.withValues(alpha: 0.15),
                            ),
                            SafeArea(
                              top: false,
                              child: MiniPlayerBar(
                                player: widget.player,
                                onOpenPlayer: widget.onOpenPlayer,
                              ),
                            ),
                          ],
                        ),
                );
              },
            ),
          ],
        );
      },
    );
  }

  Widget _drawerShell(BuildContext context) {
    return Container(
      color: Colors.white,
      child: SafeArea(
        bottom: false,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ClipRect(
              child: Align(
                alignment: Alignment.topCenter,
                heightFactor: _reveal,
                child: KeyedSubtree(
                  key: _drawerKey,
                  child: _drawerContent(context),
                ),
              ),
            ),
            GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: _toggleDrawer,
              onVerticalDragUpdate: (details) =>
                  _dragDrawer(details.delta.dy),
              onVerticalDragEnd: (details) =>
                  _settleDrawer(velocity: details.primaryVelocity ?? 0),
              child: SizedBox(
                height: 18,
                width: double.infinity,
                child: Center(
                  child: Container(
                    width: 34,
                    height: 4,
                    decoration: BoxDecoration(
                      color: AppTheme.dark.withValues(alpha: 0.28),
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _drawerContent(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 12, 12, 12),
      child: Column(
        children: [
          Row(
            children: [
              const Text(
                'Audiobooks',
                style: TextStyle(
                  fontSize: 20,
                  fontWeight: FontWeight.w700,
                  color: AppTheme.dark,
                ),
              ),
              const Spacer(),
              CircleButton(
                icon: Icons.refresh_rounded,
                size: 30,
                iconSize: 14,
                tooltip: 'Reload the library',
                onPressed: _library.reload,
              ),
              const SizedBox(width: 8),
              CircleButton(
                icon: Icons.info_outline_rounded,
                size: 30,
                iconSize: 14,
                tooltip: 'About this app',
                onPressed: widget.onOpenAbout,
              ),
            ],
          ),
          const SizedBox(height: 10),
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _search,
                  onChanged: _library.setSearch,
                  style: const TextStyle(fontSize: 13, color: AppTheme.dark),
                  decoration: InputDecoration(
                    hintText: 'Search',
                    isDense: true,
                    contentPadding: const EdgeInsets.symmetric(vertical: 9),
                    prefixIcon: Icon(
                      Icons.search_rounded,
                      size: 16,
                      color: AppTheme.dark.withValues(alpha: 0.6),
                    ),
                    prefixIconConstraints: const BoxConstraints(
                      minWidth: 34,
                      minHeight: 0,
                    ),
                    suffixIcon: _search.text.isEmpty
                        ? null
                        : IconButton(
                            iconSize: 16,
                            color: AppTheme.dark.withValues(alpha: 0.4),
                            onPressed: () {
                              _search.clear();
                              _library.setSearch('');
                            },
                            icon: const Icon(Icons.cancel_rounded),
                          ),
                  ),
                ),
              ),
              const SizedBox(width: 8),
              _tagMenu(),
            ],
          ),
        ],
      ),
    );
  }

  Widget _tagMenu() {
    final selected = _library.selectedTag;
    final label = selected == null
        ? 'Tags'
        : _library.tags
              .firstWhere(
                (tag) => tag.slug == selected,
                orElse: () => TagOption(slug: selected, name: selected),
              )
              .name;
    return PopupMenuButton<String>(
      tooltip: '',
      onSelected: (slug) => _library.setTag(slug.isEmpty ? null : slug),
      itemBuilder: (context) => [
        const PopupMenuItem(value: '', child: Text('All tags')),
        if (_library.tags.isNotEmpty) const PopupMenuDivider(),
        for (final tag in _library.tags)
          PopupMenuItem(value: tag.slug, child: Text(tag.name)),
      ],
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 7),
        decoration: BoxDecoration(
          color: Colors.white,
          borderRadius: BorderRadius.circular(17),
          border: Border.all(color: AppTheme.dark.withValues(alpha: 0.35)),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              label,
              style: const TextStyle(
                fontSize: 12,
                fontWeight: FontWeight.w500,
                color: AppTheme.dark,
              ),
            ),
            const SizedBox(width: 4),
            const Icon(
              Icons.expand_more_rounded,
              size: 12,
              color: AppTheme.dark,
            ),
          ],
        ),
      ),
    );
  }

  Widget _content() {
    switch (_library.phase) {
      case LibraryPhase.idle:
      case LibraryPhase.loading:
        return Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const SizedBox(
                width: 28,
                height: 28,
                child: CircularProgressIndicator(
                  strokeWidth: 3,
                  color: AppTheme.dark,
                ),
              ),
              const SizedBox(height: 12),
              Text(
                'Loading audiobooks…',
                style: TextStyle(
                  fontSize: 13,
                  color: AppTheme.dark.withValues(alpha: 0.6),
                ),
              ),
            ],
          ),
        );
      case LibraryPhase.failed:
        return Center(
          child: Padding(
            padding: const EdgeInsets.all(30),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 280),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    Icons.wifi_off_rounded,
                    size: 30,
                    color: AppTheme.dark.withValues(alpha: 0.5),
                  ),
                  const SizedBox(height: 12),
                  Text(
                    _library.errorMessage ??
                        'Could not load the library. Try again.',
                    textAlign: TextAlign.center,
                    style: TextStyle(
                      fontSize: 13,
                      height: 1.35,
                      color: AppTheme.dark.withValues(alpha: 0.7),
                    ),
                  ),
                  const SizedBox(height: 14),
                  PillButton(title: 'Try again', onPressed: _library.reload),
                ],
              ),
            ),
          ),
        );
      case LibraryPhase.loaded:
        if (_library.books.isEmpty) {
          return Center(
            child: Padding(
              padding: const EdgeInsets.all(30),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    Icons.menu_book_rounded,
                    size: 30,
                    color: AppTheme.dark.withValues(alpha: 0.5),
                  ),
                  const SizedBox(height: 12),
                  Text(
                    _library.searchText.isEmpty && _library.selectedTag == null
                        ? 'No published audiobooks yet.'
                        : 'No audiobooks match. Try a different search or tag.',
                    textAlign: TextAlign.center,
                    style: TextStyle(
                      fontSize: 13,
                      color: AppTheme.dark.withValues(alpha: 0.6),
                    ),
                  ),
                ],
              ),
            ),
          );
        }
        final showFooter = _library.hasMore;
        return ListView.builder(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 14),
          itemCount: _library.books.length + (showFooter ? 1 : 0),
          itemBuilder: (context, index) {
            if (index >= _library.books.length) {
              return Padding(
                padding: const EdgeInsets.only(top: 4, bottom: 8),
                child: Center(
                  child: PillButton(
                    title: 'Load more',
                    onPressed: _library.loadMore,
                  ),
                ),
              );
            }
            final book = _library.books[index];
            return Padding(
              padding: EdgeInsets.only(bottom: index == _library.books.length - 1 ? 0 : 14),
              child: _BookCard(
                book: book,
                onTap: () => widget.onOpenBook(book),
              ),
            );
          },
        );
    }
  }
}

class _BookCard extends StatelessWidget {
  const _BookCard({required this.book, required this.onTap});

  final AudiobookSummary book;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Colors.white,
      borderRadius: BorderRadius.circular(10),
      child: InkWell(
        borderRadius: BorderRadius.circular(10),
        onTap: onTap,
        child: Container(
          decoration: BoxDecoration(
            color: Colors.white,
            borderRadius: BorderRadius.circular(10),
            boxShadow: AppTheme.cardShadow,
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              AspectRatio(
                aspectRatio: 1.91,
                child: Stack(
                  children: [
                    Positioned.fill(
                      child: CoverImage(url: book.coverUrl),
                    ),
                    if (book.isVietnameseTranslation)
                      const Positioned(
                        top: 6,
                        right: 6,
                        child: VnFlagBadge(),
                      ),
                    Positioned(
                      left: 0,
                      right: 0,
                      bottom: 0,
                      child: Container(
                        height: 26,
                        color: Colors.white.withValues(alpha: 0.88),
                        child: Row(
                          mainAxisAlignment: MainAxisAlignment.center,
                          children: [
                            const Icon(
                              Icons.menu_book_rounded,
                              size: 11,
                              color: AppTheme.dark,
                            ),
                            const SizedBox(width: 4),
                            Text(
                              '${formatCount(book.trackCount)} chapter'
                              '${book.trackCount == 1 ? '' : 's'}',
                              style: const TextStyle(
                                fontSize: 11,
                                fontWeight: FontWeight.w600,
                                color: AppTheme.dark,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              Padding(
                padding: const EdgeInsets.all(10),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      book.title,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        fontSize: 14,
                        fontWeight: FontWeight.w600,
                        color: AppTheme.dark,
                      ),
                    ),
                    if (book.translator != null) ...[
                      const SizedBox(height: 3),
                      Text(
                        'By ${book.translator}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontSize: 11,
                          color: AppTheme.dark.withValues(alpha: 0.75),
                        ),
                      ),
                    ],
                    if (book.tags.isNotEmpty) ...[
                      const SizedBox(height: 3),
                      Text(
                        'tags: ${book.tags.map((tag) => '#$tag').join(' ')}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                          fontSize: 10,
                          color: AppTheme.primary,
                        ),
                      ),
                    ],
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