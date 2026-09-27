import SwiftUI

@main
struct AudiobooksApp: App {
    @StateObject private var library = Library(api: .shared)
    @StateObject private var player = PlayerModel(api: .shared)
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootPager()
                .environmentObject(library)
                .environmentObject(player)
                // No focus rings: the window opens with a control focused, which
                // drew a stray ring. Text fields keep their normal behaviour.
                .focusEffectDisabled()
                // Build marker in the titlebar, from the same place as About's.
                .navigationTitle(AppInfo.name)
                // Fixed mini window: the card floats on the page wash.
                .frame(width: 384, height: 520)
                .onChange(of: scenePhase) { _, _ in
                    if scenePhase != .active { player.saveNow() }
                }
        }
        .windowStyle(.automatic)
        .windowResizability(.contentSize)
    }
}
