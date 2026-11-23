// Photos for the Mac: SwiftUI over Photos' core (core/, through UniFFI) and
// UUOB's SDK. The connect screen and the update button are UUOB's kit for
// apps (uuob/kit/mac).

import AVFoundation
import AVKit
import AppKit
import SwiftUI
import UniformTypeIdentifiers

let photosApp = AppInfo(
    id: "photos", name: "Photos", noun: "photos", symbol: "photo", color: "orange",
    version: version(),
    updateFeed: "https://raw.githubusercontent.com/jachymri/uuob-releases/main/photos.json",
    updateKey: "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDU5QjY3MTNBQjA5NTI0QzAKUldUQUpKV3dPbkcyV2VzZU9XSWJQT29NWTR3angxNjU1VWJZazJsMi80Zjl6WDJoSWxMK0t3MlUK"
)

@main
struct PhotosApp: App {
    @State private var model = PhotosModel(Uuob(photosApp))
    @State private var updater = Updater(photosApp)

    var body: some Scene {
        WindowGroup("Photos") {
            Gate(uuob: model.uuob, updater: updater) { PhotosView(model: model, updater: updater) }
                .frame(minWidth: 380, minHeight: 420)
                .background(Look.bg)
        }
        // No title bar: the sidebar and the grid are the window, as on the web.
        .windowStyle(.hiddenTitleBar)
        .defaultSize(width: 1100, height: 760)
        .commands { Self.commands(model) }
    }

    /// The shared shortcuts (docs/design.md) and Photos' own: the places.
    @MainActor static func commands(_ model: PhotosModel) -> UuobCommands {
        let places = [("Photos", "all"), ("Videos", "videos"), ("Recently Deleted", "trash")]
        return UuobCommands(
            settings: { model.settingsOpen = true },
            new: { model.pick() }, newTitle: "Add Photos",
            refresh: { Task { await model.load() } },
            extraMenu: "Go",
            extra: places.enumerated().map { i, v in Shortcut(title: v.0, key: KeyEquivalent(Character("\(i + 1)"))) { model.show(v.1) } }
        )
    }
}

func now() -> Int64 { Int64(Date().timeIntervalSince1970 * 1000) }

/// Photos' state, kept live from UUOB. The rules are the core's; the data is UUOB's.
@Observable
@MainActor
final class PhotosModel {
    let uuob: Uuob
    let settings: UuobSettings
    var months: [J] = []
    var all = "[]"
    /// The place shown: "all", "videos" or "trash" (Recently deleted).
    var view = "all"
    var trash: Bool { view == "trash" }
    var settingsOpen = false
    /// The photo whose details are shown (⌘I).
    var info: J?
    var open: J?
    var confirm: J?
    var adding: (Int, Int)?
    var error = ""
    private var made: [String: String] = [:]
    private var purged = false
    private var thumbs: [String: NSImage] = [:]

    init(_ uuob: Uuob) {
        self.uuob = uuob
        settings = UuobSettings(uuob)
        all = uuob.client.cachedAll(collection: "photos")
        Task {
            for await e in uuob.events() where e.type.string == "changed" || e.state.string == "connected" {
                await load()
            }
        }
    }

    var client: UuobClient { uuob.client }
    var grid: J { J(text: galleryView(recordsJson: all, view: view)) }

    func show(_ place: String) {
        view = place
        open = nil
    }
    var shown: [J] { grid.groups.array.flatMap { $0.items.array } }

    func load() async {
        if let cs = try? await client.containers() { months = J(text: cs).array.filter { !$0.main.bool } }
        let r = J(text: await client.all(collection: "photos"))
        all = r.records.text == "null" ? "[]" : r.records.text
        // Anything deleted more than 30 days ago goes for good, once per start.
        if !purged && !r.records.array.isEmpty {
            purged = true
            for p in J(text: expired(recordsJson: all, now: now())).array { await destroy(p) }
        }
    }

    func thumb(_ p: J) -> NSImage? {
        let t = p.thumb.string
        if t.isEmpty { return nil }
        let key = "\(p.container.string)/\(p.id)/\(t.count)"
        if let i = thumbs[key] { return i }
        let i = NSImage(data: Data(thumbBytes(thumb: t)))
        thumbs[key] = i
        return i
    }

