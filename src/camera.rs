use std::io;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use libcamera::camera::{Camera, CameraConfiguration, Orientation};
use libcamera::camera_manager::CameraManager;
use libcamera::color_space::{ColorSpace, Range, YcbcrEncoding};
use libcamera::framebuffer::AsFrameBuffer;
use libcamera::framebuffer_allocator::{FrameBuffer, FrameBufferAllocator};
use libcamera::framebuffer_map::MemoryMappedFrameBuffer;
use libcamera::geometry::Size;
use libcamera::pixel_format::PixelFormat;
use libcamera::properties::Location;
use libcamera::request::{RequestStatus, ReuseFlag};
use libcamera::stream::StreamRole;
use parking_lot::Mutex;

const WANTED_SIZE: Size = Size { width: 640, height: 480 };
const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(10);
const STOP_CHECK_INTERVAL: Duration = Duration::from_millis(100);

static ONE_MANAGER_PER_PROCESS: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Encoding {
    Mjpeg,
    Yuyv,
    Nv12,
    Packed { bytes_per_pixel: usize, rgb_offsets: [usize; 3] },
}

const fn fourcc(code: [u8; 4]) -> PixelFormat {
    PixelFormat::new(u32::from_le_bytes(code), 0)
}

const DECODABLE: [(PixelFormat, Encoding); 7] = [
    (fourcc(*b"BG24"), Encoding::Packed { bytes_per_pixel: 3, rgb_offsets: [0, 1, 2] }),
    (fourcc(*b"RG24"), Encoding::Packed { bytes_per_pixel: 3, rgb_offsets: [2, 1, 0] }),
    (fourcc(*b"XB24"), Encoding::Packed { bytes_per_pixel: 4, rgb_offsets: [0, 1, 2] }),
    (fourcc(*b"XR24"), Encoding::Packed { bytes_per_pixel: 4, rgb_offsets: [2, 1, 0] }),
    (fourcc(*b"NV12"), Encoding::Nv12),
    (fourcc(*b"YUYV"), Encoding::Yuyv),
    (fourcc(*b"MJPG"), Encoding::Mjpeg),
];

fn encoding_of(format: PixelFormat) -> Option<Encoding> {
    DECODABLE.iter().find(|(f, _)| *f == format).map(|&(_, enc)| enc)
}

#[derive(Clone, Copy, Debug)]
struct Layout {
    encoding: Encoding,
    width: u32,
    height: u32,
    stride: u32,
    orientation: Orientation,
    yuv: YuvToRgb,
}

#[derive(Clone, Copy, Debug)]
struct YuvToRgb {
    black: f32,
    luma_scale: f32,
    chroma_scale: f32,
    r_from_v: f32,
    g_from_u: f32,
    g_from_v: f32,
    b_from_u: f32,
}

impl YuvToRgb {
    fn new(encoding: YcbcrEncoding, range: Range) -> Self {
        let (kr, kb) = match encoding {
            YcbcrEncoding::Rec709 => (0.2126, 0.0722),
            YcbcrEncoding::Rec2020 => (0.2627, 0.0593),
            YcbcrEncoding::Rec601 | YcbcrEncoding::None => (0.299, 0.114),
        };
        let kg = 1.0 - kr - kb;
        let (black, luma_scale, chroma_scale) = match range {
            Range::Full => (0.0, 1.0, 1.0),
            Range::Limited => (16.0, 255.0 / 219.0, 255.0 / 224.0),
        };
        Self {
            black,
            luma_scale,
            chroma_scale,
            r_from_v: 2.0 * (1.0 - kr),
            g_from_u: -2.0 * kb * (1.0 - kb) / kg,
            g_from_v: -2.0 * kr * (1.0 - kr) / kg,
            b_from_u: 2.0 * (1.0 - kb),
        }
    }

    fn for_stream(color_space: Option<ColorSpace>) -> Self {
        color_space.map_or_else(
            || Self::new(YcbcrEncoding::Rec601, Range::Limited),
            |cs| Self::new(cs.ycbcr_encoding, cs.range),
        )
    }

