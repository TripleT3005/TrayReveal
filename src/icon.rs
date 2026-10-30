// Constants
const VIEWBOX: f32 = 64.0;
const CIRCLE_RADIUS: f32 = 31.0;
const BACKGROUND: [f32; 3] = [47.0, 107.0, 78.0];
const FOREGROUND: [f32; 3] = [255.0, 255.0, 255.0];
const CHEVRON_WIDTH: f32 = 8.0;
const BAR_WIDTH: f32 = 6.0;
const CHEVRON: [[(f32, f32); 2]; 2] = [[(19.0, 35.0), (32.0, 22.0)], [(32.0, 22.0), (45.0, 35.0)]];
const BAR: [(f32, f32); 2] = [(20.0, 45.0), (44.0, 45.0)];
const SAMPLES: u16 = 4;

// Icon
pub fn icon_pixels(size: u16) -> Vec<u8> {
    let scale = VIEWBOX / f32::from(size);
    let samples = f32::from(SAMPLES * SAMPLES);
    let mut pixels = Vec::with_capacity(usize::from(size) * usize::from(size) * 4);
    for y in 0..size {
        for x in 0..size {
            let mut color = [0.0_f32; 3];
            let mut coverage = 0.0_f32;
            for sample_y in 0..SAMPLES {
                for sample_x in 0..SAMPLES {
                    let point = (
                        (f32::from(x) + (f32::from(sample_x) + 0.5) / f32::from(SAMPLES)) * scale,
                        (f32::from(y) + (f32::from(sample_y) + 0.5) / f32::from(SAMPLES)) * scale,
                    );
                    if let Some(sample) = color_at(point) {
                        for (channel, value) in color.iter_mut().zip(sample) {
                            *channel += value;
                        }
                        coverage += 1.0;
                    }
                }
            }
            if coverage == 0.0 {
                pixels.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                pixels.extend(color.map(|channel| to_byte(channel / coverage)));
                pixels.push(to_byte(coverage / samples * 255.0));
            }
        }
    }
    pixels
}

fn color_at((x, y): (f32, f32)) -> Option<[f32; 3]> {
    let center = VIEWBOX / 2.0;
    if (x - center).powi(2) + (y - center).powi(2) > CIRCLE_RADIUS.powi(2) {
        return None;
    }
    let on_chevron = CHEVRON
        .iter()
        .any(|&line| is_on_line((x, y), line, CHEVRON_WIDTH));
    Some(if on_chevron || is_on_line((x, y), BAR, BAR_WIDTH) {
        FOREGROUND
    } else {
        BACKGROUND
    })
}

fn is_on_line(
    (x, y): (f32, f32),
    [(start_x, start_y), (end_x, end_y)]: [(f32, f32); 2],
    width: f32,
) -> bool {
    let (delta_x, delta_y) = (end_x - start_x, end_y - start_y);
    let length = delta_x.mul_add(delta_x, delta_y * delta_y);
    let position =
        ((x - start_x).mul_add(delta_x, (y - start_y) * delta_y) / length).clamp(0.0, 1.0);
    let (nearest_x, nearest_y) = (
        position.mul_add(delta_x, start_x),
        position.mul_add(delta_y, start_y),
    );
    (x - nearest_x).powi(2) + (y - nearest_y).powi(2) <= (width / 2.0).powi(2)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_byte(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(pixels: &[u8], size: usize, x: usize, y: usize) -> &[u8] {
        let index = (y * size + x) * 4;
        &pixels[index..index + 4]
    }

    #[test]
    fn draws_a_chevron_above_a_bar() {
        let pixels = icon_pixels(64);
        assert_eq!(pixels.len(), 64 * 64 * 4);
        assert_eq!(pixel(&pixels, 64, 0, 0), [0, 0, 0, 0]);
        assert_eq!(pixel(&pixels, 64, 32, 8), [47, 107, 78, 255]);
        assert_eq!(pixel(&pixels, 64, 32, 22), [255, 255, 255, 255]);
        assert_eq!(pixel(&pixels, 64, 32, 44), [255, 255, 255, 255]);
        assert_eq!(pixel(&pixels, 64, 32, 38), [47, 107, 78, 255]);
    }

    #[test]
    fn smooths_the_edge() {
        let pixels = icon_pixels(32);
        let edge = pixel(&pixels, 32, 16, 0)[3];
        assert!(edge > 0 && edge < 255);
    }
}
