const SHRINK: usize = 4;

pub fn soften(rgba: &[u8], width: usize, height: usize, radius: f32) -> (Vec<u8>, u32, u32) {
    let (small_w, small_h) = ((width / SHRINK).max(1), (height / SHRINK).max(1));
    let mut small = vec![0f32; small_w * small_h * 3];
    for y in 0..small_h {
        for x in 0..small_w {
            let mut sum = [0f32; 3];
            let mut count = 0f32;
            for sy in y * SHRINK..((y + 1) * SHRINK).min(height) {
                for sx in x * SHRINK..((x + 1) * SHRINK).min(width) {
                    let at = (sy * width + sx) * 4;
                    for ch in 0..3 {
                        sum[ch] += f32::from(rgba[at + ch]);
                    }
                    count += 1.0;
                }
            }
            for ch in 0..3 {
                small[(y * small_w + x) * 3 + ch] = sum[ch] / count.max(1.0);
            }
        }
    }

    let sigma = (radius / SHRINK as f32).max(0.5);
    let reach = (sigma * 3.0).ceil() as usize;
    let kernel: Vec<f32> = (0..=reach)
        .map(|i| (-((i * i) as f32) / (2.0 * sigma * sigma)).exp())
        .collect();
    let pass = |source: &[f32], across: bool| {
        let mut out = vec![0f32; source.len()];
        for y in 0..small_h {
            for x in 0..small_w {
                let (at, limit) = if across { (x, small_w) } else { (y, small_h) };
                let mut sum = [0f32; 3];
                let mut weight = 0f32;
                for i in at.saturating_sub(reach)..=(at + reach).min(limit - 1) {
                    let k = kernel[i.abs_diff(at)];
                    let from = if across { y * small_w + i } else { i * small_w + x } * 3;
                    for ch in 0..3 {
                        sum[ch] += source[from + ch] * k;
                    }
                    weight += k;
                }
                for ch in 0..3 {
                    out[(y * small_w + x) * 3 + ch] = sum[ch] / weight;
                }
            }
        }
        out
    };
    let blurred = pass(&pass(&small, true), false);

    let mut out = Vec::with_capacity(small_w * small_h * 4);
    for pixel in blurred.chunks_exact(3) {
        out.extend(pixel.iter().map(|c| (c + 0.5) as u8));
        out.push(255);
    }
    (out, small_w as u32, small_h as u32)
}