    func pick() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = true
        panel.allowedContentTypes = [.image, .movie]
        panel.title = "Add photos"
        if panel.runModal() == .OK { Task { await add(panel.urls) } }
    }

    /// The container of a month, created when its first photo is added. Its id is the same on every device.
    private func monthContainer(_ key: String) async throws -> String {
        if let c = months.first(where: { $0.name.string == key }) { return c.id }
        if let id = made[key] { return id }
        let id = try await client.createContainer(name: key, key: monthContainerKey(month: key))
        made[key] = id
        return id
    }

    func add(_ urls: [URL]) async {
        error = ""
        adding = (0, urls.count)
        for (i, url) in urls.enumerated() {
            let name = url.lastPathComponent
            do {
                let values = try url.resourceValues(forKeys: [.contentModificationDateKey, .fileSizeKey, .contentTypeKey])
                let modified = Int64((values.contentModificationDate?.timeIntervalSince1970 ?? 0) * 1000)
                let taken = takenOrNow(modified: modified, now: now())
                let type = values.contentType?.preferredMIMEType ?? "application/octet-stream"
                let container = try await monthContainer(monthOf(ms: taken))
                // A format this Mac can't draw still gets stored, without a preview.
                let preview = type.hasPrefix("video/") ? await videoPreview(url) : imagePreview(url)
                let file = J(text: try await client.putFilePath(container: container, path: url.path, contentType: type))
                let record = newRecord(taken: taken, added: now(), name: name, contentType: type, size: Int64(values.fileSize ?? 0), file: file.id, preview: preview.text)
                _ = try await client.put(collection: "photos", container: container, id: nil, value: record)
            } catch {
                self.error = "\(name) could not be added. \(error.userMessage)"
            }
            adding = (i + 1, urls.count)
        }
        adding = nil
        await load()
    }

    func remove(_ p: J) async {
        _ = try? await client.put(collection: "photos", container: p.container.string, id: nil, value: markDeleted(record: p.text, now: now()))
    }

    func restore(_ p: J) async {
        _ = try? await client.put(collection: "photos", container: p.container.string, id: nil, value: markDeleted(record: p.text, now: 0))
    }

    func destroy(_ p: J) async {
        try? await client.deleteFile(container: p.container.string, id: p.file.string)
        _ = try? await client.delete(collection: "photos", container: p.container.string, id: p.id)
    }

    // MARK: The original: save, copy, share

    /// What the user is told after saving or copying (shown for a moment).
    var note = ""

    private func original(_ p: J) async -> Data? {
        guard let bytes = try? await client.readFile(container: p.container.string, id: p.file.string) else {
            note = "The original is on a storage device that can’t be reached right now."
            return nil
        }
        return Data(bytes)
    }

    /// Save the original wherever the user picks (⌘S).
    func save(_ p: J) {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = saveName(record: p.text)
        panel.canCreateDirectories = true
        guard panel.runModal() == .OK, let url = panel.url else { return }
        Task {
            guard let data = await original(p) else { return }
            do { try data.write(to: url); note = "Saved" } catch { note = "Couldn’t save: \(error.localizedDescription)" }
        }
    }

    /// Copy the picture, to paste into another app (⌘C); a video is copied as its file.
    func copy(_ p: J) {
        Task {
            guard let data = await original(p) else { return }
            let pb = NSPasteboard.general
            pb.clearContents()
            if p.type.string.hasPrefix("video/") {
                let url = FileManager.default.temporaryDirectory.appendingPathComponent(saveName(record: p.text))
                try? data.write(to: url)
                pb.writeObjects([url as NSURL])
            } else if let image = NSImage(data: data) {
                pb.writeObjects([image])
            }
            note = "Copied"
        }
    }

    /// The Mac's share menu (AirDrop, Messages, Mail…) for the original.
    func share(_ p: J) {
        Task {
            guard let data = await original(p) else { return }
            let url = FileManager.default.temporaryDirectory.appendingPathComponent(saveName(record: p.text))
            try? data.write(to: url)
            guard let view = NSApp.keyWindow?.contentView else { return }
            let anchor = NSRect(x: view.bounds.maxX - 140, y: view.bounds.maxY - 60, width: 1, height: 1)
            NSSharingServicePicker(items: [url]).show(relativeTo: anchor, of: view, preferredEdge: .minY)
        }
    }

    func step(_ by: Int) {
        guard let p = open, let i = shown.firstIndex(where: { $0.id == p.id && $0.container == p.container }) else { return }
        let j = i + by
        if j >= 0 && j < shown.count { open = shown[j] }
    }
}