    fn convert(&self, y: u8, u: u8, v: u8) -> [u8; 3] {
        let y = (f32::from(y) - self.black) * self.luma_scale;
        let u = (f32::from(u) - 128.0) * self.chroma_scale;
        let v = (f32::from(v) - 128.0) * self.chroma_scale;
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped")]
        let c = |x: f32| x.round().clamp(0.0, 255.0) as u8;
        [
            c(self.r_from_v.mul_add(v, y)),
            c(self.g_from_v.mul_add(v, self.g_from_u.mul_add(u, y))),
            c(self.b_from_u.mul_add(u, y)),
        ]
    }
}

struct Rows<'a> {
    data: &'a [u8],
    stride: usize,
    row_len: usize,
}

impl<'a> Rows<'a> {
    fn new(data: &'a [u8], stride: u32, row_len: usize) -> Option<Self> {
        Some(Self { data, stride: usize::try_from(stride).ok()?.max(row_len), row_len })
    }

    fn get(&self, row: usize) -> Option<&'a [u8]> {
        let start = row * self.stride;
        self.data.get(start..start + self.row_len)
    }
}

fn dimensions(width: u32, height: u32) -> Option<(usize, usize)> {
    Some((usize::try_from(width).ok()?, usize::try_from(height).ok()?))
}

fn yuyv_to_rgb(
    yuv: &YuvToRgb,
    width: u32,
    height: u32,
    stride: u32,
    data: &[u8],
) -> Option<image::RgbImage> {
    let (w, h) = dimensions(width, height)?;
    let rows = Rows::new(data, stride, w * 2)?;
    let mut out = Vec::with_capacity(w * h * 3);
    for row in 0..h {
        for &[y0, u, y1, v] in rows.get(row)?.as_chunks::<4>().0 {
            out.extend(yuv.convert(y0, u, v));
            out.extend(yuv.convert(y1, u, v));
        }
    }
    image::RgbImage::from_raw(width, height, out)
}

fn nv12_to_rgb(
    yuv: &YuvToRgb,
    width: u32,
    height: u32,
    stride: u32,
    luma: &[u8],
    chroma: &[u8],
) -> Option<image::RgbImage> {
    let (w, h) = dimensions(width, height)?;
    let luma = Rows::new(luma, stride, w)?;
    let chroma = Rows::new(chroma, stride, w.next_multiple_of(2))?;
    let mut out = Vec::with_capacity(w * h * 3);
    for row in 0..h {
        let (ys, uvs) = (luma.get(row)?, chroma.get(row / 2)?);
        for (x, &y) in ys.iter().enumerate() {
            let uv = x & !1;
            out.extend(yuv.convert(y, uvs[uv], uvs[uv + 1]));
        }
    }
    image::RgbImage::from_raw(width, height, out)
}

fn packed_to_rgb(
    width: u32,
    height: u32,
    stride: u32,
    bytes_per_pixel: usize,
    rgb_offsets: [usize; 3],
    data: &[u8],
) -> Option<image::RgbImage> {
    let (w, h) = dimensions(width, height)?;
    let rows = Rows::new(data, stride, w * bytes_per_pixel)?;
    let mut out = Vec::with_capacity(w * h * 3);
    for row in 0..h {
        for px in rows.get(row)?.chunks_exact(bytes_per_pixel) {
            out.extend(rgb_offsets.map(|offset| px[offset]));
        }
    }
    image::RgbImage::from_raw(width, height, out)
}

fn split_planes<'a>(planes: &[&'a [u8]], layout: &Layout) -> Option<(&'a [u8], &'a [u8])> {
    match *planes {
        [luma, chroma, ..] => Some((luma, chroma)),
        [both] => {
            let luma_len =
                usize::try_from(layout.stride).ok()? * usize::try_from(layout.height).ok()?;
            Some((both.get(..luma_len)?, both.get(luma_len..)?))
        }
        [] => None,
    }
}

