# Placeholder app icon: three stacked stones drawn on a 16px grid. Replace with a final logo before release.
from PIL import Image, ImageDraw
G = 16
img = Image.new("RGBA", (G, G), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
d.rounded_rectangle([0, 0, G - 1, G - 1], radius=3, fill=(28, 26, 24, 255))
stone, shade, lichen = (168, 160, 148, 255), (120, 113, 104, 255), (232, 163, 61, 255)
d.rectangle([3, 11, 12, 13], fill=stone); d.rectangle([3, 13, 12, 13], fill=shade)
d.rectangle([4, 7, 11, 9], fill=stone); d.rectangle([4, 9, 11, 9], fill=shade)
d.rectangle([6, 3, 9, 5], fill=lichen); d.rectangle([6, 5, 9, 5], fill=(190, 128, 40, 255))
out = "src-tauri/icons/"
for size, name in [(32, "32x32.png"), (128, "128x128.png"), (256, "128x128@2x.png"), (512, "icon.png")]:
    img.resize((size, size), Image.NEAREST).save(out + name)
img.resize((256, 256), Image.NEAREST).save("public/icon.png")