private func jpeg(_ image: CGImage, _ w: Int, _ h: Int) -> J {
    let size = thumbSize(w: UInt32(w), h: UInt32(h))
    let tw = Int(size[0]), th = Int(size[1])
    guard let ctx = CGContext(data: nil, width: tw, height: th, bitsPerComponent: 8, bytesPerRow: 0, space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue) else { return J([:]) }
    ctx.interpolationQuality = .high
    ctx.draw(image, in: CGRect(x: 0, y: 0, width: tw, height: th))
    guard let small = ctx.makeImage(), let data = NSBitmapImageRep(cgImage: small).representation(using: .jpeg, properties: [.compressionFactor: 0.6]) else { return J([:]) }
    return J(["w": w, "h": h, "thumb": jpegDataUrl(jpeg: data)])
}

/// `{ w, h, thumb }` of a picture, upright.
func imagePreview(_ url: URL) -> J {
    guard let src = CGImageSourceCreateWithURL(url as CFURL, nil),
          let image = CGImageSourceCreateThumbnailAtIndex(src, 0, [kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceCreateThumbnailWithTransform: true, kCGImageSourceThumbnailMaxPixelSize: 1600] as CFDictionary),
          let props = CGImageSourceCopyPropertiesAtIndex(src, 0, nil) as? [CFString: Any]
    else { return J([:]) }
    var w = props[kCGImagePropertyPixelWidth] as? Int ?? image.width
    var h = props[kCGImagePropertyPixelHeight] as? Int ?? image.height
    if let o = props[kCGImagePropertyOrientation] as? Int, o >= 5 { swap(&w, &h) }
    return jpeg(image, w, h)
}

/// `{ w, h, duration, thumb }` of a video: its frame at 0.1 s.
func videoPreview(_ url: URL) async -> J {
    let asset = AVURLAsset(url: url)
    let gen = AVAssetImageGenerator(asset: asset)
    gen.appliesPreferredTrackTransform = true
    guard let duration = try? await asset.load(.duration).seconds,
          let (image, _) = try? await gen.image(at: CMTime(seconds: min(0.1, duration), preferredTimescale: 600))
    else { return J([:]) }
    var r = jpeg(image, image.width, image.height).value as? [String: Any] ?? [:]
    r["duration"] = duration
    return J(r)
}

struct PhotosView: View {
    @Bindable var model: PhotosModel
    @Bindable var updater: Updater

    var body: some View {
        let g = model.grid
        ZStack {
            // The places on the left (a rail of icons when folded or narrow), the grid on the right.
            SidebarScaffold {
                PhotosSidebar(model: model, updater: updater, places: places(g))
            } rail: {
                PhotosRail(model: model, places: places(g))
            } content: {
                gridView(g)
            }
            .uuobKeys(PhotosApp.commands(model))
            if let (done, total) = model.adding {
                VStack {
                    Spacer()
                    Text("Adding \(min(done + 1, total)) of \(total)…").font(.system(size: 14))
                        .padding(.horizontal, 18).padding(.vertical, 10)
                        .background(Look.elevated, in: Capsule())
                        .overlay(Capsule().strokeBorder(Look.lineStrong))
                        .shadow(color: .black.opacity(0.25), radius: 14, y: 8)
                        .padding(20)
                }
            }
            // The viewer covers the whole window, black, as on the web.
            if let p = model.open { Viewer(model: model, p: p).transition(.opacity) }
            // "Saved", "Copied" after the right-click menu (the viewer shows its own).
            if model.open == nil && !model.note.isEmpty {
                VStack {
                    Spacer()
                    Text(model.note).font(.system(size: 14, weight: .medium))
                        .padding(.horizontal, 16).padding(.vertical, 9).background(Look.elevated, in: Capsule())
                        .shadow(radius: 8).padding(.bottom, 24)
                        .task(id: model.note) { try? await Task.sleep(for: .seconds(2.5)); model.note = "" }
                }
            }
        }
        .ignoresSafeArea(.container, edges: .top)
        .foregroundStyle(Look.text)
        .animation(.easeOut(duration: 0.18), value: model.open?.text)
        .task { await model.load() }
        .sheet(isPresented: $model.settingsOpen) { SettingsSheet(app: photosApp, settings: model.settings) { EmptyView() } }
        .sheet(item: $model.info) { p in InfoSheet(p: p) { model.info = nil } }
        .sheet(item: $model.confirm) { p in
            ChoiceSheet(
                title: "Delete this \(p.type.string.hasPrefix("video/") ? "video" : "photo")?",
                message: "It is removed from all your devices. This can’t be undone.",
                choices: [
                    ("Cancel", .plain, {}),
                    ("Delete", .danger, { model.open = nil; Task { await model.destroy(p); await model.load() } }),
                ],
                cancel: false
            )
        }
    }