fn decode(layout: &Layout, planes: &[&[u8]]) -> Option<image::RgbImage> {
    let Layout { width, height, stride, .. } = *layout;
    let first = *planes.first()?;
    match layout.encoding {
        Encoding::Mjpeg => image::load_from_memory_with_format(first, image::ImageFormat::Jpeg)
            .inspect_err(|e| tracing::debug!(?e, "bad MJPEG frame"))
            .ok()
            .map(image::DynamicImage::into_rgb8),
        Encoding::Yuyv => yuyv_to_rgb(&layout.yuv, width, height, stride, first),
        Encoding::Nv12 => {
            let (luma, chroma) = split_planes(planes, layout)?;
            nv12_to_rgb(&layout.yuv, width, height, stride, luma, chroma)
        }
        Encoding::Packed { bytes_per_pixel, rgb_offsets } => {
            packed_to_rgb(width, height, stride, bytes_per_pixel, rgb_offsets, first)
        }
    }
}

fn upright(img: image::RgbImage, orientation: Orientation) -> image::RgbImage {
    use image::imageops::{flip_horizontal, rotate90, rotate180, rotate270};
    let unmirrored = match orientation {
        Orientation::Rotate0Mirror
        | Orientation::Rotate90Mirror
        | Orientation::Rotate180Mirror
        | Orientation::Rotate270Mirror => flip_horizontal(&img),
        Orientation::Rotate0
        | Orientation::Rotate90
        | Orientation::Rotate180
        | Orientation::Rotate270 => img,
    };
    match orientation {
        Orientation::Rotate0 | Orientation::Rotate0Mirror => unmirrored,
        Orientation::Rotate90 | Orientation::Rotate90Mirror => rotate270(&unmirrored),
        Orientation::Rotate180 | Orientation::Rotate180Mirror => rotate180(&unmirrored),
        Orientation::Rotate270 | Orientation::Rotate270Mirror => rotate90(&unmirrored),
    }
}

fn configure(cam: &Camera<'_>) -> Option<(CameraConfiguration, Encoding)> {
    let mut cfgs = cam.generate_configuration(&[StreamRole::ViewFinder])?;
    let mut cfg = cfgs.get_mut(0)?;
    let offered = cfg.formats().pixel_formats().into_iter().collect::<Vec<_>>();
    let &(format, _) = DECODABLE.iter().find(|(f, _)| offered.contains(f))?;
    cfg.set_pixel_format(format);
    cfg.set_size(WANTED_SIZE);
    cfgs.set_orientation(Orientation::Rotate0);
    if cfgs.validate().is_invalid() {
        return None;
    }
    let encoding = encoding_of(cfgs.get(0)?.get_pixel_format())?;
    Some((cfgs, encoding))
}

fn preference(cam: &Camera<'_>) -> u8 {
    match cam.properties().get::<Location>() {
        Ok(Location::CameraFront) => 0,
        Ok(Location::CameraExternal) => 1,
        Err(_) => 2,
        Ok(Location::CameraBack) => 3,
    }
}

fn usable_cameras(mgr: &CameraManager) -> Vec<(Camera<'_>, CameraConfiguration, Encoding)> {
    let list = mgr.cameras();
    let mut cameras = (0..list.len())
        .filter_map(|i| list.get(i))
        .filter_map(|cam| {
            let configured = configure(&cam);
            tracing::debug!(id = cam.id(), usable = configured.is_some(), "Found a camera");
            configured.map(|(cfgs, enc)| (cam, cfgs, enc))
        })
        .collect::<Vec<_>>();
    cameras.sort_by_key(|(cam, ..)| preference(cam));
    cameras
}

fn manager() -> io::Result<CameraManager> {
    CameraManager::new()
        .inspect(|mgr| tracing::debug!(version = mgr.version(), "Started libcamera"))
}

pub fn available() -> bool {
    let _only = ONE_MANAGER_PER_PROCESS.lock();
    manager()
        .inspect_err(|e| tracing::warn!(?e, "cannot start libcamera"))
        .is_ok_and(|mgr| !usable_cameras(&mgr).is_empty())
}

