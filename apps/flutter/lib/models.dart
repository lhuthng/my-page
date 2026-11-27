const String defaultApiBase = String.fromEnvironment(
  'AUDIOBOOKS_API_BASE',
  defaultValue: 'https://api.huuthangle.site',
);

String? mediaUrl(String? path, {String base = defaultApiBase}) {
  if (path == null || path.isEmpty) return null;
  if (path.startsWith('http://') || path.startsWith('https://')) return path;
  return Uri.parse(base).resolve(path).toString();
}

int _asInt(Object? value) => (value as num).toInt();
int? _asIntOrNull(Object? value) => (value as num?)?.toInt();

class AudiobookSummary {
  const AudiobookSummary({
    required this.id,
    required this.title,
    required this.slug,
    required this.description,
    required this.status,
    required this.trackCount,
    required this.totalDurationSeconds,
    required this.tags,
    required this.tagSlugs,
    required this.createdAt,
    this.translator,
    this.url,
    this.publishedAt,
    this.ownerUsername,
    this.ownerDisplayName,
  });

  factory AudiobookSummary.fromJson(Map<String, dynamic> json) {
    return AudiobookSummary(
      id: _asInt(json['id']),
      title: json['title'] as String,
      slug: json['slug'] as String,
      description: json['description'] as String? ?? '',
      translator: json['translator'] as String?,
      status: json['status'] as String? ?? '',
      url: json['url'] as String?,
      trackCount: _asInt(json['track_count']),
      totalDurationSeconds: _asInt(json['total_duration_seconds']),
      tags: (json['tags'] as List<dynamic>? ?? const []).cast<String>(),
      tagSlugs: (json['tag_slugs'] as List<dynamic>? ?? const []).cast<String>(),
      createdAt: json['created_at'] as String? ?? '',
      publishedAt: json['published_at'] as String?,
      ownerUsername: json['owner_username'] as String?,
      ownerDisplayName: json['owner_display_name'] as String?,
    );
  }

  final int id;
  final String title;
  final String slug;
  final String description;
  final String? translator;
  final String status;
  final String? url;
  final int trackCount;
  final int totalDurationSeconds;
  final List<String> tags;
  final List<String> tagSlugs;
  final String createdAt;
  final String? publishedAt;
  final String? ownerUsername;
  final String? ownerDisplayName;

  String? get coverUrl => mediaUrl(url);

  bool get isVietnameseTranslation => tagSlugs.contains('vietnamese-translated');
}

class AudiobookTag {
  const AudiobookTag({
    required this.id,
    required this.name,
    required this.slug,
    this.description,
    this.audiobookCount = 0,
  });

  factory AudiobookTag.fromJson(Map<String, dynamic> json) {
    return AudiobookTag(
      id: _asInt(json['id']),
      name: json['name'] as String,
      slug: json['slug'] as String,
      description: json['description'] as String?,
      audiobookCount: _asIntOrNull(json['audiobook_count']) ?? 0,
    );
  }

  final int id;
  final String name;
  final String slug;
  final String? description;
  final int audiobookCount;
}

class AudiobookTrack {
  AudiobookTrack({
    required this.id,
    required this.title,
    required this.number,
    required this.shortName,
    required this.url,
    required this.fileType,
    this.durationSeconds,
    this.playCount,
  });

  factory AudiobookTrack.fromJson(Map<String, dynamic> json) {
    return AudiobookTrack(
      id: _asInt(json['id']),
      title: json['title'] as String,
      number: _asInt(json['number']),
      durationSeconds: _asIntOrNull(json['duration_seconds']),
      playCount: _asIntOrNull(json['play_count']),
      shortName: json['short_name'] as String,
      url: json['url'] as String,
      fileType: json['file_type'] as String? ?? '',
    );
  }

  final int id;
  final String title;
  final int number;
  final int? durationSeconds;
  final String shortName;
  final String url;
  final String fileType;
  int? playCount;

  String? get streamUrl => mediaUrl(url);
}

