import 'dart:async';

import 'package:flutter/foundation.dart';

import 'api.dart';
import 'models.dart';

enum LibraryPhase { idle, loading, loaded, failed }

class TagOption {
  const TagOption({required this.slug, required this.name});

  final String slug;
  final String name;
}

class LibraryController extends ChangeNotifier {
  LibraryController({required this.api});

  static const int pageSize = 24;

  final AudiobookApi api;

  LibraryPhase phase = LibraryPhase.idle;
  List<AudiobookSummary> books = const [];
  List<TagOption> tags = const [];
  bool hasMore = false;
  String searchText = '';
  String? selectedTag;
  String? errorMessage;

  Timer? _searchDebounce;

  Future<void> reload() async {
    phase = LibraryPhase.loading;
    notifyListeners();
    try {
      final page = await api.list(
        term: searchText,
        tag: selectedTag,
        limit: pageSize,
      );
      books = page.audiobooks;
      hasMore = page.hasMore;
      _recomputeTags();
      phase = LibraryPhase.loaded;
    } catch (error) {
      errorMessage = describeError(error);
      phase = LibraryPhase.failed;
    }
    notifyListeners();
  }

  Future<void> loadMore() async {
    if (!hasMore || phase != LibraryPhase.loaded) return;
    try {
      final page = await api.list(
        term: searchText,
        tag: selectedTag,
        limit: pageSize,
        offset: books.length,
      );
      final known = books.map((book) => book.id).toSet();
      books = [
        ...books,
        ...page.audiobooks.where((book) => !known.contains(book.id)),
      ];
      hasMore = page.hasMore;
      notifyListeners();
    } catch (_) {
      // The button stays available; the current page is kept.
    }
  }

  void setSearch(String value) {
    searchText = value;
    notifyListeners();
    _searchDebounce?.cancel();
    _searchDebounce = Timer(const Duration(milliseconds: 350), reload);
  }

  void setTag(String? slug) {
    if (selectedTag == slug) return;
    selectedTag = slug;
    reload();
  }

  void _recomputeTags() {
    final names = <String, String>{};
    final order = <String>[];
    for (final book in books) {
      for (var i = 0; i < book.tagSlugs.length && i < book.tags.length; i++) {
        final slug = book.tagSlugs[i];
        if (!names.containsKey(slug)) {
          names[slug] = book.tags[i];
          order.add(slug);
        }
      }
    }
    tags = [
      for (final slug in order) TagOption(slug: slug, name: names[slug]!),
    ];
  }

  @override
  void dispose() {
    _searchDebounce?.cancel();
    super.dispose();
  }
}
