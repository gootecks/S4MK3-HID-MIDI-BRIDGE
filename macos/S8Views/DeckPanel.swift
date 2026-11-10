import CoreGraphics

public enum DeckTime {
    public static func format(_ seconds: Double?, negative: Bool = false) -> String {
        guard let seconds, seconds.isFinite else { return "\u{2013}" }
        let total = Int(max(0, seconds).rounded(.down))
        let text = "\(total / 60):" + (total % 60 < 10 ? "0" : "") + "\(total % 60)"
        return negative ? "-" + text : text
    }
}

public enum DeckPanelRenderer {
    enum Layout {
        static let box = CGRect(x: 8, y: 126, width: 464, height: 140)
        static let innerLeft: CGFloat = 12
        static let innerWidth = 456
        static let centerY: CGFloat = 194
        static let halfHeight: CGFloat = 52
        static let barTickTop: CGFloat = 250
        static let beatTickTop: CGFloat = 256
        static let tickBottom: CGFloat = 262
        static let cueTop: CGFloat = 130
        static let minimumBeatSpacing: CGFloat = 5
    }

    static let bass = RGB(0, 120, 255)
    static let mid = RGB(255, 184, 50)
    static let treble = RGB(236, 240, 248)

    public static func render(_ snapshot: DeckSnapshot) -> Bitmap {
        let canvas = Canvas()
        canvas.fill(CGRect(x: 0, y: 0, width: Metrics.width, height: Metrics.height), Palette.background)
        drawHeader(canvas, snapshot)
        guard snapshot.hasTrack else {
            let text = snapshot.notice ?? "No track on deck \(snapshot.deck)"
            let style = TextStyle(size: 18, bold: true, color: Palette.dim, align: .center, maxWidth: 440)
            canvas.text(text, at: CGPoint(x: 240, y: 150), style)
            return canvas.bitmap
        }
        drawTitle(canvas, snapshot)
        drawStats(canvas, snapshot)
        canvas.fill(Layout.box, Palette.surface, radius: 6)
        let analysis = snapshot.analysis
        if let analysis, let total = snapshot.totalDuration, total > 0, !analysis.amplitude.isEmpty {
            DeckWaveform.draw(canvas, snapshot, analysis, total: total)
        } else {
            let style = TextStyle(size: 15, bold: true, color: Palette.faint, align: .center, maxWidth: 440)
            canvas.text("No analysis for this track", at: CGPoint(x: 240, y: 200), style)
        }
        return canvas.bitmap
    }

    private static func drawHeader(_ canvas: Canvas, _ snapshot: DeckSnapshot) {
        let label = TextStyle(size: 12, bold: true, color: Palette.amber)
        canvas.text("DECK \(snapshot.deck)", at: CGPoint(x: 12, y: 22), label)
        guard snapshot.hasTrack else { return }
        let state = TextStyle(
            size: 12, bold: true, color: snapshot.playing ? Palette.key : Palette.dim, align: .right
        )
        canvas.text(snapshot.playing ? "PLAYING" : "PAUSED", at: CGPoint(x: 468, y: 22), state)
    }

    private static func drawTitle(_ canvas: Canvas, _ snapshot: DeckSnapshot) {
        let title = TextStyle(size: 22, bold: true, color: Palette.text, maxWidth: 456)
        canvas.text(snapshot.title ?? "Unknown title", at: CGPoint(x: 12, y: 50), title)
        if let artist = snapshot.artist {
            let style = TextStyle(size: 15, color: Palette.dim, maxWidth: 456)
            canvas.text(artist, at: CGPoint(x: 12, y: 72), style)
        }
    }

    private static func drawStats(_ canvas: Canvas, _ snapshot: DeckSnapshot) {
        let remaining = snapshot.remaining ?? snapshot.totalDuration.flatMap { total in
            snapshot.elapsed.map { max(0, total - $0) }
        }
        let elapsed = snapshot.position.map { DeckTime.format($0) }
        let left = remaining.map { DeckTime.format($0, negative: true) }
        let columns: [DeckStat] = [
            DeckStat(label: "ELAPSED", value: elapsed, color: Palette.text),
            DeckStat(label: "REMAINING", value: left, color: Palette.text),
            DeckStat(label: "BPM", value: snapshot.bpm, color: Palette.text),
            DeckStat(label: "KEY", value: snapshot.key, color: Palette.key)
        ]
        let caption = TextStyle(size: 10, bold: true, color: Palette.dim)
        for (index, column) in columns.enumerated() {
            let left = 12 + CGFloat(index) * 116
            canvas.text(column.label, at: CGPoint(x: left, y: 92), caption)
            let value = TextStyle(
                size: 22, bold: true, color: column.value == nil ? Palette.faint : column.color, maxWidth: 110
            )
            canvas.text(column.value ?? "\u{2013}", at: CGPoint(x: left, y: 116), value)
        }
    }
}

private struct DeckStat {
    let label: String
    let value: String?
    let color: RGB
}

