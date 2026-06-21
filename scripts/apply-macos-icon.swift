#!/usr/bin/env swift
/// macOS app icon: Apple 824/1024 safe zone + continuous squircle (radius 185.4 @ 824).
/// Usage: apply-macos-icon.swift <in.png> <out.png> [canvasSize]
import AppKit
import SwiftUI

guard CommandLine.arguments.count >= 3 else {
    fputs("Usage: apply-macos-icon.swift <in.png> <out.png> [canvasSize]\n", stderr)
    exit(1)
}

let srcPath = CommandLine.arguments[1]
let dstPath = CommandLine.arguments[2]
let canvas = CommandLine.arguments.count > 3 ? Int(CommandLine.arguments[3])! : 1024

// Apple macOS template (1024 reference): 824² artwork, radius 185.4, 100px gutter.
let contentFraction = 824.0 / 1024.0
let cornerRadiusFraction = 185.4 / 824.0

let c = CGFloat(canvas)
let canvasRect = CGRect(x: 0, y: 0, width: c, height: c)
let content = c * contentFraction
let inset = (c - content) / 2.0
let contentRect = CGRect(x: inset, y: inset, width: content, height: content)
let cornerRadius = content * cornerRadiusFraction

func continuousSquirclePath(in rect: CGRect, cornerRadius: CGFloat) -> CGPath {
    RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
        .path(in: rect)
        .cgPath
}

guard let src = NSImage(contentsOfFile: srcPath) else {
    fputs("Failed to load \(srcPath)\n", stderr)
    exit(1)
}

guard
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: canvas,
        pixelsHigh: canvas,
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    )
else {
    fputs("Failed to create bitmap\n", stderr)
    exit(1)
}

NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)

if let ctx = NSGraphicsContext.current?.cgContext {
    ctx.clear(canvasRect)
    ctx.saveGState()
    ctx.addPath(continuousSquirclePath(in: contentRect, cornerRadius: cornerRadius))
    ctx.clip()

    ctx.interpolationQuality = .high
    let srcSize = src.size
    let srcSide = min(srcSize.width, srcSize.height)
    let srcRect = CGRect(
        x: (srcSize.width - srcSide) / 2.0,
        y: (srcSize.height - srcSide) / 2.0,
        width: srcSide,
        height: srcSide
    )

    if let cgImage = src.cgImage(forProposedRect: nil, context: nil, hints: nil) {
        ctx.draw(cgImage, in: contentRect, byTiling: false)
    } else {
        src.draw(
            in: contentRect,
            from: srcRect,
            operation: .copy,
            fraction: 1.0,
            respectFlipped: false,
            hints: nil
        )
    }
    ctx.restoreGState()
}

NSGraphicsContext.restoreGraphicsState()

guard let png = rep.representation(using: .png, properties: [:]) else {
    fputs("Failed to export PNG\n", stderr)
    exit(1)
}

do {
    try png.write(to: URL(fileURLWithPath: dstPath))
} catch {
    fputs("Failed to write \(dstPath): \(error)\n", stderr)
    exit(1)
}