    /// The sidebar's places: (id, name, icon, count); Recently deleted only while something is there.
    private func places(_ g: J) -> [(String, String, String, Int)] {
        [("all", "Photos", "photo", g.photos.int), ("videos", "Videos", "play", g.videos.int), ("trash", "Recently deleted", "trash", g.trash.int)]
            .filter { $0.0 != "trash" || $0.3 > 0 || model.trash }
    }

    /// The grid: the toolbar (the place and its count, the +), then the months.
    private func gridView(_ g: J) -> some View {
        let here = places(g).first { $0.0 == model.view } ?? ("all", "Photos", "photo", g.photos.int)
        return GeometryReader { geo in
            let compact = geo.size.width < 640
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    Toolbar(title: titleText(here.1, here.3 > 0 ? "\(here.3)" : ""), compact: compact,
                            add: model.trash || model.adding != nil ? nil : { model.pick() }, addHelp: "Add photos (⌘N)") {
                        UpdateButton(updater: updater)
                    }
                    if !model.error.isEmpty { Text(model.error).foregroundStyle(Look.danger).padding(.bottom, 14) }
                    if g.groups.array.isEmpty {
                        switch model.view {
                        case "trash": EmptyState(icon: "trash", headline: "Nothing here")
                        case "videos": EmptyState(icon: "play", headline: "No videos")
                        default: EmptyState(icon: "photo", headline: "No photos yet", color: Palette.color("orange"), action: model.adding == nil ? ("Add photos", { model.pick() } as () -> Void) : nil)
                        }
                    }
                    ForEach(Array(g.groups.array.enumerated()), id: \.element.monthId) { n, group in
                        SectionLabel(group.label.string).padding(.top, n == 0 ? 6 : 26).padding(.bottom, 10)
                        LazyVGrid(columns: [GridItem(.adaptive(minimum: 150), spacing: 4)], spacing: 4) {
                            ForEach(group.items.array, id: \.text) { p in cell(p) }
                        }
                    }
                }
                .frame(maxWidth: 1040)
                .padding(.horizontal, compact ? 14 : 28).padding(.top, compact ? 40 : 48).padding(.bottom, 48)
                .frame(maxWidth: .infinity)
            }
            .scrollIndicators(.never)
            .background(Look.bg)
        }
    }

    private func cell(_ p: J) -> some View {
        let video = p.type.string.hasPrefix("video/")
        return Button { model.open = p } label: {
            ZStack(alignment: .bottomTrailing) {
                Rectangle().fill(Look.surface2).aspectRatio(1, contentMode: .fit)
                    .overlay {
                        if let t = model.thumb(p) { Image(nsImage: t).resizable().scaledToFill() }
                        else { Ico(video ? "play" : "photo", size: 28).foregroundStyle(Look.text3) }
                    }
                    .clipped()
                if model.trash {
                    let d = daysLeft(deleted: p.deleted.int64, now: now())
                    badge(nil, "\(d) \(d == 1 ? "day" : "days")")
                } else if video {
                    badge("play", p.duration.isNull ? "" : durationLabel(seconds: p.duration.double))
                }
            }
            .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
        }
        .buttonStyle(.plain)
        .help("\(video ? "Video" : "Photo") from \(whenLabel(ms: p.taken.int64))")
        // Right-click (or Control-click): what the viewer offers, without opening it.
        .contextMenu {
            Button("Open") { model.open = p }
            if p.deleted.isNull {
                Button("Save…") { model.save(p) }
                Button("Copy") { model.copy(p) }
                Button("Share…") { model.share(p) }
                Divider()
                Button("Delete") { Task { await model.remove(p) } }
            } else {
                Button("Restore") { Task { await model.restore(p) } }
                Button("Delete Forever…") { model.confirm = p }
            }
        }
    }

    /// `.badge`: a dark pill in the corner.
    private func badge(_ icon: String?, _ t: String) -> some View {
        HStack(spacing: 4) {
            if let icon { Ico(icon, size: 11, stroke: 2.4) }
            if !t.isEmpty { Text(t) }
        }
        .font(.system(size: 12, weight: .semibold)).foregroundStyle(.white)
        .padding(.horizontal, 7).padding(.vertical, 2).background(.black.opacity(0.55), in: Capsule()).padding(6)
    }
}

