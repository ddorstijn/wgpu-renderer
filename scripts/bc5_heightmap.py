from PIL import Image
import numpy as np

# Load grayscale heightmap (16-bit PNG)
heightmap = Image.open("./assets/heightmap_big.png").convert("I")
data = np.array(heightmap)

# Normalize if values aren't in full 0–65535 range
# data = ((data - data.min()) / (data.max() - data.min()) * 65535).astype(np.uint16)

# Split into two channels
red = (data // 256).astype(np.uint8)  # High 8 bits
green = (data % 256).astype(np.uint8)  # Low 8 bits

# Stack into RG image
heightmap_rg = np.stack((red, green), axis=-1)

# Convert to image and save
heightmap_image = Image.fromarray(heightmap_rg)
heightmap_image.save("heightmap_rg.png", format="DXT1")

print("✅ Heightmap converted to RG format ready for BC5 compression.")
