//! # cdda_noise — 3D simplex noise matching CDDA master
//!
//! Bit-identical port of `simplexnoise.cpp`.

const F3: f32 = 1.0 / 3.0;
const G3: f32 = 1.0 / 6.0;

/// 512-entry permutation table (256 duplicated).
static PERM: [u8; 512] = {
    const B: [u8; 256] = [
        151,160,137,91,90,15,131,13,201,95,96,53,194,233,7,225,
        140,36,103,30,69,142,8,99,37,240,21,10,23,190,6,148,
        247,120,234,75,0,26,197,62,94,252,219,203,117,35,11,32,
        57,177,33,88,237,149,56,87,174,20,125,136,171,168,68,175,
        74,165,71,134,139,48,27,166,77,146,158,231,83,111,229,122,
        60,211,133,230,220,105,92,41,55,46,245,40,244,102,143,54,
        65,25,63,161,1,216,80,73,209,76,132,187,208,89,18,169,
        200,196,135,130,116,188,159,86,164,100,109,198,173,186,3,64,
        52,217,226,250,124,123,5,202,38,147,118,126,255,82,85,212,
        207,206,59,227,47,16,58,17,182,189,28,42,223,183,170,213,
        119,248,152,2,44,154,163,70,221,153,101,155,167,43,172,9,
        129,22,39,253,19,98,108,110,79,113,224,232,178,185,112,104,
        218,246,97,228,251,34,242,193,238,210,144,12,191,179,162,241,
        81,51,145,235,249,14,239,107,49,192,214,31,181,199,106,157,
        184,84,204,176,115,121,50,45,127,4,150,254,138,236,205,93,
        222,114,67,29,24,72,243,141,128,195,78,66,215,61,156,180,
    ];
    let mut p = [0u8; 512];
    let mut i = 0usize;
    while i < 512 {
        p[i] = B[i & 255];
        i = i.wrapping_add(1);
    }
    p
};

const GRAD3: [(f32, f32, f32); 12] = [
    (1.0, 1.0, 0.0), (-1.0, 1.0, 0.0), (1.0, -1.0, 0.0), (-1.0, -1.0, 0.0),
    (1.0, 0.0, 1.0), (-1.0, 0.0, 1.0), (1.0, 0.0, -1.0), (-1.0, 0.0, -1.0),
    (0.0, 1.0, 1.0), (0.0, -1.0, 1.0), (0.0, 1.0, -1.0), (0.0, -1.0, -1.0),
];

#[inline] fn fastfloor(x: f32) -> i32 { if x > 0.0 { x as i32 } else { (x as i32) - 1 } }
#[inline] fn dot3(g: (f32, f32, f32), x: f32, y: f32, z: f32) -> f32 { g.0 * x + g.1 * y + g.2 * z }

/// Safe perm lookup — masks index to 0..511.
#[inline]
fn perm(idx: i32) -> u8 {
    PERM[(idx & 511) as usize]
}

/// Safe gradient index — returns 0..11.
#[inline]
fn grad_idx(i: i32, j: i32, k: i32) -> usize {
    perm(i + perm(j + perm(k) as i32) as i32) as usize % 12
}

pub fn raw_noise_3d(x: f32, y: f32, z: f32) -> f32 {
    let s = (x + y + z) * F3;
    let i = fastfloor(x + s);
    let j = fastfloor(y + s);
    let k = fastfloor(z + s);
    let t = (i.wrapping_add(j).wrapping_add(k)) as f32 * G3;
    let x0 = x - (i as f32 - t);
    let y0 = y - (j as f32 - t);
    let z0 = z - (k as f32 - t);

    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0      { (1, 0, 0, 1, 1, 0) }
        else if x0 >= z0 { (1, 0, 0, 1, 0, 1) }
        else             { (0, 0, 1, 1, 0, 1) }
    } else {
        if y0 >= z0      { (0, 1, 0, 0, 1, 1) }
        else if x0 >= z0 { (0, 1, 0, 1, 1, 0) }
        else             { (0, 0, 1, 0, 1, 1) }
    };

    let x1 = x0 - i1 as f32 + G3;
    let y1 = y0 - j1 as f32 + G3;
    let z1 = z0 - k1 as f32 + G3;
    let x2 = x0 - i2 as f32 + 2.0 * G3;
    let y2 = y0 - j2 as f32 + 2.0 * G3;
    let z2 = z0 - k2 as f32 + 2.0 * G3;
    let x3 = x0 - 1.0 + 3.0 * G3;
    let y3 = y0 - 1.0 + 3.0 * G3;
    let z3 = z0 - 1.0 + 3.0 * G3;

    let gi0 = grad_idx(i, j, k);
    let gi1 = grad_idx(i + i1, j + j1, k + k1);
    let gi2 = grad_idx(i + i2, j + j2, k + k2);
    let gi3 = grad_idx(i + 1, j + 1, k + 1);

    let mut n0 = 0.0f32; let mut n1 = 0.0f32; let mut n2 = 0.0f32; let mut n3 = 0.0f32;

    let t0 = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
    if t0 > 0.0 { let t = t0 * t0; n0 = t * t * dot3(GRAD3[gi0], x0, y0, z0); }
    let t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
    if t1 > 0.0 { let t = t1 * t1; n1 = t * t * dot3(GRAD3[gi1], x1, y1, z1); }
    let t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
    if t2 > 0.0 { let t = t2 * t2; n2 = t * t * dot3(GRAD3[gi2], x2, y2, z2); }
    let t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
    if t3 > 0.0 { let t = t3 * t3; n3 = t * t * dot3(GRAD3[gi3], x3, y3, z3); }

    32.0 * (n0 + n1 + n2 + n3)
}

