import AppKit
import CoreGraphics
import Foundation

/// Cover loader with an in-memory NSCache. Images are downsampled so a huge
/// cover never sits decoded at full resolution.
actor ImageCache {
    static let shared = ImageCache()

    private let cache = NSCache<NSURL, NSImage>()
    private var inFlight: [URL: Task<Void, Never>] = [:]

    init() {
        cache.countLimit = 200
        cache.totalCostLimit = 96 * 1024 * 1024
    }

    func image(at url: URL?) async -> NSImage? {
        guard let url else { return nil }
        if let hit = cache.object(forKey: url as NSURL) { return hit }
        if let running = inFlight[url] {
            await running.value
            return cache.object(forKey: url as NSURL)
        }
        let task = Task<Void, Never> { [weak self] in
            do {
                let (data, _) = try await URLSession.shared.data(from: url)
                if let image = Self.downsampled(data, maxPixelSize: 640) {
                    await self?.store(image, for: url)
                }
            } catch {
                // Leave the cache untouched; callers render the placeholder.
            }
            await self?.finish(url)
        }
        inFlight[url] = task
        await task.value
        return cache.object(forKey: url as NSURL)
    }

    private func store(_ image: NSImage, for url: URL) {
        cache.setObject(image, forKey: url as NSURL, cost: image.pixelByteCount)
    }

    private func finish(_ url: URL) {
        inFlight[url] = nil
    }

    /// Decode with a thumbnail request instead of a full bitmap.
    private static func downsampled(_ data: Data, maxPixelSize: Int) -> NSImage? {
        let sourceOptions = [kCGImageSourceShouldCache: false] as CFDictionary
        guard let source = CGImageSourceCreateWithData(data as CFData, sourceOptions) else {
            return nil
        }
        let thumbnailOptions = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceShouldCacheImmediately: true,
            kCGImageSourceThumbnailMaxPixelSize: maxPixelSize,
        ] as CFDictionary
        guard let cgImage = CGImageSourceCreateThumbnailAtIndex(source, 0, thumbnailOptions) else {
            return nil
        }
        return NSImage(cgImage: cgImage, size: NSSize(width: cgImage.width, height: cgImage.height))
    }
}

extension NSImage {
    var pixelByteCount: Int {
        guard let cg = cgImage(forProposedRect: nil, context: nil, hints: nil) else { return 0 }
        return cg.bytesPerRow * cg.height
    }
}