/// One photo or video, full size over the whole window: the preview at once, then the original.
struct Viewer: View {
    @Bindable var model: PhotosModel
    let p: J
    @State private var image: NSImage?
    @State private var player: AVPlayer?
    @State private var missing = false

    var body: some View {
        let video = p.type.string.hasPrefix("video/")
        let i = model.shown.firstIndex(where: { $0.id == p.id && $0.container == p.container }) ?? 0
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                Button { model.open = nil } label: { Ico("close", size: 20) }.buttonStyle(OnBlack()).keyboardShortcut(.cancelAction).help("Close")
                Text(whenLabel(ms: p.taken.int64)).font(.system(size: 15)).foregroundStyle(.white.opacity(0.8)).lineLimit(1).truncationMode(.tail)
                Spacer()
                Button { model.info = p } label: { Ico("info", size: 20) }.buttonStyle(OnBlack()).keyboardShortcut("i").help("Info (⌘I)")
                // Space closes, as Quick Look does.
                Button("") { model.open = nil }.keyboardShortcut(.space, modifiers: []).opacity(0).frame(width: 0, height: 0)
                if !p.deleted.isNull {
                    Button("Restore") { model.open = nil; Task { await model.restore(p) } }.buttonStyle(UuobButton())
                    Button("Delete") { model.confirm = p }.buttonStyle(UuobButton(.danger))
                } else {
                    Button { model.share(p) } label: { Ico("share", size: 20) }.buttonStyle(OnBlack()).help("Share")
                    Button { model.copy(p) } label: { Ico("copy", size: 20) }.buttonStyle(OnBlack()).keyboardShortcut("c").help("Copy (⌘C)")
                    Button { model.save(p) } label: { Ico("download", size: 20) }.buttonStyle(OnBlack()).keyboardShortcut("s").help("Save… (⌘S)")
                    Button {
                        let next = model.shown.indices.contains(i + 1) ? model.shown[i + 1] : nil
                        model.open = next
                        Task { await model.remove(p) }
                    } label: { Ico("trash", size: 20) }.buttonStyle(OnBlack()).keyboardShortcut(.delete, modifiers: []).help("Delete (⌫)")
                }
            }
            // Below the window's own buttons, which sit over the top left.
            .padding(.horizontal, 12).padding(.top, 34).padding(.bottom, 10)
            ZStack {
                if missing {
                    if let t = model.thumb(p) { Image(nsImage: t).resizable().scaledToFit().opacity(0.5) }
                    Text("The original is on a storage device that can’t be reached right now.").font(.system(size: 15)).foregroundStyle(.white)
                        .multilineTextAlignment(.center).frame(maxWidth: 420)
                        .padding(.horizontal, 18).padding(.vertical, 12).background(.black.opacity(0.7), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                } else if video, let player {
                    VideoPlayerView(player: player)
                } else if let image {
                    Image(nsImage: image).resizable().scaledToFit()
                } else if let t = model.thumb(p) {
                    Image(nsImage: t).resizable().scaledToFit()
                }
                HStack {
                    if i > 0 { Button { model.step(-1) } label: { Ico("left", size: 22) }.buttonStyle(Nav()).keyboardShortcut(.leftArrow, modifiers: []).help("Previous") }
                    Spacer()
                    if i < model.shown.count - 1 { Button { model.step(1) } label: { Ico("right", size: 22) }.buttonStyle(Nav()).keyboardShortcut(.rightArrow, modifiers: []).help("Next") }
                }
                .padding(.horizontal, 14)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .background(Color.black)
        .foregroundStyle(.white)
        .overlay(alignment: .bottom) {
            if !model.note.isEmpty {
                Text(model.note).font(.system(size: 14, weight: .medium)).foregroundStyle(.white)
                    .padding(.horizontal, 16).padding(.vertical, 9).background(.black.opacity(0.75), in: Capsule())
                    .padding(.bottom, 24)
                    .task(id: model.note) { try? await Task.sleep(for: .seconds(2.5)); model.note = "" }
            }
        }
        .task(id: p.text) {
            image = nil; player = nil; missing = false
            if video {
                let url = await model.client.fileUrl(container: p.container.string, id: p.file.string)
                if let u = URL(string: url) { let pl = AVPlayer(url: u); player = pl; pl.play() }
            } else {
                do {
                    let bytes = try await model.client.readFile(container: p.container.string, id: p.file.string)
                    image = NSImage(data: Data(bytes))
                    if image == nil { missing = true }
                } catch {
                    missing = true
                }
            }
        }
    }

    /// The viewer bar's icon buttons: white, a faint circle when hovered.
    struct OnBlack: ButtonStyle {
        @State private var hover = false
        func makeBody(configuration: Configuration) -> some View {
            configuration.label.foregroundStyle(.white).frame(width: 36, height: 36)
                .background(hover || configuration.isPressed ? Color.white.opacity(0.12) : .clear, in: Circle())
                .contentShape(Circle()).onHover { hover = $0 }
        }
    }

    /// `.nav`: the arrows, 44 round on a dark circle.
    struct Nav: ButtonStyle {
        func makeBody(configuration: Configuration) -> some View {
            configuration.label.foregroundStyle(.white).frame(width: 44, height: 44)
                .background(.black.opacity(configuration.isPressed ? 0.6 : 0.4), in: Circle()).contentShape(Circle())
        }
    }
}

/// A photo's details (the kit's detail card): an icon and a line each.
struct InfoSheet: View {
    let p: J
    var onClose: () -> Void
    var body: some View {
        let rows = J(text: details(record: p.text)).array
        DetailSheet(title: "Info", onClose: onClose) {
            EmptyView()
        } content: {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(Array(rows.enumerated()), id: \.offset) { i, r in
                    IconRow(r.icon.string, last: i == rows.count - 1) {
                        Text(r.label.string)
                        Spacer(minLength: 16)
                        Text(r["value"].string).foregroundStyle(Look.text2).lineLimit(1).truncationMode(.middle).textSelection(.enabled)
                    }
                }
            }
            .padding(.top, 6)
        }
    }
}

