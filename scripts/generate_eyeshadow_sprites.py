import math
import os
from PIL import Image

WIDTH = 256
HEIGHT = 256

def smoothstep(edge0, edge1, x):
    t = max(0.0, min(1.0, (x - edge0) / (edge1 - edge0)))
    return t * t * (3.0 - 2.0 * t)

def create_natural_nude():
    img = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0))
    pixels = img.load()
    for y in range(HEIGHT):
        v = (HEIGHT - 1 - y) / (HEIGHT - 1) # 0 at bottom (lashline), 1 at top (brow)
        for x in range(WIDTH):
            u = x / (WIDTH - 1) # 0 at inner corner, 1 at outer corner
            
            # Vertical blend: strong at lashline, soft decay towards brow
            v_alpha = math.exp(-((v - 0.12) ** 2) / 0.18) if v >= 0.05 else smoothstep(0.0, 0.05, v)
            if v > 0.85:
                v_alpha *= smoothstep(1.0, 0.85, v)
            
            # Horizontal blend: softer on inner, gentle build-up on outer
            u_alpha = smoothstep(0.02, 0.25, u) * smoothstep(1.0, 0.80, u)
            
            alpha = max(0.0, min(1.0, v_alpha * u_alpha * 0.95))
            
            # Grayscale intensity for tinting (luminous core on center)
            center_luma = 0.85 + 0.15 * math.exp(-((u - 0.5) ** 2) / 0.08)
            val = int(255 * center_luma)
            a = int(alpha * 255)
            pixels[x, y] = (val, val, val, a)
    return img

def create_smoky_glam():
    img = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0))
    pixels = img.load()
    for y in range(HEIGHT):
        v = (HEIGHT - 1 - y) / (HEIGHT - 1)
        for x in range(WIDTH):
            u = x / (WIDTH - 1)
            
            # Lash line density + deep outer V
            lash_v = math.exp(-(v ** 2) / 0.08)
            outer_v = math.exp(-((u - 0.85)**2) / 0.09) * math.exp(-((v - 0.35)**2) / 0.12)
            
            v_alpha = lash_v * 0.75 + outer_v * 0.85
            if v < 0.03:
                v_alpha *= smoothstep(0.0, 0.03, v)
            if v > 0.88:
                v_alpha *= smoothstep(1.0, 0.88, v)
                
            u_alpha = smoothstep(0.04, 0.22, u)
            alpha = max(0.0, min(1.0, (v_alpha * u_alpha) * 0.98))
            
            # Outer has deeper smoke
            val = int(255 * (0.95 - 0.25 * u))
            a = int(alpha * 255)
            pixels[x, y] = (val, val, val, a)
    return img

def create_cat_wing():
    img = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0))
    pixels = img.load()
    for y in range(HEIGHT):
        v = (HEIGHT - 1 - y) / (HEIGHT - 1)
        for x in range(WIDTH):
            u = x / (WIDTH - 1)
            
            # Lash line gradient
            lash = math.exp(-((v - 0.08)**2) / 0.06) * smoothstep(0.05, 0.4, u)
            
            # Wing tilted upward at outer edge: centerline v_wing = 0.18 + (u - 0.75) * 0.8
            wing_center_v = 0.12 + max(0.0, (u - 0.65)) * 0.95
            wing_dist = abs(v - wing_center_v)
            wing = math.exp(-(wing_dist ** 2) / 0.04) * smoothstep(0.65, 0.92, u) if u >= 0.65 else 0.0
            
            total_alpha = max(lash * 0.75, wing * 0.95)
            if v < 0.02:
                total_alpha *= smoothstep(0.0, 0.02, v)
            if v > 0.85:
                total_alpha *= smoothstep(1.0, 0.85, v)
                
            alpha = max(0.0, min(1.0, total_alpha * smoothstep(0.05, 0.25, u)))
            val = 255
            a = int(alpha * 255)
            pixels[x, y] = (val, val, val, a)
    return img

def create_sunset_glow():
    img = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0))
    pixels = img.load()
    for y in range(HEIGHT):
        v = (HEIGHT - 1 - y) / (HEIGHT - 1)
        for x in range(WIDTH):
            u = x / (WIDTH - 1)
            
            # Halo eye: bright and luminous in center, soft on edges
            halo_center = math.exp(-((u - 0.50)**2) / 0.06) * math.exp(-((v - 0.25)**2) / 0.08)
            surround = math.exp(-((v - 0.20)**2) / 0.15) * smoothstep(0.08, 0.35, u) * smoothstep(0.95, 0.75, u)
            
            total_alpha = (halo_center * 0.90 + surround * 0.55)
            if v < 0.03:
                total_alpha *= smoothstep(0.0, 0.03, v)
            if v > 0.85:
                total_alpha *= smoothstep(1.0, 0.85, v)
                
            alpha = max(0.0, min(1.0, total_alpha))
            # Halo center has bright highlight
            val = int(255 * (0.80 + 0.20 * math.exp(-((u - 0.50)**2) / 0.05)))
            a = int(alpha * 255)
            pixels[x, y] = (val, val, val, a)
    return img

def main():
    generators = {
        "natural_nude.png": create_natural_nude,
        "smoky_glam.png": create_smoky_glam,
        "cat_wing.png": create_cat_wing,
        "sunset_glow.png": create_sunset_glow,
    }

    target_dirs = [
        "d:/proyectos/CamAr Studio/ui/public/sprites/eyeshadow",
        "d:/proyectos/CamAr Studio/engine-rs/assets/sprites/eyeshadow",
        "d:/proyectos/CamAr Studio/release/CamAr-Studio-v0.1.0-beta-Portable/sprites/eyeshadow",
    ]

    for d in target_dirs:
        os.makedirs(d, exist_ok=True)

    for filename, fn in generators.items():
        print(f"Generating {filename}...")
        img = fn()
        for d in target_dirs:
            out_path = os.path.join(d, filename)
            img.save(out_path, "PNG")
            print(f" -> Saved to {out_path}")

    print("All eyeshadow sprites generated successfully!")

if __name__ == "__main__":
    main()
