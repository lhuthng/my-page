import 'package:audio_service/audio_service.dart';
import 'package:audio_session/audio_session.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:media_kit/media_kit.dart';

import 'api.dart';
import 'app_shell.dart';
import 'audio_handler.dart';
import 'player_controller.dart';
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
  final player = PlayerController(api: AudiobookApi(), progress: progress);
  final audioHandler = AudiobookAudioHandler(player);
  await AudioService.init(
    builder: () => audioHandler,
    config: const AudioServiceConfig(
      androidNotificationChannelId: 'site.huuthangle.audiobooks.audio',
      androidNotificationChannelName: 'Audiobook playback',
      androidNotificationChannelDescription:
          'Controls for the current audiobook',
      notificationColor: Color(0xFF7A86B6),
      androidStopForegroundOnPause: false,
      fastForwardInterval: Duration(seconds: 30),
      rewindInterval: Duration(seconds: 15),
      artDownscaleWidth: 600,
      artDownscaleHeight: 600,
    ),
  );
  final session = await AudioSession.instance;
  await session.configure(AudioSessionConfiguration.speech());
  runApp(AudiobooksApp(player: player));
}

class AudiobooksApp extends StatelessWidget {
  const AudiobooksApp({super.key, required this.player});

  final PlayerController player;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Audiobooks',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.build(),
      home: AppShell(player: player),
    );
  }
}
