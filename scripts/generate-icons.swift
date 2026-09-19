// Renders the app icon source, the macOS tray template and the drag preview.
// Usage: swift scripts/generate-icons.swift && pnpm tauri icon src-tauri/icons/app-icon.png
import AppKit

func render(_ size: Int, to path: String, draw: (CGContext, CGFloat) -> Void) {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size, bitsPerSample: 8,
                               samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    let context = NSGraphicsContext(bitmapImageRep: rep)!
    NSGraphicsContext.current = context
    let cg = context.cgContext
    // Flip to a top-left origin, which matches how the shapes are described.
    cg.translateBy(x: 0, y: CGFloat(size))
    cg.scaleBy(x: 1, y: -1)
    draw(cg, CGFloat(size))
    NSGraphicsContext.restoreGraphicsState()
    try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: path))
}

/// Notch silhouette: concave ears at the top, rounded bottom corners.
func notchPath(x: CGFloat, width: CGFloat, height: CGFloat, ear: CGFloat, bottom: CGFloat) -> CGPath {
    let p = CGMutablePath()
    p.move(to: CGPoint(x: x - ear, y: 0))
    p.addQuadCurve(to: CGPoint(x: x, y: ear), control: CGPoint(x: x, y: 0))
    p.addLine(to: CGPoint(x: x, y: height - bottom))
    p.addQuadCurve(to: CGPoint(x: x + bottom, y: height), control: CGPoint(x: x, y: height))
    p.addLine(to: CGPoint(x: x + width - bottom, y: height))
    p.addQuadCurve(to: CGPoint(x: x + width, y: height - bottom), control: CGPoint(x: x + width, y: height))
    p.addLine(to: CGPoint(x: x + width, y: ear))
    p.addQuadCurve(to: CGPoint(x: x + width + ear, y: 0), control: CGPoint(x: x + width, y: 0))
    p.closeSubpath()
    return p
}

func rgb(_ hex: UInt32, _ alpha: CGFloat = 1) -> CGColor {
    CGColor(red: CGFloat((hex >> 16) & 0xff) / 255, green: CGFloat((hex >> 8) & 0xff) / 255,
            blue: CGFloat(hex & 0xff) / 255, alpha: alpha)
}

let ink = rgb(0x0D0F12)
let dir = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "src-tauri/icons"

/// A mint tile with the notch and its fringe in ink: the app's accent, inverted so the
/// icon stays legible at 32px and against a dark Dock or Finder row.
render(1024, to: "\(dir)/app-icon.png") { cg, s in
    let inset: CGFloat = 100
    let tile = CGRect(x: inset, y: inset, width: s - inset * 2, height: s - inset * 2)
    cg.addPath(CGPath(roundedRect: tile, cornerWidth: 185, cornerHeight: 185, transform: nil))
    cg.clip()
    let gradient = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                              colors: [rgb(0xB8FFCE), rgb(0x3FCE8A)] as CFArray,
                              locations: [0, 1])!
    cg.drawLinearGradient(gradient, start: CGPoint(x: inset, y: inset),
                          end: CGPoint(x: s - inset, y: s - inset), options: [])

    // The fringe: strands hanging out of the notch, which is drawn over their tops.
    let notchWidth: CGFloat = 600, notchHeight: CGFloat = 196
    let notchX = (s - notchWidth) / 2, notchBottom = inset + notchHeight
    let strokeWidth: CGFloat = 52, gap: CGFloat = 28
    let lengths: [CGFloat] = [222, 318, 268, 412, 292, 350, 232]
    let total = CGFloat(lengths.count) * strokeWidth + CGFloat(lengths.count - 1) * gap
    cg.saveGState()
    cg.setShadow(offset: CGSize(width: 0, height: -18), blur: 46, color: rgb(0x0B3A24, 0.35))
    for (index, length) in lengths.enumerated() {
        let x = (s - total) / 2 + CGFloat(index) * (strokeWidth + gap)
        let rect = CGRect(x: x, y: notchBottom - 76, width: strokeWidth, height: length)
        cg.addPath(CGPath(roundedRect: rect, cornerWidth: strokeWidth / 2, cornerHeight: strokeWidth / 2, transform: nil))
    }
    cg.setFillColor(ink)
    cg.fillPath()
    cg.restoreGState()

    cg.saveGState()
    cg.translateBy(x: 0, y: inset - 2)
    cg.setShadow(offset: CGSize(width: 0, height: -16), blur: 44, color: rgb(0x0B3A24, 0.45))
    cg.addPath(notchPath(x: notchX, width: notchWidth, height: notchHeight, ear: 44, bottom: 84))
    cg.setFillColor(ink)
    cg.fillPath()
    cg.restoreGState()

    // A hairline rim keeps the tile edge crisp on a light background.
    cg.addPath(CGPath(roundedRect: tile.insetBy(dx: 1.5, dy: 1.5), cornerWidth: 183.5, cornerHeight: 183.5, transform: nil))
    cg.setStrokeColor(rgb(0xFFFFFF, 0.30))
    cg.setLineWidth(3)
    cg.strokePath()
}

render(44, to: "\(dir)/tray-template.png") { cg, s in
    let screen = CGRect(x: 4, y: 8, width: s - 8, height: s - 16)
    cg.setStrokeColor(CGColor(gray: 0, alpha: 1))
    cg.setLineWidth(3)
    cg.addPath(CGPath(roundedRect: screen, cornerWidth: 6, cornerHeight: 6, transform: nil))
    cg.strokePath()
    cg.translateBy(x: 0, y: 8)
    cg.addPath(notchPath(x: 14, width: 16, height: 9, ear: 3, bottom: 4))
    cg.setFillColor(CGColor(gray: 0, alpha: 1))
    cg.fillPath()
}

render(128, to: "\(dir)/drag-file.png") { cg, s in
    let page = CGMutablePath()
    page.move(to: CGPoint(x: 24, y: 10))
    page.addLine(to: CGPoint(x: 80, y: 10))
    page.addLine(to: CGPoint(x: 104, y: 34))
    page.addLine(to: CGPoint(x: 104, y: 118))
    page.addLine(to: CGPoint(x: 24, y: 118))
    page.closeSubpath()
    cg.setShadow(offset: CGSize(width: 0, height: -2), blur: 6, color: CGColor(gray: 0, alpha: 0.35))
    cg.addPath(page)
    cg.setFillColor(CGColor(gray: 0.97, alpha: 1))
    cg.fillPath()
    cg.setShadow(offset: .zero, blur: 0, color: nil)
    let fold = CGMutablePath()
    fold.move(to: CGPoint(x: 80, y: 10))
    fold.addLine(to: CGPoint(x: 80, y: 34))
    fold.addLine(to: CGPoint(x: 104, y: 34))
    fold.closeSubpath()
    cg.addPath(fold)
    cg.setFillColor(CGColor(gray: 0.82, alpha: 1))
    cg.fillPath()
}
