use std::ffi::c_void;
use std::mem;
use std::ptr;
use std::sync::OnceLock;

use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDIBSection, DIB_RGB_COLORS, DeleteObject, HBITMAP,
};
use windows_sys::Win32::Graphics::GdiPlus::{
    BitmapData, GdipBitmapLockBits, GdipBitmapUnlockBits, GdipCreateBitmapFromScan0,
    GdipCreateBitmapFromStream, GdipCreatePen1, GdipDeleteGraphics, GdipDeletePen,
    GdipDisposeImage, GdipDrawImageRectI, GdipDrawLines, GdipGetImageGraphicsContext,
    GdipSetInterpolationMode, GdipSetPenEndCap, GdipSetPenStartCap, GdipSetSmoothingMode,
    GdiplusStartup, GdiplusStartupInput, GpBitmap, GpGraphics, ImageLockModeRead,
    InterpolationModeHighQualityBicubic, LineCapRound, PointF, Rect, SmoothingModeAntiAlias,
    UnitPixel,
};
use windows_sys::Win32::UI::Shell::SHCreateMemStream;

// Constants
const PIXEL_FORMAT_32BPP_PARGB: i32 = 0x000E_200B;
const RELEASE_SLOT: usize = 2;
const CROSS_START: f32 = 0.2;
const CROSS_END: f32 = 0.8;
const CROSS_WIDTH: f32 = 0.17;
const OUTLINE_WIDTH: f32 = 0.12;
const CROSS_COLOR: u32 = 0xFFE0_2424;
const OUTLINE_COLOR: u32 = 0xE0FF_FFFF;

// Types
pub struct MenuBitmap(HBITMAP);

impl MenuBitmap {
    pub fn handle(&self) -> HBITMAP {
        self.0
    }
}

impl Drop for MenuBitmap {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.0);
        }
    }
}

struct Stream(*mut c_void);

impl Drop for Stream {
    fn drop(&mut self) {
        unsafe {
            let table = *self.0.cast::<*const usize>();
            let release: unsafe extern "system" fn(*mut c_void) -> u32 =
                mem::transmute(*table.add(RELEASE_SLOT));
            release(self.0);
        }
    }
}

struct Image(*mut GpBitmap);

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            GdipDisposeImage(self.0.cast());
        }
    }
}

// Conversion
pub fn menu_bitmap(png: &[u8], size: i32, checked: bool) -> Option<MenuBitmap> {
    if png.is_empty() || size <= 0 || !start_gdiplus() {
        return None;
    }
    let length = u32::try_from(png.len()).ok()?;
    let stream = unsafe { SHCreateMemStream(png.as_ptr(), length) };
    if stream.is_null() {
        return None;
    }
    let stream = Stream(stream);
    let mut source = ptr::null_mut();
    if unsafe { GdipCreateBitmapFromStream(stream.0, &raw mut source) } != 0 {
        return None;
    }
    let source = Image(source);
    let scaled = scale(&source, size, checked)?;
    let pixels = premultiplied_pixels(&scaled, size)?;
    drop(source);
    drop(stream);
    create_bitmap(&pixels, size)
}

fn scale(source: &Image, size: i32, checked: bool) -> Option<Image> {
    let mut target = ptr::null_mut();
    if unsafe {
        GdipCreateBitmapFromScan0(
            size,
            size,
            0,
            PIXEL_FORMAT_32BPP_PARGB,
            ptr::null(),
            &raw mut target,
        )
    } != 0
    {
        return None;
    }
    let target = Image(target);
    let mut graphics = ptr::null_mut();
    if unsafe { GdipGetImageGraphicsContext(target.0.cast(), &raw mut graphics) } != 0 {
        return None;
    }
    let drawn = unsafe {
        GdipSetInterpolationMode(graphics, InterpolationModeHighQualityBicubic);
        GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
        let status = GdipDrawImageRectI(graphics, source.0.cast(), 0, 0, size, size);
        if checked && status == 0 {
            draw_cross(graphics, size);
        }
        GdipDeleteGraphics(graphics);
        status
    };
    (drawn == 0).then_some(target)
}

#[allow(clippy::cast_precision_loss)]
fn draw_cross(graphics: *mut GpGraphics, size: i32) {
    let size = size as f32;
    let point = |x: f32, y: f32| PointF {
        X: size * x,
        Y: size * y,
    };
    let strokes = [
        [point(CROSS_START, CROSS_START), point(CROSS_END, CROSS_END)],
        [point(CROSS_END, CROSS_START), point(CROSS_START, CROSS_END)],
    ];
    for (color, width) in [
        (OUTLINE_COLOR, size * (CROSS_WIDTH + OUTLINE_WIDTH)),
        (CROSS_COLOR, size * CROSS_WIDTH),
    ] {
        unsafe {
            let mut pen = ptr::null_mut();
            if GdipCreatePen1(color, width, UnitPixel, &raw mut pen) != 0 {
                continue;
            }
            GdipSetPenStartCap(pen, LineCapRound);
            GdipSetPenEndCap(pen, LineCapRound);
            for stroke in &strokes {
                GdipDrawLines(graphics, pen, stroke.as_ptr(), 2);
            }
            GdipDeletePen(pen);
        }
    }
}

