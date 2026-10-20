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
        v = (HEIGHT - 1 - y) / (HEIGHT - 1)
        for x in range(WIDTH):
            u = x / (WIDTH - 1)
            
            # Smooth soft gradient across full mobile lid (v from 0.0 to 0.35)
            if v <= 0.35:
                lid = 0.88 + 0.12 * (1.0 - v / 0.35)
            else:
                lid = math.exp(-((v - 0.35) ** 1.6) / 0.14)
                
            u_alpha = smoothstep(0.02, 0.20, u) * smoothstep(1.0, 0.85, u)
            total_alpha = lid * u_alpha
            if v > 0.88:
                total_alpha *= smoothstep(1.0, 0.88, v)
                
            alpha = max(0.0, min(1.0, total_alpha * 0.96))
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
            
            # Full dense mobile lid coverage with deep outer V
            if v <= 0.40:
                lid = 0.92 + 0.08 * (1.0 - v / 0.40)
            else:
                lid = math.exp(-((v - 0.40) ** 1.6) / 0.12)
                
            outer_v = math.exp(-((u - 0.85)**2) / 0.10) * math.exp(-((v - 0.35)**2) / 0.14)
            
            u_alpha = smoothstep(0.02, 0.18, u)
            total_alpha = max(lid * 0.90, outer_v * 0.95) * u_alpha
            if v > 0.90:
                total_alpha *= smoothstep(1.0, 0.90, v)
                
            alpha = max(0.0, min(1.0, total_alpha * 0.98))
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
            
            # Full mobile lid base
            if v <= 0.35:
                lid = 0.88 + 0.12 * (1.0 - v / 0.35)
            else:
                lid = math.exp(-((v - 0.35) ** 1.8) / 0.10)
                
            # Wing tilted upward at outer edge
            wing_center_v = 0.12 + max(0.0, (u - 0.55)) * 1.05
            wing_dist = abs(v - wing_center_v)
            wing = math.exp(-(wing_dist ** 2) / 0.05) * smoothstep(0.55, 0.95, u) if u >= 0.55 else 0.0
            
            total_alpha = max(lid * 0.85, wing * 0.98) * smoothstep(0.02, 0.20, u)
            if v > 0.88:
                total_alpha *= smoothstep(1.0, 0.88, v)
                
            alpha = max(0.0, min(1.0, total_alpha))
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
            
            # Halo eye: full lid coverage with luminous center highlight
            if v <= 0.40:
                lid = 0.88 + 0.10 * (1.0 - v / 0.40)
            else:
                lid = math.exp(-((v - 0.40) ** 1.6) / 0.12)
                
            halo_center = math.exp(-((u - 0.50)**2) / 0.07) * math.exp(-((v - 0.25)**2) / 0.09)
            
            total_alpha = max(lid * 0.85, halo_center * 0.95) * smoothstep(0.02, 0.18, u)
            if v > 0.90:
                total_alpha *= smoothstep(1.0, 0.90, v)
                
            alpha = max(0.0, min(1.0, total_alpha))
            val = int(255 * (0.80 + 0.20 * math.exp(-((u - 0.50)**2) / 0.05)))
            a = int(alpha * 255)
            pixels[x, y] = (val, val, val, a)
    return img

def create_grunge_kohl():
    img = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0))
    pixels = img.load()
    for y in range(HEIGHT):
        v = (HEIGHT - 1 - y) / (HEIGHT - 1)
        for x in range(WIDTH):
            u = x / (WIDTH - 1)
            
            # 360 Rocker Smokey: Full intense solid kohl on mobile lid + seamless brow diffusion
            if v <= 0.45:
                lid_base = 0.95 + 0.05 * (1.0 - v / 0.45)
            else:
                lid_base = math.exp(-((v - 0.45) ** 1.5) / 0.10)
                
            crease_smoke = math.exp(-((v - 0.40) ** 2) / 0.18) * 0.85
            u_spread = smoothstep(0.01, 0.14, u) * smoothstep(1.0, 0.92, u)
            
            total_alpha = max(lid_base, crease_smoke) * u_spread
            if v > 0.92:
                total_alpha *= smoothstep(1.0, 0.92, v)
                
            alpha = max(0.0, min(1.0, total_alpha * 0.99))
            val = int(255 * (0.90 - 0.20 * v))
            a = int(alpha * 255)
            pixels[x, y] = (val, val, val, a)
    return img

def create_foxy_glam():
    img = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0))
    pixels = img.load()
    for y in range(HEIGHT):
        v = (HEIGHT - 1 - y) / (HEIGHT - 1)
        for x in range(WIDTH):
            u = x / (WIDTH - 1)
            
            # Full mobile lid coverage from lashline (v=0) to cut crease (v=0.45)
            if v <= 0.42:
                mobile_lid = 0.92 + 0.08 * (1.0 - v / 0.42)
            else:
                mobile_lid = math.exp(-((v - 0.42) ** 1.6) / 0.08)
                
            # Sharp outer wing lift starting from mid-eye (u >= 0.45)
            wing_center_v = 0.16 + max(0.0, (u - 0.45)) * 1.05
            wing = math.exp(-((v - wing_center_v) ** 2) / 0.05) * smoothstep(0.45, 0.92, u)
            
            u_spread = smoothstep(0.02, 0.20, u)
            total_alpha = max(mobile_lid * 0.92, wing * 0.98) * u_spread
            if v > 0.88:
                total_alpha *= smoothstep(1.0, 0.88, v)
                
            alpha = max(0.0, min(1.0, total_alpha))
            val = 255
            a = int(alpha * 255)
            pixels[x, y] = (val, val, val, a)
    return img

def create_glitter_pop():
    img = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0))
    pixels = img.load()
    for y in range(HEIGHT):
        v = (HEIGHT - 1 - y) / (HEIGHT - 1)
        for x in range(WIDTH):
            u = x / (WIDTH - 1)
            
            # Festival sparkle density on mobile lid
            base_wash = math.exp(-((v - 0.25) ** 2) / 0.16) * smoothstep(0.03, 0.25, u) * smoothstep(0.99, 0.82, u)
            
            # Pseudo-random micro-glitter stippling
            noise = (math.sin(x * 12.9898 + y * 78.233) * 43758.5453) % 1.0
            sparkle = noise if noise > 0.55 else 0.35
            
            total_alpha = base_wash * (0.65 + 0.35 * sparkle)
            if v > 0.88:
                total_alpha *= smoothstep(1.0, 0.88, v)
                
            alpha = max(0.0, min(1.0, total_alpha * 0.98))
            val = int(255 * (0.80 + 0.20 * sparkle))
            a = int(alpha * 255)
            pixels[x, y] = (val, val, val, a)
    return img

def main():
    generators = {
        "natural_nude.png": create_natural_nude,
        "smoky_glam.png": create_smoky_glam,
        "cat_wing.png": create_cat_wing,
        "sunset_glow.png": create_sunset_glow,
        "grunge_kohl.png": create_grunge_kohl,
        "foxy_glam.png": create_foxy_glam,
        "glitter_pop.png": create_glitter_pop,
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