class AudiobookDetails {
  AudiobookDetails({
    required this.id,
    required this.title,
    required this.slug,
    required this.description,
    required this.status,
    required this.totalDurationSeconds,
    required this.ownerUsername,
    required this.ownerDisplayName,
    required this.tags,
    required this.tracks,
    this.translator,
    this.url,
    this.trackCount,
    this.hasMoreTracks,
    this.createdAt = '',
    this.publishedAt,
  });

  factory AudiobookDetails.fromJson(Map<String, dynamic> json) {
    return AudiobookDetails(
      id: _asInt(json['id']),
      title: json['title'] as String,
      slug: json['slug'] as String,
      description: json['description'] as String? ?? '',
      translator: json['translator'] as String?,
      status: json['status'] as String? ?? '',
      url: json['url'] as String?,
      totalDurationSeconds: _asInt(json['total_duration_seconds']),
      ownerUsername: json['owner_username'] as String? ?? '',
      ownerDisplayName: json['owner_display_name'] as String? ?? '',
      tags: (json['tags'] as List<dynamic>? ?? const [])
          .map((tag) => AudiobookTag.fromJson(tag as Map<String, dynamic>))
          .toList(),
      tracks: (json['tracks'] as List<dynamic>? ?? const [])
          .map((track) => AudiobookTrack.fromJson(track as Map<String, dynamic>))
          .toList(),
      trackCount: _asIntOrNull(json['track_count']),
      hasMoreTracks: json['has_more_tracks'] as bool?,
      createdAt: json['created_at'] as String? ?? '',
      publishedAt: json['published_at'] as String?,
    );
  }

  final int id;
  final String title;
  final String slug;
  final String description;
  final String? translator;
  final String status;
  final String? url;
  final int totalDurationSeconds;
  final String ownerUsername;
  final String ownerDisplayName;
  final List<AudiobookTag> tags;
  final List<AudiobookTrack> tracks;
  final int? trackCount;
  final bool? hasMoreTracks;
  final String createdAt;
  final String? publishedAt;

  String? get coverUrl => mediaUrl(url);

  int get totalTracks => trackCount ?? tracks.length;

  bool get hasMoreChapters => hasMoreTracks ?? false;
}

class AudiobookListPage {
  const AudiobookListPage({required this.audiobooks, required this.hasMore});

  factory AudiobookListPage.fromJson(Map<String, dynamic> json) {
    return AudiobookListPage(
      audiobooks: (json['audiobooks'] as List<dynamic>? ?? const [])
          .map((book) => AudiobookSummary.fromJson(book as Map<String, dynamic>))
          .toList(),
      hasMore: json['has_more'] as bool? ?? false,
    );
  }

  final List<AudiobookSummary> audiobooks;
  final bool hasMore;
}

String formatClock(double seconds) {
  if (!seconds.isFinite || seconds < 0) return '--:--';
  final total = seconds.floor();
  final hours = total ~/ 3600;
  final minutes = (total % 3600) ~/ 60;
  final secs = total % 60;
  String two(int value) => value.toString().padLeft(2, '0');
  if (hours > 0) return '$hours:${two(minutes)}:${two(secs)}';
  return '$minutes:${two(secs)}';
}

String formatDurationLabel(int seconds) {
  if (seconds <= 0) return '';
  if (seconds < 60) return 'under a minute';
  final minutes = seconds ~/ 60;
  if (minutes < 60) return '$minutes min';
  final hours = minutes ~/ 60;
  final rest = minutes % 60;
  if (rest == 0) return '$hours hr';
  return '$hours hr $rest min';
}

String formatCount(int value) {
  final digits = value.abs().toString();
  final buffer = StringBuffer();
  for (var i = 0; i < digits.length; i++) {
    if (i > 0 && (digits.length - i) % 3 == 0) buffer.write(',');
    buffer.write(digits[i]);
  }
  return value < 0 ? '-$buffer' : buffer.toString();
}
