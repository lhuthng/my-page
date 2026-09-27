import SwiftUI

/// Screen 1: the catalogue, with the mini player pinned to the bottom while a
/// book is loaded. The header (title, search, tags) is a pull-down drawer: tucked
/// above the top edge, pulled in by its grip — its own bottom edge — and toggled
/// by a click.
struct BrowserScreen: View {
    @EnvironmentObject private var library: Library
    @EnvironmentObject private var player: PlayerModel
    /// The strip's paging state: the info button and mini player move the pages.
    @EnvironmentObject private var pager: PagerModel

    /// How far the drawer stands open in points; `headerHeight` is measured off
    /// the header itself, so the reveal can be driven as a height.
    @State private var reveal: CGFloat = 0
    @State private var headerHeight: CGFloat = 0
    /// Where the current pull began, so the drawer tracks the pointer 1:1.
    @State private var pullFrom: CGFloat?

    /// How far past fully open a pull may tug the drawer before it resists, and
    /// the height of the always-visible grip tab.
    private let stretch: CGFloat = 26
    private let grab: CGFloat = 18

    var body: some View {
        VStack(spacing: 0) {
            headerDrawer
            Divider()
                .overlay(Theme.dark.opacity(0.15))
                .opacity(reveal > 1 ? 1 : 0)
            content
            if player.book != nil {
                Divider().overlay(Theme.dark.opacity(0.15))
                MiniPlayerBar(onOpenPlayer: { pager.openPlayer() })
                    .transition(.move(edge: .bottom))
            }
        }
        .background(Theme.page)
    }

    // MARK: - The header drawer

    private var headerDrawer: some View {
        VStack(spacing: 0) {
            header
                .frame(height: max(reveal, 0), alignment: .top)
                .clipped()
            headerGrip
        }
        .background(Color.white)
        .onPreferenceChange(HeaderHeight.self) { height in
            guard height > 0, abs(height - headerHeight) > 0.5 else { return }
            headerHeight = height
        }
    }

    /// The drawer's grip tab, hanging off its bottom edge. Always visible, so the
    /// header is not an affordance you have to know about.
    private var headerGrip: some View {
        Capsule()
            .fill(Theme.dark.opacity(0.28))
            .frame(width: 34, height: 4)
            .frame(maxWidth: .infinity)
            .frame(height: grab)
            .contentShape(Rectangle())
            .onTapGesture { settle(open: !drawerOpen) }
            .gesture(pullGesture)
            .help(drawerOpen ? "Drag up to hide search and tags" : "Drag down for search and tags")
    }

    private var drawerOpen: Bool { headerHeight > 0 && reveal > headerHeight / 2 }

    private var pullGesture: some Gesture {
        DragGesture(minimumDistance: 1)
            .onChanged { gesture in
                let origin = pullFrom ?? reveal
                pullFrom = origin
                reveal = resisted(origin + gesture.translation.height)
            }
            .onEnded { gesture in
                pullFrom = nil
                // A flick counts even from a short pull.
                let landing = reveal + gesture.predictedEndTranslation.height * 0.25
                settle(open: landing > headerHeight * 0.5)
            }
    }

    /// Rubber band: the drawer tracks the pointer exactly between tucked and
    /// open, and gives only a fraction beyond.
    private func resisted(_ raw: CGFloat) -> CGFloat {
        guard raw > 0 else { return 0 }
        let open = max(headerHeight, 1)
        guard raw > open else { return raw }
        return open + min((raw - open) * 0.3, stretch)
    }

    /// Settle the drawer on one of its two resting positions, open or tucked.
    private func settle(open: Bool) {
        withAnimation(.spring(response: 0.42, dampingFraction: 0.72)) {
            reveal = open ? headerHeight : 0
        }
    }

