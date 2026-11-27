class ChapterWindow {
  static const int size = 20;
  static const int prefetchAhead = 3;

  static int start(int index) => index < 0 ? 0 : index ~/ size * size;

  static List<int> starts(int count) {
    return [for (var i = 0; i < count; i += size) i];
  }

  static int count(int start, int total) {
    final remaining = total - start;
    if (remaining <= 0) return 0;
    return remaining < size ? remaining : size;
  }
}