/// The sidebar (the kit's): Photos, Videos, Recently deleted; Settings.
struct PhotosSidebar: View {
    @Bindable var model: PhotosModel
    @Bindable var updater: Updater
    let places: [(String, String, String, Int)]
    @Environment(\.uuobClosePeek) private var closePeek

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Color.clear.frame(height: 36) // the window's buttons
            ForEach(places, id: \.0) { p in
                SidebarRow(title: p.1, selected: model.view == p.0) {
                    Ico(p.2, size: 19).foregroundStyle(Look.text2)
                } trailing: {
                    // No count by Recently deleted: its name needs the room (the toolbar shows it).
                    Text(p.3 > 0 && p.0 != "trash" ? "\(p.3)" : "")
                } action: { model.show(p.0); closePeek() }
            }
            Spacer()
            SidebarRow(title: "Settings") { Ico("settings", size: 19).foregroundStyle(Look.text2) } action: { closePeek(); model.settingsOpen = true }
        }
        .padding(.horizontal, 12).padding(.top, 20).padding(.bottom, 4)
    }
}

/// Folded or narrow: the places' icons, Settings.
struct PhotosRail: View {
    @Bindable var model: PhotosModel
    let places: [(String, String, String, Int)]

    var body: some View {
        VStack(spacing: 6) {
            Color.clear.frame(height: 30) // the window's buttons
            ForEach(places, id: \.0) { p in RailButton(p.2, help: p.1, selected: model.view == p.0) { model.show(p.0) } }
            Spacer()
            RailButton("settings", help: "Settings (⌘,)") { model.settingsOpen = true }
        }
        .padding(.top, 4)
    }
}

/// AVKit's player view, for a video.
struct VideoPlayerView: NSViewRepresentable {
    let player: AVPlayer
    func makeNSView(context: Context) -> NSView {
        let v = AVPlayerViewHost()
        v.player = player
        return v
    }
    func updateNSView(_ nsView: NSView, context: Context) { (nsView as? AVPlayerViewHost)?.player = player }
}

final class AVPlayerViewHost: AVPlayerView {}

extension J {
    /// Ids for `ForEach`.
    var monthId: String { self["month"].string }
}