    private var header: some View {
        VStack(spacing: 10) {
            HStack(alignment: .center) {
                Text("Audiobooks")
                    .font(.title3.weight(.bold))
                    .foregroundStyle(Theme.dark)

                Spacer()

                HStack(spacing: 8) {
                    CircleButton(systemImage: "arrow.clockwise", size: 30, iconSize: 12) {
                        Task { await library.reload() }
                    }
                    CircleButton(systemImage: "info.circle", size: 30, iconSize: 13) {
                        pager.show(2)
                    }
                    .help("About this app")
                }
            }
            // The title row can be pulled too; a click has no travel, so it never
            // starts a drag.
            .contentShape(Rectangle())
            .gesture(pullGesture)

            HStack(spacing: 8) {
                HStack(spacing: 6) {
                    Image(systemName: "magnifyingglass")
                        .font(.caption)
                        .foregroundStyle(Theme.dark.opacity(0.6))
                    TextField(
                        "Search",
                        text: $library.searchText,
                        prompt: Text("Search").foregroundColor(Theme.dark.opacity(0.5))
                    )
                    .textFieldStyle(.plain)
                    .font(.callout)
                    .foregroundStyle(Theme.dark)
                    // This window never paints a text field's placeholder, so it
                    // is drawn on top of the field.
                    .overlay(alignment: .leading) {
                        if library.searchText.isEmpty {
                            Text("Search")
                                .font(.callout)
                                .foregroundStyle(Theme.dark.opacity(0.5))
                                .allowsHitTesting(false)
                        }
                    }
                    if !library.searchText.isEmpty {
                        Button {
                            library.searchText = ""
                        } label: {
                            Image(systemName: "xmark.circle.fill")
                                .font(.caption)
                                .foregroundStyle(Theme.dark.opacity(0.4))
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .frame(maxWidth: .infinity)
                .background(Capsule().fill(.white))
                .overlay(Capsule().strokeBorder(Theme.dark.opacity(0.35)))

                tagMenu
            }
        }
        // The site's header: a white band above the purple page.
        .padding(.horizontal, 12)
        .padding(.vertical, 12)
        .background(Color.white)
        // Report the band's natural height, so the drawer knows how far to open.
        .background(
            GeometryReader { geo in
                Color.clear.preference(key: HeaderHeight.self, value: geo.size.height)
            }
        )
    }

    /// The tag filter capsule. Drawn a second time on top of the menu's label,
    /// because this window never paints a `Menu` label.
    private var tagMenu: some View {
        Menu {
            Button("All tags") { library.selectedTag = nil }
            if !library.tags.isEmpty { Divider() }
            ForEach(library.tags) { tag in
                Button(tag.name) { library.selectedTag = tag.slug }
            }
        } label: {
            tagLabel
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .fixedSize()
        .overlay { tagLabel.allowsHitTesting(false) }
    }

    private var tagLabel: some View {
        HStack(spacing: 4) {
            Text(selectedTagName)
                .font(.footnote.weight(.medium))
            Image(systemName: "chevron.down")
                .font(.system(size: 9, weight: .semibold))
        }
        .foregroundStyle(Theme.dark)
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(Capsule().fill(.white))
        .overlay(Capsule().strokeBorder(Theme.dark.opacity(0.35)))
    }

    private var selectedTagName: String {
        library.selectedTag.flatMap { slug in
            library.tags.first { $0.slug == slug }?.name
        } ?? "Tags"
    }

    @ViewBuilder
    private var content: some View {
        if library.books.isEmpty {
            emptyState
        } else {
            // Indicators off, as in the chapter list.
            ScrollView(.vertical, showsIndicators: false) {
                // One card per row: the window is mini-player width.
                LazyVGrid(columns: [GridItem(.flexible(), spacing: 0)], spacing: 14) {
                    ForEach(library.books) { book in
                        BookCard(book: book) {
                            click { open(book) }
                        }
                    }
                    if library.hasMore {
                        loadMoreButton
                    }
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 14)
            }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Spacer()
            switch library.phase {
            case .failed(let message):
                Image(systemName: "wifi.exclamationmark")
                    .font(.system(size: 30))
                    .foregroundStyle(Theme.dark.opacity(0.5))
                Text(message)
                    .font(.callout)
                    .foregroundStyle(Theme.dark.opacity(0.7))
                    .multilineTextAlignment(.center)
                Button("Try again") {
                    Task { await library.reload() }
                }
                .buttonStyle(.bordered)
            case .loading, .idle:
                ProgressView().controlSize(.large)
                Text("Loading audiobooks…")
                    .font(.callout)
                    .foregroundStyle(Theme.dark.opacity(0.6))
            case .loaded:
                Image(systemName: "book.closed")
                    .font(.system(size: 30))
                    .foregroundStyle(Theme.dark.opacity(0.5))
                Text(library.searchText.isEmpty && library.selectedTag == nil
                    ? "No published audiobooks yet."
                    : "No audiobooks match. Try a different search or tag.")
                    .font(.callout)
                    .foregroundStyle(Theme.dark.opacity(0.6))
            }
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var loadMoreButton: some View {
        Button {
            click { Task { await library.loadMore() } }
        } label: {
            HStack(spacing: 8) {
                Image(systemName: "chevron.down")
                    .font(.caption.weight(.bold))
                Text("Load more")
                    .font(.footnote.weight(.semibold))
            }
            .foregroundStyle(Theme.dark)
            .padding(.horizontal, 18)
            .padding(.vertical, 10)
            .background(Capsule().fill(.white))
            .overlay(Capsule().strokeBorder(Theme.dark.opacity(0.25)))
        }
        .buttonStyle(.plain)
        .frame(maxWidth: .infinity)
        .padding(.top, 4)
    }

    /// Runs a catalogue click, unless the press behind it was a swipe.
    private func click(_ action: () -> Void) {
        guard !pager.lastPressWasDrag else { return }
        action()
    }

    private func open(_ book: AudiobookSummary) {
        // Navigate first, load second: the fetch waits for the slide, so the mini
        // player does not flip to the new book while the library is still up.
        let slides = pager.pageIndex != 0
        pager.openPlayer()
        player.prepare(book)
        Task {
            if slides { try? await Task.sleep(for: .milliseconds(450)) }
            await player.open(book)
        }
    }
}

/// The header's own height, reported so the drawer can be driven as a height.
private struct HeaderHeight: PreferenceKey {
    static var defaultValue: CGFloat = 0
    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = max(value, nextValue())
    }
}

/// Catalogue card echoing AudiobookCard.svelte: wide 1.91:1 cover with the
/// chapter bar overlay, VN flag chip, title, translator, and #tags.
private struct BookCard: View {
    let book: AudiobookSummary
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: 0) {
                ZStack(alignment: .bottom) {
                    CoverImage(url: book.coverURL, cornerRadius: 7)
                        .aspectRatio(1.91 / 1, contentMode: .fit)
                        .overlay(alignment: .topTrailing) {
                            if book.isVietnameseTranslation {
                                VNFlagBadge()
                                    .padding(6)
                            }
                        }
                        .overlay(alignment: .bottom) {
                            HStack(spacing: 4) {
                                Image(systemName: "book.fill")
                                    .font(.system(size: 10, weight: .semibold))
                                Text("\(book.trackCount) chapter\(book.trackCount == 1 ? "" : "s")")
                                    .font(.caption.weight(.semibold))
                            }
                            .foregroundStyle(Theme.dark)
                            .frame(maxWidth: .infinity)
                            .frame(height: 26)
                            .background(.white.opacity(0.88))
                        }
                }

                VStack(alignment: .leading, spacing: 3) {
                    Text(book.title)
                        .font(.system(size: 14, weight: .semibold))
                        .lineLimit(2)
                        .foregroundStyle(Theme.dark)
                        .multilineTextAlignment(.leading)
                    if let translator = book.translator {
                        Text("By \(translator)")
                            .font(.caption)
                            .foregroundStyle(Theme.dark.opacity(0.75))
                    }
                    if !book.tags.isEmpty {
                        Text(
                            "tags: "
                                + book.tags.map { "#\($0)" }.joined(separator: " ")
                        )
                        .font(.caption2)
                        .foregroundStyle(Theme.primary)
                        .lineLimit(1)
                    }
                }
                .padding(10)
            }
            .background(Color.white)
            .clipShape(RoundedRectangle(cornerRadius: 10))
            .shadow(color: .black.opacity(0.14), radius: 5, y: 2)
        }
        .buttonStyle(.plain)
    }
}
