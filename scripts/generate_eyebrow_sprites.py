import math
import random
import os
from PIL import Image, ImageDraw

WIDTH = 256
HEIGHT = 128

def smoothstep(e0, e1, x):
    t = max(0.0, min(1.0, (x - e0) / (e1 - e0)))
    return t * t * (3.0 - 2.0 * t)

def generate_style(style="defined_arch"):
    img_powder = Image.new('RGBA', (WIDTH, HEIGHT), (0, 0, 0, 0))
    pix_powder = img_powder.load()

    # High-res canvas for anti-aliased hair strokes
    img_strokes = Image.new('RGBA', (WIDTH * 2, HEIGHT * 2), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img_strokes)

    # 1. BASE POWDER SILHOUETTE (Relleno cosmético denso y esculpido)
    for y in range(HEIGHT):
        v = (HEIGHT - 1 - y) / (HEIGHT - 1.0) # 0 = borde inferior, 1 = borde superior
        for x in range(WIDTH):
            u = x / (WIDTH - 1.0) # 0 = cabeza (nariz), 1 = cola (sien)
            
            if style == "defined_arch":
                # Arco Bigo: base curvada limpia, arco alto en u=0.62 y cola afilada
                v_bot = 0.22 + 0.14 * math.sin(u * math.pi * 0.75) * (1.0 - 0.25 * u)
                arch_lift = math.sin(min(1.0, u / 0.62) * (math.pi * 0.5))
                v_top = 0.68 + 0.25 * arch_lift - 0.72 * (max(0.0, u - 0.62) / 0.38) ** 1.45
            elif style == "korean_straight":
                # Recta Coreana: base horizontal definida, grosor juvenil uniforme
                v_bot = 0.24 + 0.05 * math.sin(u * math.pi * 0.8)
                v_top = 0.74 - 0.42 * (max(0.0, u - 0.70) / 0.30) ** 1.5
            elif style == "fluffy_feathered":
                # Laminada / Pluma: arco natural con margen superior para vellos peinados
                v_bot = 0.20 + 0.12 * math.sin(u * math.pi * 0.75)
                arch_lift = math.sin(min(1.0, u / 0.60) * (math.pi * 0.5))
                v_top = 0.72 + 0.22 * arch_lift - 0.68 * (max(0.0, u - 0.60) / 0.40) ** 1.4
            else: # natural_fill
                # Relleno Natural: forma clásica equilibrada, suave arco y cola pulida
                v_bot = 0.22 + 0.10 * math.sin(u * math.pi * 0.75)
                arch_lift = math.sin(min(1.0, u / 0.58) * (math.pi * 0.5))
                v_top = 0.70 + 0.18 * arch_lift - 0.66 * (max(0.0, u - 0.58) / 0.42) ** 1.4

            if v_top <= v_bot:
                continue

            thick = v_top - v_bot
            if v >= v_bot and v <= v_top:
                v_norm = (v - v_bot) / max(1e-4, thick)
                # Borde inferior nítido de navaja
                edge_bot = smoothstep(0.0, 0.05, v_norm)
                # Borde superior con caída suave
                edge_top = smoothstep(1.0, 0.88, v_norm)
                # Inicio ombré en el nacimiento (junto al entrecejo)
                head_ombre = smoothstep(0.02, 0.16, u)
                # Punta afilada en la cola
                tail_taper = smoothstep(0.98, 0.88, u)
                
                # Densidad sólida de polvo base (95% opaco en el centro)
                alpha = 0.95 * edge_bot * edge_top * head_ombre * tail_taper
                lum = int(255 * (0.50 + 0.50 * v_norm))
                pix_powder[x, y] = (lum, lum, lum, int(alpha * 255))

    # 2. MICROBLADING HAIR STROKES (Trazos de vello pelo a pelo)
    random.seed(1337 if style == "defined_arch" else 42)
    num_hairs = 90 if style != "korean_straight" else 65

    for i in range(num_hairs):
        u_start = (i / float(num_hairs)) * 0.92 + random.uniform(-0.015, 0.015)
        u_start = max(0.02, min(0.94, u_start))

        if style == "defined_arch":
            v_bot = 0.22 + 0.14 * math.sin(u_start * math.pi * 0.75) * (1.0 - 0.25 * u_start)
            arch_lift = math.sin(min(1.0, u_start / 0.62) * (math.pi * 0.5))
            v_top = 0.68 + 0.25 * arch_lift - 0.72 * (max(0.0, u_start - 0.62) / 0.38) ** 1.45
        elif style == "korean_straight":
            v_bot = 0.24 + 0.05 * math.sin(u_start * math.pi * 0.8)
            v_top = 0.74 - 0.42 * (max(0.0, u_start - 0.70) / 0.30) ** 1.5
        elif style == "fluffy_feathered":
            v_bot = 0.20 + 0.12 * math.sin(u_start * math.pi * 0.75)
            arch_lift = math.sin(min(1.0, u_start / 0.60) * (math.pi * 0.5))
            v_top = 0.72 + 0.22 * arch_lift - 0.68 * (max(0.0, u_start - 0.60) / 0.40) ** 1.4
        else:
            v_bot = 0.22 + 0.10 * math.sin(u_start * math.pi * 0.75)
            arch_lift = math.sin(min(1.0, u_start / 0.58) * (math.pi * 0.5))
            v_top = 0.70 + 0.18 * arch_lift - 0.66 * (max(0.0, u_start - 0.58) / 0.42) ** 1.4

        if v_top <= v_bot:
            continue

        thick = v_top - v_bot
        v_start = v_bot + random.uniform(0.0, 0.35) * thick
        
        # En estilo fluffy_feathered, los pelitos sobresalen más arriba
        extra_top = 0.15 if style == "fluffy_feathered" else 0.08
        hair_len_v = random.uniform(0.60, 0.98) * thick
        v_end = min(v_top + random.uniform(-0.02, extra_top) * thick, v_start + hair_len_v)
        
        # Ángulo anatómico según la zona de la ceja:
        if u_start < 0.18:
            dx_per_v = random.uniform(0.06, 0.16) # Casi vertical en el nacimiento
        elif u_start < 0.65:
            dx_per_v = random.uniform(0.24, 0.42) # Diagonal hacia el arco
        else:
            dx_per_v = random.uniform(0.40, 0.72) # Caída hacia la sien

        pts = []
        steps = 8
        for s in range(steps + 1):
            t = s / float(steps)
            curr_v = v_start + (v_end - v_start) * t
            curve_drift = (t ** 1.3) * (v_end - v_start) * dx_per_v
            curr_u = u_start + curve_drift
            
            x_px = curr_u * (WIDTH * 2 - 1)
            y_px = (1.0 - curr_v) * (HEIGHT * 2 - 1)
            pts.append((x_px, y_px))

        head_fade = smoothstep(0.01, 0.15, u_start)
        tail_fade = smoothstep(0.96, 0.88, u_start)
        hair_alpha = int(255 * 0.96 * head_fade * tail_fade)
        stroke_width = 3 if (u_start > 0.15 and u_start < 0.75) else 2
        
        draw.line(pts, fill=(255, 255, 255, hair_alpha), width=stroke_width)

    img_strokes_low = img_strokes.resize((WIDTH, HEIGHT), Image.Resampling.LANCZOS)
    final_img = Image.alpha_composite(img_powder, img_strokes_low)
    return final_img

def main():
    styles = {
        "natural_fill": generate_style("natural_fill"),
        "defined_arch": generate_style("defined_arch"),
        "korean_straight": generate_style("korean_straight"),
        "fluffy_feathered": generate_style("fluffy_feathered"),
    }

    dirs = [
        "engine-rs/assets/sprites/eyebrows",
        "ui/public/sprites/eyebrows",
        "release/CamAr-Studio-v0.1.0-beta-Portable/sprites/eyebrows",
    ]

    for d in dirs:
        os.makedirs(d, exist_ok=True)
        for name, img in styles.items():
            path = os.path.join(d, f"{name}.png")
            img.save(path, "PNG")
            print(f"Saved: {path}")

if __name__ == "__main__":
    main()