pub fn stream<KeepGoing, OnFrame>(keep_going: KeepGoing, mut on_frame: OnFrame) -> io::Result<()>
where
    KeepGoing: Fn() -> bool,
    OnFrame: FnMut(image::RgbImage),
{
    let _only = ONE_MANAGER_PER_PROCESS.lock();
    let mgr = manager()?;
    let (cam, mut cfgs, encoding) = usable_cameras(&mgr)
        .into_iter()
        .next()
        .ok_or_else(|| io::Error::other("no camera found"))?;
    let mut cam = cam.acquire()?;
    cam.configure(&mut cfgs)?;
    let cfg = cfgs.get(0).ok_or_else(|| io::Error::other("camera has no stream"))?;
    let size = cfg.get_size();
    let layout = Layout {
        encoding,
        width: size.width,
        height: size.height,
        stride: cfg.get_stride(),
        orientation: cfgs.orientation(),
        yuv: YuvToRgb::for_stream(cfg.get_color_space()),
    };
    tracing::debug!(id = cam.id(), ?layout, "Camera started");
    let stream = cfg.stream().ok_or_else(|| io::Error::other("camera has no stream"))?;
    let mut alloc = FrameBufferAllocator::new(&cam);
    let requests = alloc
        .alloc(&stream)?
        .into_iter()
        .map(|buf| {
            let buf = MemoryMappedFrameBuffer::new(buf).map_err(io::Error::other)?;
            let mut req = cam
                .create_request(None)
                .ok_or_else(|| io::Error::other("cannot create request"))?;
            req.add_buffer(&stream, buf)?;
            Ok(req)
        })
        .collect::<io::Result<Vec<_>>>()?;
    let completed = cam.subscribe_request_completed();
    cam.start(None)?;
    for req in requests {
        cam.queue_request(req).map_err(|(_, e)| e)?;
    }
    let deadline = Instant::now() + FIRST_FRAME_TIMEOUT;
    let mut started = false;
    while keep_going() {
        let mut req = match completed.recv_timeout(STOP_CHECK_INTERVAL) {
            Ok(req) => req,
            Err(RecvTimeoutError::Timeout) if started || Instant::now() < deadline => continue,
            Err(RecvTimeoutError::Timeout) => {
                return Err(io::Error::other("camera sent no frames"));
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(io::Error::other("camera went away"));
            }
        };
        started = true;
        if req.status() == RequestStatus::Complete
            && let Some(fb) = req.buffer::<MemoryMappedFrameBuffer<FrameBuffer>>(&stream)
            && let Some(frame) = decode(&layout, &used_planes(fb))
        {
            on_frame(upright(frame, layout.orientation));
        }
        req.reuse(ReuseFlag::REUSE_BUFFERS);
        cam.queue_request(req).map_err(|(_, e)| e)?;
    }
    Ok(())
}

