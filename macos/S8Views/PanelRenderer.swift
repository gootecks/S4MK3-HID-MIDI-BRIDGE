import CoreGraphics

enum Palette {
    static let background = RGB(8, 10, 14)
    static let surface = RGB(26, 31, 40)
    static let text = RGB(240, 243, 248)
    static let dim = RGB(150, 160, 176)
    static let faint = RGB(78, 86, 100)
    static let accent = RGB(0, 168, 255)
    static let amber = RGB(255, 184, 50)
    static let selection = RGB(18, 84, 164)
    static let selectionDim = RGB(196, 214, 238)
    static let key = RGB(104, 228, 156)
    static let onAccent = RGB(0, 8, 16)
}

public enum PanelRenderer {
    public static func render(_ side: PanelSide, _ state: ViewState) -> Bitmap {
        let canvas = Canvas()
        canvas.fill(CGRect(x: 0, y: 0, width: Metrics.width, height: Metrics.height), Palette.background)
        switch side {
        case .left: drawLeft(canvas, state)
        case .right: DetailPanel.draw(canvas, state)
        }
        return canvas.bitmap
    }

    static func drawStrip(_ canvas: Canvas, tabs: [SidebarSnapshot.Tab]) {
        let slotWidth = CGFloat(Metrics.width / ButtonSlots.count)
        let numberStyle = TextStyle(size: 10, color: Palette.faint, align: .center)
        for slot in 1 ... ButtonSlots.count {
            let left = CGFloat(slot - 1) * slotWidth
            let center = left + slotWidth / 2
            if let reserved = ButtonSlots.reserved[slot] {
                canvas.text(String(slot), at: CGPoint(x: center, y: 16), numberStyle)
                var style = numberStyle
                style.size = 12
                style.bold = true
                canvas.text(reserved, at: CGPoint(x: center, y: 34), style)
                continue
            }
            guard let index = ButtonSlots.tabIndex(forSlot: slot), tabs.indices.contains(index) else {
                canvas.text(String(slot), at: CGPoint(x: center, y: 16), numberStyle)
                continue
            }
            let tab = tabs[index]
            let box = CGRect(x: left + 2, y: 2, width: slotWidth - 4, height: Metrics.stripHeight - 6)
            canvas.fill(box, tab.selected ? Palette.accent : Palette.surface, radius: 5)
            let small = TextStyle(
                size: 10, bold: true, color: tab.selected ? Palette.onAccent : Palette.dim, align: .center
            )
            canvas.text(String(slot), at: CGPoint(x: center, y: 15), small)
            let label = TextStyle(
                size: 12, bold: true, color: tab.selected ? Palette.onAccent : Palette.text, align: .center,
                maxWidth: slotWidth - 8
            )
            canvas.text(tab.label, at: CGPoint(x: center, y: 33), label)
        }
    }

    private static func drawLeft(_ canvas: Canvas, _ state: ViewState) {
        let snapshot = state.snapshot
        drawStrip(canvas, tabs: snapshot.tabs)
        let heading = snapshot.heading ?? snapshot.tabs.first(where: \.selected)?.label ?? ""
        let headingStyle = TextStyle(size: 20, bold: true, color: Palette.text, maxWidth: 456)
        canvas.text(heading, at: CGPoint(x: 12, y: Metrics.headingTop + 22), headingStyle)
        let notice = TextStyle(size: 17, bold: true, color: Palette.dim, align: .center)
        guard snapshot.sidebarOpen else {
            canvas.text("Open the djay sidebar", at: CGPoint(x: 240, y: 168), notice)
            return
        }
        guard !snapshot.rows.isEmpty else {
            canvas.text("Nothing to show", at: CGPoint(x: 240, y: 168), notice)
            return
        }
        let listArea = CGRect(x: 0, y: Metrics.listTop, width: CGFloat(Metrics.width), height: Metrics.viewportHeight)
        canvas.context.saveGState()
        canvas.context.clip(to: listArea)
        for (index, row) in snapshot.rows.enumerated() {
            let top = Metrics.listTop - state.scrollTop + state.rowOffset(index)
            guard top + Metrics.rowHeight(row.kind) > Metrics.listTop, top < Metrics.listBottom else { continue }
            RowPainter.draw(canvas, row, top: top, selected: index == state.selectedIndex)
        }
        canvas.context.restoreGState()
        drawScrollbar(canvas, state)
    }

    private static func drawScrollbar(_ canvas: Canvas, _ state: ViewState) {
        guard state.contentHeight > Metrics.viewportHeight else { return }
        let thumb = max(16, Metrics.viewportHeight * Metrics.viewportHeight / state.contentHeight)
        let travel = Metrics.viewportHeight - thumb
        let position = travel * state.scrollTop / (state.contentHeight - Metrics.viewportHeight)
        let bar = CGRect(x: 475, y: Metrics.listTop + position, width: 3, height: thumb)
        canvas.fill(bar, Palette.dim, radius: 1.5)
    }
}

