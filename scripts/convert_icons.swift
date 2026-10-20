import Cocoa

func convertToRGBA(inputPath: String, outputPath: String, width: Int, height: Int) {
    guard let inputImage = NSImage(contentsOfFile: inputPath) else {
        print("Failed to load \(inputPath)")
        return
    }

    let colorSpace = CGColorSpaceCreateDeviceRGB()
    let bitmapInfo = CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue)

    guard let context = CGContext(
        data: nil,
        width: width,
        height: height,
        bitsPerComponent: 8,
        bytesPerRow: width * 4,
        space: colorSpace,
        bitmapInfo: bitmapInfo.rawValue
    ) else {
        print("Failed to create CGContext")
        return
    }

    let rect = CGRect(x: 0, y: 0, width: width, height: height)
    if let cgImage = inputImage.cgImage(forProposedRect: nil, context: nil, hints: nil) {
        context.draw(cgImage, in: rect)
    }

    guard let outputCGImage = context.makeImage() else {
        print("Failed to make CGImage")
        return
    }

    let rep = NSBitmapImageRep(cgImage: outputCGImage)
    guard let pngData = rep.representation(using: .png, properties: [:]) else {
        print("Failed to encode PNG")
        return
    }

    try? pngData.write(to: URL(fileURLWithPath: outputPath))
    print("Generated RGBA \(outputPath) (\(width)x\(height))")
}

let src = "/Users/neksand/.gemini/antigravity/brain/0347ed02-0e00-4001-a1ff-e1a2fc467bed/.user_uploaded/media_1792420441824.jpg"

convertToRGBA(inputPath: src, outputPath: "src-tauri/icons/32x32.png", width: 32, height: 32)
convertToRGBA(inputPath: src, outputPath: "src-tauri/icons/128x128.png", width: 128, height: 128)
convertToRGBA(inputPath: src, outputPath: "src-tauri/icons/128x128@2x.png", width: 256, height: 256)
convertToRGBA(inputPath: src, outputPath: "src-tauri/icons/icon.png", width: 512, height: 512)
convertToRGBA(inputPath: src, outputPath: "src/assets/logo.png", width: 512, height: 512)