fn premultiplied_pixels(image: &Image, size: i32) -> Option<Vec<u8>> {
    let rect = Rect {
        X: 0,
        Y: 0,
        Width: size,
        Height: size,
    };
    let mut data = BitmapData::default();
    if unsafe {
        GdipBitmapLockBits(
            image.0,
            &raw const rect,
            u32::try_from(ImageLockModeRead).ok()?,
            PIXEL_FORMAT_32BPP_PARGB,
            &raw mut data,
        )
    } != 0
    {
        return None;
    }
    let row = usize::try_from(size).ok()? * 4;
    let stride = isize::try_from(data.Stride).ok()?;
    let mut pixels = Vec::with_capacity(row * row / 4);
    for line in 0..isize::try_from(size).ok()? {
        let start = unsafe { data.Scan0.cast::<u8>().offset(line * stride) };
        pixels.extend_from_slice(unsafe { std::slice::from_raw_parts(start, row) });
    }
    unsafe {
        GdipBitmapUnlockBits(image.0, &raw mut data);
    }
    Some(pixels)
}

fn create_bitmap(pixels: &[u8], size: i32) -> Option<MenuBitmap> {
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: u32::try_from(mem::size_of::<BITMAPINFOHEADER>()).ok()?,
            biWidth: size,
            biHeight: -size,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..unsafe { mem::zeroed() }
        },
        ..unsafe { mem::zeroed() }
    };
    let mut bits = ptr::null_mut();
    let bitmap = unsafe {
        CreateDIBSection(
            ptr::null_mut(),
            &raw const info,
            DIB_RGB_COLORS,
            &raw mut bits,
            ptr::null_mut(),
            0,
        )
    };
    if bitmap.is_null() || bits.is_null() {
        return None;
    }
    unsafe {
        ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len());
    }
    Some(MenuBitmap(bitmap))
}

fn start_gdiplus() -> bool {
    static STARTED: OnceLock<bool> = OnceLock::new();
    *STARTED.get_or_init(|| {
        let input = GdiplusStartupInput {
            GdiplusVersion: 1,
            ..GdiplusStartupInput::default()
        };
        let mut token = 0;
        unsafe { GdiplusStartup(&raw mut token, &raw const input, ptr::null_mut()) == 0 }
    })
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFF_u32;
        for &byte in data {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    fn chunk(png: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
        png.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        let start = png.len();
        png.extend_from_slice(&kind);
        png.extend_from_slice(data);
        let crc = crc32(&png[start..]);
        png.extend_from_slice(&crc.to_be_bytes());
    }

    fn red_png(size: u8) -> Vec<u8> {
        let row: Vec<u8> = std::iter::once(0)
            .chain([255, 0, 0, 255].repeat(usize::from(size)))
            .collect();
        let raw = row.repeat(usize::from(size));
        let (mut a, mut b) = (1_u32, 0_u32);
        for &byte in &raw {
            a = (a + u32::from(byte)) % 65_521;
            b = (b + a) % 65_521;
        }
        let length = u16::try_from(raw.len()).unwrap();
        let mut deflate = vec![0x78, 0x01, 0x01];
        deflate.extend_from_slice(&length.to_le_bytes());
        deflate.extend_from_slice(&(!length).to_le_bytes());
        deflate.extend_from_slice(&raw);
        deflate.extend_from_slice(&((b << 16) | a).to_be_bytes());
        let mut header = Vec::new();
        header.extend_from_slice(&u32::from(size).to_be_bytes());
        header.extend_from_slice(&u32::from(size).to_be_bytes());
        header.extend_from_slice(&[8, 6, 0, 0, 0]);
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        chunk(&mut png, *b"IHDR", &header);
        chunk(&mut png, *b"IDAT", &deflate);
        chunk(&mut png, *b"IEND", &[]);
        png
    }

    #[test]
    fn converts_a_png_into_a_menu_bitmap() {
        let bitmap = menu_bitmap(&red_png(8), 16, true);
        assert!(bitmap.is_some_and(|bitmap| !bitmap.handle().is_null()));
    }

    #[test]
    fn ignores_invalid_data() {
        assert!(menu_bitmap(&[], 16, false).is_none());
        assert!(menu_bitmap(b"not a png", 16, false).is_none());
    }
}