enum DeckWaveform {
    static func draw(_ canvas: Canvas, _ snapshot: DeckSnapshot, _ analysis: DeckSnapshot.Analysis, total: Double) {
        typealias Layout = DeckPanelRenderer.Layout
        let width = Layout.innerWidth
        let playheadColumn = snapshot.position.map { Int((min(total, $0) / total * Double(width)).rounded(.down)) }
        let peak = max(1, Int(analysis.amplitude.max() ?? 1))
        for column in 0 ..< width {
            let first = Int(Double(column) / Double(width) * Double(analysis.amplitude.count))
            let next = Int(Double(column + 1) / Double(width) * Double(analysis.amplitude.count))
            let slice = analysis.amplitude[first ..< min(analysis.amplitude.count, max(first + 1, next))]
            let level = CGFloat(slice.max() ?? 0) / CGFloat(peak)
            let height = max(1, level * Layout.halfHeight)
            var color = bandColor(analysis, at: (Double(column) + 0.5) / Double(width) * total)
            if let playheadColumn, column < playheadColumn {
                color = blend(color, Palette.surface, 0.55)
            }
            let rect = CGRect(
                x: Layout.innerLeft + CGFloat(column), y: Layout.centerY - height, width: 1, height: height * 2
            )
            canvas.fill(rect, color)
        }
        drawTicks(canvas, analysis, total: total)
        drawCues(canvas, snapshot.cues, total: total)
        if let playheadColumn {
            let rect = CGRect(
                x: Layout.innerLeft + CGFloat(playheadColumn) - 1, y: Layout.box.minY + 2, width: 2,
                height: Layout.box.height - 4
            )
            canvas.fill(rect, Palette.text)
        }
    }

    private static func x(_ time: Double, total: Double) -> CGFloat {
        DeckPanelRenderer.Layout.innerLeft + CGFloat(time / total) * CGFloat(DeckPanelRenderer.Layout.innerWidth)
    }

    private static func drawTicks(_ canvas: Canvas, _ analysis: DeckSnapshot.Analysis, total: Double) {
        typealias Layout = DeckPanelRenderer.Layout
        if analysis.beats.count > 1 {
            let spacing = x(analysis.beats[1], total: total) - x(analysis.beats[0], total: total)
            if spacing >= Layout.minimumBeatSpacing {
                for beat in analysis.beats where beat >= 0 && beat <= total {
                    let rect = CGRect(
                        x: x(beat, total: total), y: Layout.beatTickTop, width: 1,
                        height: Layout.tickBottom - Layout.beatTickTop
                    )
                    canvas.fill(rect, Palette.faint)
                }
            }
        }
        for bar in analysis.bars where bar >= 0 && bar <= total {
            let rect = CGRect(
                x: x(bar, total: total), y: Layout.barTickTop, width: 1,
                height: Layout.tickBottom - Layout.barTickTop
            )
            canvas.fill(rect, Palette.dim)
        }
    }

    private static func drawCues(_ canvas: Canvas, _ cues: [DeckSnapshot.Cue], total: Double) {
        typealias Layout = DeckPanelRenderer.Layout
        for cue in cues where cue.time >= 0 && cue.time <= total {
            let color = cue.color.flatMap(parseHex) ?? Palette.amber
            let left = x(cue.time, total: total)
            canvas.fill(CGRect(x: left, y: Layout.cueTop, width: 1, height: 22), color)
            canvas.fill(CGRect(x: left, y: Layout.cueTop, width: 7, height: 7), color)
        }
    }

    static func parseHex(_ text: String) -> RGB? {
        let digits = text.hasPrefix("#") ? String(text.dropFirst()) : text
        guard digits.count == 6, let value = UInt32(digits, radix: 16) else { return nil }
        return RGB(UInt8(value >> 16 & 0xFF), UInt8(value >> 8 & 0xFF), UInt8(value & 0xFF))
    }

    private static func bandColor(_ analysis: DeckSnapshot.Analysis, at time: Double) -> RGB {
        guard analysis.bands.count == 3, analysis.bandRate > 0 else { return Palette.accent }
        let index = Int(time * analysis.bandRate)
        let weights = analysis.bands.map { $0.indices.contains(index) ? Double($0[index]) : 0 }
        let sum = weights.reduce(0, +)
        guard sum > 0 else { return Palette.accent }
        let colors = [DeckPanelRenderer.bass, DeckPanelRenderer.mid, DeckPanelRenderer.treble]
        func channel(_ pick: (RGB) -> UInt8) -> UInt8 {
            let mixed = zip(weights, colors).reduce(0.0) { $0 + $1.0 * Double(pick($1.1)) } / sum
            return UInt8(min(255, max(0, mixed.rounded())))
        }
        return RGB(channel(\.red), channel(\.green), channel(\.blue))
    }

    private static func blend(_ color: RGB, _ other: RGB, _ amount: Double) -> RGB {
        func mix(_ lhs: UInt8, _ rhs: UInt8) -> UInt8 {
            UInt8((Double(lhs) * (1 - amount) + Double(rhs) * amount).rounded())
        }
        return RGB(mix(color.red, other.red), mix(color.green, other.green), mix(color.blue, other.blue))
    }
}
