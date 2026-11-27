import Cocoa
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    self.contentViewController = flutterViewController
    RegisterGeneratedPlugins(registry: flutterViewController)

    super.awakeFromNib()

    title = "Audiobooks"
    let contentSize = NSSize(width: 384, height: 520)
    styleMask.remove(.resizable)
    contentMinSize = contentSize
    contentMaxSize = contentSize
    setContentSize(contentSize)
  }
}
