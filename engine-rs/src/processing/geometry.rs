//! Módulo de Primitivas Geométricas y Muestreo de Curvas Anatómicas.
//!
//! Responsabilidad única (SRP): Estructuras 2D y triangulaciones baricéntricas (UV mapping),
//! cálculo de distancias ortogonales e interpolación lineal continua de contornos faciales.

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct LineSegment2D {
    pub ax: f32,
    pub ay: f32,
    pub bx: f32,
    pub by: f32,
    pub abx: f32,
    pub aby: f32,
    pub inv_len_sq: f32,
}

impl LineSegment2D {
    #[inline(always)]
    pub fn new(ax: f32, ay: f32, bx: f32, by: f32) -> Self {
        let abx = bx - ax;
        let aby = by - ay;
        let len_sq = abx * abx + aby * aby;
        let inv_len_sq = if len_sq > 1e-5 { 1.0 / len_sq } else { 0.0 };
        Self { ax, ay, bx, by, abx, aby, inv_len_sq }
    }

    #[inline(always)]
    pub fn distance_and_side(&self, px: f32, py: f32) -> (f32, f32) {
        let apx = px - self.ax;
        let apy = py - self.ay;
        let t = ((apx * self.abx + apy * self.aby) * self.inv_len_sq).clamp(0.0, 1.0);
        let proj_x = self.ax + t * self.abx;
        let proj_y = self.ay + t * self.aby;
        let dx = px - proj_x;
        let dy = py - proj_y;
        let dist = (dx * dx + dy * dy).sqrt();
        let cross = self.abx * apy - self.aby * apx;
        (dist, cross)
    }
}

/// Triángulo 2D para rasterización continua y cálculo de coordenadas baricéntricas UV.
#[derive(Clone, Copy)]
pub struct MeshTriangle {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub p0: (f32, f32),
    pub p1: (f32, f32),
    pub p2: (f32, f32),
    pub uv0: (f32, f32),
    pub uv1: (f32, f32),
    pub uv2: (f32, f32),
    pub inv_denom: f32,
}

impl Default for MeshTriangle {
    fn default() -> Self {
        Self {
            min_x: 0.0, max_x: 0.0, min_y: 0.0, max_y: 0.0,
            p0: (0.0, 0.0), p1: (0.0, 0.0), p2: (0.0, 0.0),
            uv0: (0.0, 0.0), uv1: (0.0, 0.0), uv2: (0.0, 0.0),
            inv_denom: 0.0,
        }
    }
}

impl MeshTriangle {
    pub fn new(
        p0: (f32, f32), p1: (f32, f32), p2: (f32, f32),
        uv0: (f32, f32), uv1: (f32, f32), uv2: (f32, f32),
    ) -> Option<Self> {
        let denom = (p1.1 - p2.1) * (p0.0 - p2.0) + (p2.0 - p1.0) * (p0.1 - p2.1);
        if denom.abs() < 1e-4 {
            return None;
        }
        let inv_denom = 1.0 / denom;
        let min_x = p0.0.min(p1.0).min(p2.0);
        let max_x = p0.0.max(p1.0).max(p2.0);
        let min_y = p0.1.min(p1.1).min(p2.1);
        let max_y = p0.1.max(p1.1).max(p2.1);
        Some(Self {
            min_x, max_x, min_y, max_y,
            p0, p1, p2,
            uv0, uv1, uv2,
            inv_denom,
        })
    }

    #[inline(always)]
    pub fn sample_uv(&self, px: f32, py: f32) -> Option<(f32, f32)> {
        if px < self.min_x || px > self.max_x || py < self.min_y || py > self.max_y {
            return None;
        }
        let w0 = ((self.p1.1 - self.p2.1) * (px - self.p2.0) + (self.p2.0 - self.p1.0) * (py - self.p2.1)) * self.inv_denom;
        if w0 < -0.001 || w0 > 1.001 { return None; }
        let w1 = ((self.p2.1 - self.p0.1) * (px - self.p2.0) + (self.p0.0 - self.p2.0) * (py - self.p2.1)) * self.inv_denom;
        if w1 < -0.001 || w1 > 1.001 { return None; }
        let w2 = 1.0 - w0 - w1;
        if w2 < -0.001 || w2 > 1.001 { return None; }

        let u = w0 * self.uv0.0 + w1 * self.uv1.0 + w2 * self.uv2.0;
        let v = w0 * self.uv0.1 + w1 * self.uv1.1 + w2 * self.uv2.1;
        Some((u, v))
    }
}

/// Interpolación lineal exacta y robusta de la curva palpebral (inmune a rotación de cabeza y no-monotonía en X)
#[inline(always)]
pub fn sample_lid_y(pts: &[(f32, f32); 9], px: f32) -> f32 {
    let min_x = pts[0].0.min(pts[8].0);
    let max_x = pts[0].0.max(pts[8].0);
    if px <= min_x {
        return if pts[0].0 <= pts[8].0 { pts[0].1 } else { pts[8].1 };
    }
    if px >= max_x {
        return if pts[0].0 <= pts[8].0 { pts[8].1 } else { pts[0].1 };
    }
    for i in 0..8 {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[i + 1];
        let seg_min_x = x0.min(x1);
        let seg_max_x = x0.max(x1);
        if px >= seg_min_x && px <= seg_max_x {
            let dx = (x1 - x0).abs();
            if dx > 1e-3 {
                let t = ((px - seg_min_x) / (seg_max_x - seg_min_x)).clamp(0.0, 1.0);
                return if x0 <= x1 { y0 + t * (y1 - y0) } else { y1 + t * (y0 - y1) };
            } else {
                return (y0 + y1) * 0.5;
            }
        }
    }
    pts[4].1
}

/// Interpolación lineal exacta de las 4 curvas de los labios (11 vértices ordenados de comisura izquierda a derecha).
#[inline(always)]
pub fn sample_lip_y(pts: &[(f32, f32); 11], px: f32) -> f32 {
    if px <= pts[0].0 {
        return pts[0].1;
    }
    if px >= pts[10].0 {
        return pts[10].1;
    }
    for i in 0..10 {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[i + 1];
        if px >= x0 && px <= x1 {
            let dx = x1 - x0;
            if dx > 1e-4 {
                return y0 + ((px - x0) / dx) * (y1 - y0);
            } else {
                return (y0 + y1) * 0.5;
            }
        }
    }
    pts[5].1
}

/// Muestreo vertical de la línea inferior de la ceja para perfilado con corrector.
#[inline(always)]
#[allow(dead_code)]
pub fn sample_eyebrow_lower_y(pts: &[(f32, f32); 5], px: f32) -> f32 {
    let min_x = pts[0].0.min(pts[4].0);
    let max_x = pts[0].0.max(pts[4].0);
    let clamped_x = px.clamp(min_x, max_x);

    for i in 0..4 {
        let x0 = pts[i].0.min(pts[i + 1].0);
        let x1 = pts[i].0.max(pts[i + 1].0);
        if clamped_x >= x0 && clamped_x <= x1 {
            let dx = pts[i + 1].0 - pts[i].0;
            if dx.abs() < 1e-3 {
                return pts[i].1;
            }
            let t = (clamped_x - pts[i].0) / dx;
            return pts[i].1 + t * (pts[i + 1].1 - pts[i].1);
        }
    }
    pts[2].1
}
