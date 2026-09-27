import Foundation

/// Catalogue state: paginated feed, debounced search, and a tag filter whose
/// vocabulary comes from the loaded books (there is no tag-list endpoint).
@MainActor
final class Library: ObservableObject {
    enum Phase: Equatable {
        case idle
        case loading
        case loaded
        case failed(String)
    }

    static let pageSize = 24

    struct TagOption: Identifiable, Hashable {
        let slug: String
        let name: String
        var id: String { slug }
    }

    @Published private(set) var books: [AudiobookSummary] = []
    @Published private(set) var phase: Phase = .idle
    @Published private(set) var hasMore = false
    @Published private(set) var tags: [TagOption] = []
    @Published var searchText = "" {
        didSet { scheduleSearch() }
    }
    @Published var selectedTag: String? {
        didSet { Task { await reload() } }
    }

    private let api: AudiobookAPI
    private var searchTask: Task<Void, Never>?

    init(api: AudiobookAPI) {
        self.api = api
        Task { await reload() }
    }

    func reload() async {
        phase = .loading
        do {
            let page = try await api.list(
                term: searchText,
                tag: selectedTag,
                limit: Self.pageSize,
                offset: 0
            )
            books = page.audiobooks
            hasMore = page.hasMore
            recomputeTags()
            phase = .loaded
        } catch {
            phase = .failed(error.localizedDescription)
        }
    }

    func loadMore() async {
        guard hasMore else { return }
        do {
            let page = try await api.list(
                term: searchText,
                tag: selectedTag,
                limit: Self.pageSize,
                offset: books.count
            )
            // The feed can shift between pages; skip rows already shown.
            let known = Set(books.map(\.id))
            books += page.audiobooks.filter { !known.contains($0.id) }
            hasMore = page.hasMore
        } catch {
            // Keep the current page; the Load-more button stays available.
        }
    }

    private func scheduleSearch() {
        searchTask?.cancel()
        searchTask = Task {
            try? await Task.sleep(for: .milliseconds(350))
            guard !Task.isCancelled else { return }
            await reload()
        }
    }

    private func recomputeTags() {
        var names: [String: String] = [:]
        var order: [String] = []
        for book in books {
            for (slug, name) in zip(book.tagSlugs, book.tags) where names[slug] == nil {
                names[slug] = name
                order.append(slug)
            }
        }
        tags = order.map { TagOption(slug: $0, name: names[$0] ?? $0) }
    }
}