enum RowPainter {
    static func draw(_ canvas: Canvas, _ row: SidebarSnapshot.Row, top: CGFloat, selected: Bool) {
        let height = Metrics.rowHeight(row.kind)
        switch row.kind {
        case .header:
            canvas.fill(CGRect(x: 4, y: top + 1, width: 466, height: height - 2), Palette.surface, radius: 3)
            let style = TextStyle(size: 11, bold: true, color: Palette.amber, maxWidth: 450)
            canvas.text(row.text ?? "", at: CGPoint(x: 12, y: top + 15), style)
        case .action:
            let box = CGRect(x: 4, y: top + 2, width: 466, height: height - 4)
            if selected {
                canvas.fill(box, Palette.selection, radius: 6)
            }
            canvas.stroke(box, selected ? Palette.accent : Palette.faint, radius: 6)
            let style = TextStyle(
                size: 13, bold: true, color: selected ? Palette.text : Palette.accent, align: .center,
                maxWidth: 440
            )
            canvas.text(row.text ?? "", at: CGPoint(x: 237, y: top + 20), style)
        case .track, .unknown:
            drawTrack(canvas, row, top: top, selected: selected)
        }
    }

    private static func drawTrack(_ canvas: Canvas, _ row: SidebarSnapshot.Row, top: CGFloat, selected: Bool) {
        if selected {
            let box = CGRect(x: 4, y: top + 1, width: 466, height: Metrics.trackRowHeight - 2)
            canvas.fill(box, Palette.selection, radius: 5)
        }
        let secondary = selected ? Palette.selectionDim : Palette.dim
        let title = TextStyle(size: 14, bold: true, color: Palette.text, maxWidth: 286)
        canvas.text(row.title ?? row.text ?? "", at: CGPoint(x: 12, y: top + 15), title)
        if let artist = row.artist {
            let style = TextStyle(size: 11, color: secondary, maxWidth: 286)
            canvas.text(artist, at: CGPoint(x: 12, y: top + 29), style)
        }
        let value = TextStyle(size: 15, bold: true, color: Palette.text, align: .right)
        let key = TextStyle(size: 15, bold: true, color: Palette.key, align: .right)
        let time = TextStyle(size: 13, color: secondary, align: .right)
        drawColumn(canvas, row.bpm, right: 352, top: top, value)
        drawColumn(canvas, row.key, right: 410, top: top, key)
        drawColumn(canvas, row.time, right: 464, top: top, time)
    }

    private static func drawColumn(_ canvas: Canvas, _ text: String?, right: CGFloat, top: CGFloat, _ look: TextStyle) {
        guard let text else { return }
        canvas.text(text, at: CGPoint(x: right, y: top + 22), look)
    }
}

enum DetailPanel {
    static func draw(_ canvas: Canvas, _ state: ViewState) {
        let snapshot = state.snapshot
        PanelRenderer.drawStrip(canvas, tabs: snapshot.subtabs)
        let labelBaseline = Metrics.headingTop + 20
        let label = TextStyle(size: 12, bold: true, color: Palette.amber)
        canvas.text("SELECTED", at: CGPoint(x: 12, y: labelBaseline), label)
        guard let track = state.selectedTrack else {
            let style = TextStyle(size: 18, bold: true, color: Palette.dim, align: .center, maxWidth: 440)
            canvas.text(state.selectedRow?.text ?? "No track selected", at: CGPoint(x: 240, y: 168), style)
            return
        }
        if let index = state.selectedIndex {
            let tracks = snapshot.rows.filter { $0.kind == .track }.count
            let position = snapshot.rows[...index].filter { $0.kind == .track }.count
            let count = TextStyle(size: 12, color: Palette.dim, align: .right)
            canvas.text("\(position) of \(tracks)", at: CGPoint(x: 468, y: labelBaseline), count)
        }
        drawTitle(canvas, track)
        drawTiles(canvas, track)
    }

    private static func drawTitle(_ canvas: Canvas, _ track: SidebarSnapshot.Row) {
        let style = TextStyle(size: 26, bold: true, color: Palette.text, maxWidth: 456)
        let lines = canvas.wrap(track.title ?? "", style, maxWidth: 456, maxLines: 2)
        for (index, line) in lines.enumerated() {
            canvas.text(line, at: CGPoint(x: 12, y: 108 + CGFloat(index) * 30), style)
        }
        if let artist = track.artist {
            let artistStyle = TextStyle(size: 17, color: Palette.dim, maxWidth: 456)
            canvas.text(artist, at: CGPoint(x: 12, y: 114 + CGFloat(lines.count) * 30), artistStyle)
        }
    }

    private static func drawTiles(_ canvas: Canvas, _ track: SidebarSnapshot.Row) {
        drawTile(canvas, 0, caption: "BPM", value: track.bpm, color: Palette.text)
        drawTile(canvas, 1, caption: "KEY", value: track.key, color: Palette.key)
        drawTile(canvas, 2, caption: "TIME", value: track.time, color: Palette.text)
    }

    private static func drawTile(_ canvas: Canvas, _ index: Int, caption: String, value: String?, color: RGB) {
        let box = CGRect(x: 12 + CGFloat(index) * 154, y: 186, width: 148, height: 74)
        canvas.fill(box, Palette.surface, radius: 8)
        let captionStyle = TextStyle(size: 11, bold: true, color: Palette.dim)
        canvas.text(caption, at: CGPoint(x: box.minX + 12, y: box.minY + 20), captionStyle)
        let valueStyle = TextStyle(size: 36, bold: true, color: value == nil ? Palette.faint : color, align: .center)
        canvas.text(value ?? "\u{2013}", at: CGPoint(x: box.midX, y: box.minY + 58), valueStyle)
    }
}