fn used_planes(fb: &MemoryMappedFrameBuffer<FrameBuffer>) -> Vec<&[u8]> {
    let used =
        fb.metadata().map(|m| m.planes().into_iter().map(|p| p.bytes_used).collect::<Vec<_>>());
    fb.data()
        .into_iter()
        .enumerate()
        .map(|(i, data)| {
            let len = used.as_ref().and_then(|u| u.get(i)).and_then(|&n| usize::try_from(n).ok());
            len.and_then(|n| data.get(..n)).unwrap_or(data)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_601: YuvToRgb = YuvToRgb {
        black: 0.0,
        luma_scale: 1.0,
        chroma_scale: 1.0,
        r_from_v: 1.402,
        g_from_u: -0.344_136,
        g_from_v: -0.714_136,
        b_from_u: 1.772,
    };

    #[test]
    fn full_601_matches_the_formula() {
        let built = YuvToRgb::new(YcbcrEncoding::Rec601, Range::Full);
        for (a, b) in [
            (built.r_from_v, FULL_601.r_from_v),
            (built.g_from_u, FULL_601.g_from_u),
            (built.g_from_v, FULL_601.g_from_v),
            (built.b_from_u, FULL_601.b_from_u),
        ] {
            assert!((a - b).abs() < 1e-4, "{a} != {b}");
        }
    }

    #[test]
    fn limited_range_stretches_to_full_black_and_white() {
        let yuv = YuvToRgb::new(YcbcrEncoding::Rec601, Range::Limited);
        assert_eq!(yuv.convert(16, 128, 128), [0, 0, 0]);
        assert_eq!(yuv.convert(235, 128, 128), [255, 255, 255]);
    }

    #[test]
    fn rec709_limited_red_is_red() {
        let yuv = YuvToRgb::new(YcbcrEncoding::Rec709, Range::Limited);
        let [r, g, b] = yuv.convert(63, 102, 240);
        assert!(r >= 254 && g <= 1 && b <= 1, "{r} {g} {b}");
    }

    #[test]
    fn unknown_color_space_is_limited_601() {
        let yuv = YuvToRgb::for_stream(None);
        assert_eq!(yuv.convert(16, 128, 128), [0, 0, 0]);
    }

    #[test]
    fn yuyv_grey_stays_grey() {
        let mid_grey_pair = [128, 128, 128, 128];
        let rgb = yuyv_to_rgb(&FULL_601, 2, 1, 0, &mid_grey_pair).unwrap();
        assert_eq!(rgb.as_raw(), &[128, 128, 128, 128, 128, 128]);
    }

    #[test]
    fn yuyv_skips_line_padding() {
        let two_lines_padded_with_white =
            [16, 128, 16, 128, 255, 255, 255, 255, 235, 128, 235, 128, 255, 255, 255, 255];
        let rgb = yuyv_to_rgb(&FULL_601, 2, 2, 8, &two_lines_padded_with_white).unwrap();
        assert_eq!(rgb.as_raw(), &[16, 16, 16, 16, 16, 16, 235, 235, 235, 235, 235, 235]);
        let cut_off_in_last_line = &two_lines_padded_with_white[..9];
        assert!(yuyv_to_rgb(&FULL_601, 2, 2, 8, cut_off_in_last_line).is_none());
    }

    #[test]
    fn nv12_shares_chroma_between_two_rows() {
        let luma = [16, 235, 0, 0, 235, 16, 0, 0];
        let chroma = [128, 128, 0, 0];
        let rgb = nv12_to_rgb(&FULL_601, 2, 2, 4, &luma, &chroma).unwrap();
        assert_eq!(rgb.as_raw(), &[16, 16, 16, 235, 235, 235, 235, 235, 235, 16, 16, 16]);
    }

    #[test]
    fn nv12_from_a_single_plane() {
        let layout = Layout {
            encoding: Encoding::Nv12,
            width: 2,
            height: 2,
            stride: 2,
            orientation: Orientation::Rotate0,
            yuv: FULL_601,
        };
        let frame = [50, 60, 70, 80, 128, 128];
        let rgb = decode(&layout, &[&frame]).unwrap();
        assert_eq!(rgb.as_raw(), &[50, 50, 50, 60, 60, 60, 70, 70, 70, 80, 80, 80]);
    }

    #[test]
    fn packed_formats_pick_the_right_channels() {
        let rgb888_in_memory_order = [3, 2, 1, 6, 5, 4];
        let rgb = packed_to_rgb(2, 1, 0, 3, [2, 1, 0], &rgb888_in_memory_order).unwrap();
        assert_eq!(rgb.as_raw(), &[1, 2, 3, 4, 5, 6]);
        let xbgr8888_in_memory_order = [1, 2, 3, 255, 4, 5, 6, 255];
        let rgb = packed_to_rgb(2, 1, 0, 4, [0, 1, 2], &xbgr8888_in_memory_order).unwrap();
        assert_eq!(rgb.as_raw(), &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn upright_undoes_clockwise_rotation() {
        let top_left_marked = image::RgbImage::from_fn(3, 2, |x, y| {
            image::Rgb(if (x, y) == (0, 0) { [255; 3] } else { [0; 3] })
        });
        let rotated_clockwise = image::imageops::rotate90(&top_left_marked);
        let fixed = upright(rotated_clockwise, Orientation::Rotate90);
        assert_eq!(fixed, top_left_marked);
        let mirrored =
            image::imageops::flip_horizontal(&image::imageops::rotate180(&top_left_marked));
        assert_eq!(upright(mirrored, Orientation::Rotate180Mirror), top_left_marked);
    }
}
