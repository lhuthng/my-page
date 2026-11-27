import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:media_kit/media_kit.dart';

import 'api.dart';
import 'app_shell.dart';
import 'progress_store.dart';
import 'theme.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  MediaKit.ensureInitialized();
  SystemChrome.setSystemUIOverlayStyle(
    const SystemUiOverlayStyle(
      statusBarColor: Colors.transparent,
      statusBarIconBrightness: Brightness.dark,
      statusBarBrightness: Brightness.light,
      systemNavigationBarColor: AppTheme.page,
      systemNavigationBarIconBrightness: Brightness.dark,
    ),
  );
  final progress = ProgressStore();
  await progress.load();
  runApp(AudiobooksApp(api: AudiobookApi(), progress: progress));
}

class AudiobooksApp extends StatelessWidget {
  const AudiobooksApp({super.key, required this.api, required this.progress});

  final AudiobookApi api;
  final ProgressStore progress;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Audiobooks',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.build(),
      home: AppShell(api: api, progress: progress),
    );
  }
}