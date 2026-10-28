//! Rust port of the CPU path tracer in BenchRenderer.cs.
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
    Arc,
};
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
struct V {
    x: f32,
    y: f32,
    z: f32,
}
impl V {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    fn dot(self, b: Self) -> f32 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }
    fn cross(self, b: Self) -> Self {
        Self::new(
            self.y * b.z - self.z * b.y,
            self.z * b.x - self.x * b.z,
            self.x * b.y - self.y * b.x,
        )
    }
    fn norm(self) -> Self {
        self / self.dot(self).sqrt()
    }
    fn reflect(self, n: Self) -> Self {
        self - n * (2.0 * self.dot(n))
    }
}
impl Add for V {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }
}
impl Sub for V {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }
}
impl Mul<f32> for V {
    type Output = Self;
    fn mul(self, n: f32) -> Self {
        Self::new(self.x * n, self.y * n, self.z * n)
    }
}
impl Div<f32> for V {
    type Output = Self;
    fn div(self, n: f32) -> Self {
        self * (1.0 / n)
    }
}
impl Neg for V {
    type Output = Self;
    fn neg(self) -> Self {
        self * (-1.0)
    }
}
const X: V = V::new(1.0, 0.0, 0.0);
const Y: V = V::new(0.0, 1.0, 0.0);
#[derive(Clone, Copy)]
enum Mat {
    Diffuse,
    Metal,
    Glass,
    Light,
}
#[derive(Clone, Copy)]
struct Sphere {
    c: V,
    r: f32,
    m: Mat,
    albedo: f32,
    rough: f32,
}
const LAMP: Sphere = Sphere {
    c: V::new(-4.0, 7.0, -3.0),
    r: 2.2,
    m: Mat::Light,
    albedo: 0.0,
    rough: 0.0,
};
const POWER: f32 = 9.0;
fn scene() -> Vec<Sphere> {
    let mut s = vec![
        Sphere {
            c: V::new(0.0, 1.1, 0.0),
            r: 1.1,
            m: Mat::Glass,
            albedo: 0.98,
            rough: 0.0,
        },
        Sphere {
            c: V::new(-2.5, 0.9, 0.9),
            r: 0.9,
            m: Mat::Metal,
            albedo: 0.92,
            rough: 0.03,
        },
        Sphere {
            c: V::new(2.4, 0.8, 0.7),
            r: 0.8,
            m: Mat::Diffuse,
            albedo: 0.85,
            rough: 0.0,
        },
        Sphere {
            c: V::new(1.0, 0.45, -1.9),
            r: 0.45,
            m: Mat::Metal,
            albedo: 0.85,
            rough: 0.25,
        },
        Sphere {
            c: V::new(-1.1, 0.35, -2.0),
            r: 0.35,
            m: Mat::Diffuse,
            albedo: 0.15,
            rough: 0.0,
        },
    ];
    for i in 0..14 {
        let a = i as f32 / 14.0 * std::f32::consts::TAU;
        let m = match i % 3 {
            0 => Mat::Metal,
            1 => Mat::Diffuse,
            _ => Mat::Glass,
        };
        s.push(Sphere {
            c: V::new(a.cos() * 4.2, 0.28, a.sin() * 4.2 + 1.5),
            r: 0.28,
            m,
            albedo: if i % 2 == 0 { 0.9 } else { 0.3 },
            rough: 0.1,
        });
    }
    s
}
fn rnd(s: &mut u32) -> f32 {
    *s ^= *s << 13;
    *s ^= *s >> 17;
    *s ^= *s << 5;
    (*s >> 8) as f32 * (1.0 / 16777216.0)
}
fn basis(n: V) -> (V, V) {
    let a = (if n.x.abs() > 0.9 { Y } else { X }).cross(n).norm();
    (a, n.cross(a))
}
fn random_sphere(s: &mut u32) -> V {
    loop {
        let v = V::new(rnd(s) * 2.0 - 1.0, rnd(s) * 2.0 - 1.0, rnd(s) * 2.0 - 1.0);
        if v.dot(v) < 1.0 {
            return v;
        }
    }
}
fn hemisphere(n: V, s: &mut u32) -> V {
    let r1 = rnd(s);
    let r2 = rnd(s);
    let r = r1.sqrt();
    let phi = std::f32::consts::TAU * r2;
    let (a, b) = basis(n);
    (a * (r * phi.cos()) + b * (r * phi.sin()) + n * (1.0 - r1).sqrt()).norm()
}
fn sphere_hit(sp: Sphere, o: V, d: V, max: f32) -> Option<f32> {
    let oc = o - sp.c;
    let b = oc.dot(d);
    let c = oc.dot(oc) - sp.r * sp.r;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let mut t = -b - sq;
    if t < 1e-4 {
        t = -b + sq
    }
    if t > 1e-4 && t < max {
        Some(t)
    } else {
        None
    }
}
fn intersect(scene: &[Sphere], o: V, d: V) -> Option<(f32, i32, bool)> {
    let mut t = f32::MAX;
    let mut idx = -2;
    let mut lamp = false;
    if d.y < -1e-6 {
        let a = -o.y / d.y;
        if a > 1e-4 && a < 60.0 {
            t = a;
            idx = -1;
        }
    }
    for (i, sp) in scene.iter().enumerate() {
        if let Some(x) = sphere_hit(*sp, o, d, t) {
            t = x;
            idx = i as i32;
        }
    }
    if let Some(x) = sphere_hit(LAMP, o, d, t) {
        t = x;
        lamp = true;
    }
    if idx != -2 || lamp {
        Some((t, idx, lamp))
    } else {
        None
    }
}
fn direct(scene: &[Sphere], p: V, n: V, s: &mut u32) -> f32 {
    let to = LAMP.c - p;
    let dist2 = to.dot(to);
    let cos_max = (1.0 - LAMP.r * LAMP.r / dist2).max(0.0).sqrt();
    let w = to / dist2.sqrt();
    let r1 = rnd(s);
    let r2 = rnd(s);
    let ct = 1.0 - r1 * (1.0 - cos_max);
    let st = (1.0 - ct * ct).sqrt();
    let phi = std::f32::consts::TAU * r2;
    let (a, b) = basis(w);
    let dir = (a * (phi.cos() * st) + b * (phi.sin() * st) + w * ct).norm();
    let cn = dir.dot(n);
    if cn <= 0.0 {
        return 0.0;
    }
    if !matches!(intersect(scene, p + n * 1e-3, dir), Some((_, _, true))) {
        return 0.0;
    }
    POWER * cn * 2.0 * (1.0 - cos_max)
}
fn trace(scene: &[Sphere], mut o: V, mut d: V, s: &mut u32) -> f32 {
    let (mut radiance, mut throughput, mut specular) = (0.0, 1.0, true);
    for depth in 0..6 {
        let Some((t, idx, lamp)) = intersect(scene, o, d) else {
            radiance += throughput * (0.06 + 0.22 * d.y.max(0.0));
            break;
        };
        if lamp {
            if specular {
                radiance += throughput * POWER;
            }
            break;
        }
        let p = o + d * t;
        let (n, m, albedo, rough) = if idx < 0 {
            let check = ((p.x * 0.9).floor() as i32 + (p.z * 0.9).floor() as i32) & 1 == 0;
            (Y, Mat::Diffuse, if check { 0.62 } else { 0.38 }, 0.0)
        } else {
            let sp = scene[idx as usize];
            ((p - sp.c) / sp.r, sp.m, sp.albedo, sp.rough)
        };
        match m {
            Mat::Diffuse => {
                radiance += throughput * albedo * direct(scene, p, n, s);
                throughput *= albedo;
                d = hemisphere(n, s);
                o = p + n * 1e-3;
                specular = false;
            }
            Mat::Metal => {
                let r = d.reflect(n) + random_sphere(s) * rough;
                if r.dot(n) <= 0.0 {
                    return radiance;
                }
                d = r.norm();
                o = p + n * 1e-3;
                throughput *= albedo;
                specular = true;
            }
            Mat::Glass => {
                let ior = 1.5;
                let outside = d.dot(n) < 0.0;
                let nn = if outside { n } else { -n };
                let eta = if outside { 1.0 / ior } else { ior };
                let cos = (-d.dot(nn)).min(1.0);
                let sin2 = eta * eta * (1.0 - cos * cos);
                let r0: f32 = (1.0 - ior) / (1.0 + ior);
                let r0 = r0 * r0;
                let fresnel = r0 + (1.0 - r0) * (1.0 - cos).powf(5.0);
                if sin2 > 1.0 || rnd(s) < fresnel {
                    d = d.reflect(nn);
                    o = p + nn * 1e-3;
                } else {
                    d = (d * eta + nn * (eta * cos - (1.0 - sin2).sqrt())).norm();
                    o = p - nn * 1e-3;
                }
                throughput *= albedo;
                specular = true;
            }
            Mat::Light => break,
        }
        if depth >= 3 {
            let q = (1.0 - throughput).max(0.05);
            if rnd(s) < q {
                break;
            }
            throughput /= 1.0 - q;
        }
    }
    radiance
}
struct Camera {
    origin: V,
    lower: V,
    horizontal: V,
    vertical: V,
}
impl Camera {
    fn new(width: usize, height: usize) -> Self {
        let origin = V::new(0.0, 2.1, -7.2);
        let target = V::new(0.0, 0.75, 0.0);
        let hh = (38.0f32 * std::f32::consts::PI / 180.0 / 2.0).tan();
        let hw = width as f32 / height as f32 * hh;
        let w = (origin - target).norm();
        let u = Y.cross(w).norm();
        let v = w.cross(u);
        Self {
            origin,
            lower: origin - u * hw + v * hh - w,
            horizontal: u * (2.0 * hw),
            vertical: v * (-2.0 * hh),
        }
    }
    fn ray(&self, u: f32, v: f32) -> V {
        (self.lower + self.horizontal * u + self.vertical * v - self.origin).norm()
    }
}
fn pixel(
    scene: &[Sphere],
    camera: &Camera,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    spp: usize,
) -> u8 {
    let mut seed = (y * width + x) as u32;
    seed = seed.wrapping_mul(9781).wrapping_add(0x9e3779b9);
    let mut sum = 0.0;
    for _ in 0..spp {
        let u = (x as f32 + rnd(&mut seed)) / width as f32;
        let v = (y as f32 + rnd(&mut seed)) / height as f32;
        sum += trace(scene, camera.origin, camera.ray(u, v), &mut seed).min(4.0);
    }
    let mut v = sum / spp as f32 * 1.1;
    v = v * (2.51 * v + 0.03) / (v * (2.43 * v + 0.59) + 0.14);
    (v.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8
}
#[derive(Debug)]
pub struct RenderResult {
    pub threads: usize,
    pub seconds: f64,
    pub samples: u64,
    pub score: f64,
}
/// Shared frame buffer; readers can snapshot it safely while workers render tiles.
pub struct RenderPreview {
    width: usize,
    height: usize,
    pixels: Vec<AtomicU32>,
}
impl RenderPreview {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: (0..width * height)
                .map(|_| AtomicU32::new(0xff000000))
                .collect(),
        }
    }
    pub fn rgba(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.width * self.height * 4);
        for pixel in &self.pixels {
            let value = pixel.load(Ordering::Relaxed);
            bytes.extend_from_slice(&[
                ((value >> 16) & 255) as u8,
                ((value >> 8) & 255) as u8,
                (value & 255) as u8,
                255,
            ]);
        }
        bytes
    }
}
/// One full pass of the original fixed scene. Progress counts completed pixels.
pub fn run(
    threads: usize,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicUsize>,
) -> Result<RenderResult, String> {
    run_with_preview(
        threads,
        cancel,
        progress,
        Arc::new(RenderPreview::new(960, 540)),
    )
}
pub fn run_with_preview(
    threads: usize,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicUsize>,
    preview: Arc<RenderPreview>,
) -> Result<RenderResult, String> {
    if preview.width != 960 || preview.height != 540 {
        return Err("Dimensões de prévia inválidas".into());
    }
    run_sized_with_preview(threads, cancel, progress, 960, 540, 1024, preview)
}
#[cfg(test)]
fn run_sized(
    threads: usize,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicUsize>,
    width: usize,
    height: usize,
    spp: usize,
) -> Result<RenderResult, String> {
    run_sized_with_preview(
        threads,
        cancel,
        progress,
        width,
        height,
        spp,
        Arc::new(RenderPreview::new(width, height)),
    )
}
fn run_sized_with_preview(
    threads: usize,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicUsize>,
    width: usize,
    height: usize,
    spp: usize,
    pixels: Arc<RenderPreview>,
) -> Result<RenderResult, String> {
    if threads == 0 || width == 0 || height == 0 || spp == 0 {
        return Err("Parâmetros inválidos".into());
    }
    let scene = Arc::new(scene());
    let camera = Arc::new(Camera::new(width, height));
    let mut buckets = Vec::new();
    for y in (0..height).step_by(32) {
        for x in (0..width).step_by(32) {
            buckets.push((x, y, (width - x).min(32), (height - y).min(32)));
        }
    }
    buckets.sort_by(|a, b| {
        let key = |(x, y, w, h): &(usize, usize, usize, usize)| {
            let dx = *x as f64 + *w as f64 / 2.0 - width as f64 / 2.0;
            let dy = (*y as f64 + *h as f64 / 2.0 - height as f64 / 2.0) * 1.6;
            dx * dx + dy * dy
        };
        key(a).total_cmp(&key(b))
    });
    let buckets = Arc::new(buckets);
    let next = Arc::new(AtomicUsize::new(0));
    let samples = Arc::new(AtomicUsize::new(0));
    let start = Instant::now();
    let mut workers = Vec::new();
    for _ in 0..threads {
        let (sc, cam, bs, next, samples, worker_cancel, progress, pixels) = (
            scene.clone(),
            camera.clone(),
            buckets.clone(),
            next.clone(),
            samples.clone(),
            cancel.clone(),
            progress.clone(),
            pixels.clone(),
        );
        let result = std::thread::Builder::new().spawn(move || loop {
            if worker_cancel.load(Ordering::Relaxed) {
                break;
            }
            let i = next.fetch_add(1, Ordering::Relaxed);
            if i >= bs.len() {
                break;
            }
            let (x, y, w, h) = bs[i];
            for py in y..y + h {
                if worker_cancel.load(Ordering::Relaxed) {
                    break;
                }
                for px in x..x + w {
                    let gray = pixel(&sc, &cam, px, py, width, height, spp) as u32;
                    pixels.pixels[py * width + px].store(
                        0xff000000 | gray << 16 | gray << 8 | gray,
                        Ordering::Relaxed,
                    );
                }
                samples.fetch_add(w * spp, Ordering::Relaxed);
                progress.fetch_add(w, Ordering::Relaxed);
            }
        });
        match result {
            Ok(h) => workers.push(h),
            Err(e) => {
                cancel.store(true, Ordering::Relaxed);
                for w in workers {
                    let _ = w.join();
                }
                return Err(format!("Falha ao iniciar thread: {e}"));
            }
        }
    }
    for w in workers {
        w.join()
            .map_err(|_| "Falha na thread de renderização".to_owned())?;
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("Teste cancelado; nenhum resultado foi registrado.".into());
    }
    let seconds = start.elapsed().as_secs_f64();
    let samples = samples.load(Ordering::Relaxed) as u64;
    Ok(RenderResult {
        threads,
        seconds,
        samples,
        score: samples as f64 / seconds / 10_000.0,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_pixel() {
        let scene = scene();
        let camera = Camera::new(32, 18);
        assert_eq!(
            pixel(&scene, &camera, 8, 7, 32, 18, 8),
            pixel(&scene, &camera, 8, 7, 32, 18, 8)
        );
    }
    #[test]
    fn small_pass_counts_samples() {
        let r = run_sized(
            2,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicUsize::new(0)),
            8,
            6,
            2,
        )
        .unwrap();
        assert_eq!(r.samples, 96);
        assert!(r.score > 0.0)
    }
    #[test]
    fn preview_shows_rendered_pixels() {
        let preview = Arc::new(RenderPreview::new(8, 6));
        run_sized_with_preview(
            2,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicUsize::new(0)),
            8,
            6,
            2,
            preview.clone(),
        )
        .unwrap();
        let frame = preview.rgba();
        assert_eq!(frame.len(), 8 * 6 * 4);
        assert!(frame.chunks_exact(4).any(|p| p[0] > 0 && p[3] == 255));
    }
    #[test]
    fn cancelled_has_no_score() {
        let r = run_sized(
            1,
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicUsize::new(0)),
            8,
            6,
            2,
        );
        assert!(r.is_err());
    }
}

// Values obtained from the original .NET 8 BenchRenderer.RenderBucket.
#[test]
fn original_scene_reference_pixels() {
    let sc = scene();
    let cam = Camera::new(960, 540);
    for (x, y, expected) in [
        (0, 0, 97),
        (100, 50, 89),
        (480, 270, 93),
        (900, 500, 218),
        (300, 350, 121),
        (200, 240, 180),
    ] {
        assert_eq!(
            pixel(&sc, &cam, x, y, 960, 540, 1024),
            expected,
            "pixel {x},{y}"
        );
    }
}