pub fn octave_noise_3d(octaves: u32, persistence: f32, scale: f32, x: f32, y: f32, z: f32) -> f32 {
    let mut total = 0.0;
    let mut freq = scale;
    let mut amp = 1.0;
    let mut max_amp = 0.0;
    for _ in 0..octaves {
        total += raw_noise_3d(x * freq, y * freq, z * freq) * amp;
        max_amp += amp;
        amp *= persistence;
        freq *= 2.0;
    }
    total / max_amp
}

pub fn scaled_octave_noise_3d(oct: u32, per: f32, sc: f32, lo: f32, hi: f32, x: f32, y: f32, z: f32) -> f32 {
    let raw = octave_noise_3d(oct, per, sc, x, y, z);
    raw * (hi - lo) / 2.0 + (hi + lo) / 2.0
}

/// Hash a u32 seed into a safe f32 range for use as the z-coordinate.
/// This prevents i32 overflow in fastfloor when the seed is large.
#[inline]
fn seed_z(seed: u32) -> f32 {
    // LCG hash: maps any u32 into [0, 10000) deterministically.
    ((seed.wrapping_mul(1103515245).wrapping_add(12345)) % 10000) as f32
}

pub fn forest_noise_at(x: i32, y: i32, seed: u32) -> f32 {
    let r = scaled_octave_noise_3d(4, 0.5, 0.03, 0.0, 1.0, x as f32, y as f32, seed_z(seed));
    let d = scaled_octave_noise_3d(6, 0.5, 0.07, 0.0, 1.0, x as f32, y as f32, seed_z(seed));
    (r * r - d * d * d * 0.5).max(0.0)
}

pub fn lake_noise_at(x: i32, y: i32, seed: u32) -> f32 {
    let r = scaled_octave_noise_3d(8, 0.5, 0.002, 0.0, 1.0, x as f32, y as f32, seed_z(seed));
    r * r * r * r
}

pub fn ocean_noise_at(x: i32, y: i32, seed: u32) -> f32 { lake_noise_at(x, y, seed) }

pub fn floodplain_noise_at(x: i32, y: i32, seed: u32) -> f32 {
    let r = scaled_octave_noise_3d(4, 0.5, 0.05, 0.0, 1.0, x as f32, y as f32, seed_z(seed));
    r * r
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_raw_deterministic() { assert_eq!(raw_noise_3d(10., 20., 42.), raw_noise_3d(10., 20., 42.)); }
    #[test] fn test_forest_range() { for x in 0..50 { for y in 0..50 { let v = forest_noise_at(x*3, y*3, 42); assert!(v >= 0. && v <= 1., "{x},{y}: {v}"); } } }
    #[test] fn test_lake_range() { for x in 0..50 { for y in 0..50 { let v = lake_noise_at(x*3, y*3, 42); assert!(v >= 0. && v <= 1., "{x},{y}: {v}"); } } }
    #[test] fn test_ocean_eq_lake() { for x in 0..20 { for y in 0..20 { assert_eq!(ocean_noise_at(x, y, 42), lake_noise_at(x, y, 42)); } } }
    #[test] fn test_floodplain_range() { for x in 0..50 { for y in 0..50 { let v = floodplain_noise_at(x*3, y*3, 42); assert!(v >= 0. && v <= 1., "{x},{y}: {v}"); } } }
    #[test] fn test_diff_seeds() { assert_ne!(forest_noise_at(50, 50, 42), forest_noise_at(50, 50, 99)); }
}
