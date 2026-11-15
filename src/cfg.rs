pub use libtaidan::cfg::{Config, SkipPage};
use libtaidan::passwd::user_already_exists;

#[must_use]
pub fn shows_page(page: SkipPage) -> bool {
    !crate::CFG.skip_pages.contains(&page)
}

#[derive(Clone, Debug)]
pub struct PageNav {
    skipped: Vec<i32>,
    summary: i32,
}

impl PageNav {
    #[must_use]
    pub fn new(cfg: &Config, pages: &crate::ui::Pages<'_>) -> Self {
        let skipped = cfg
            .skip_pages
            .iter()
            .filter_map(|page| {
                Some(match page {
                    SkipPage::Accessibility => pages.get_accessibility(),
                    SkipPage::Language => pages.get_language(),
                    SkipPage::Keyboard => pages.get_keyboard(),
                    SkipPage::Welcome => pages.get_welcome(),
                    SkipPage::Theme => pages.get_theme(),
                    SkipPage::Timezone => pages.get_timezone(),
                    SkipPage::Network => pages.get_network(),
                    SkipPage::Location => pages.get_location(),
                    SkipPage::Drivers => pages.get_drivers(),
                    SkipPage::Tweaks | SkipPage::InputMethod => return None,
                })
            })
            .collect();
        Self { skipped, summary: pages.get_summary() }
    }

    fn next(&self, page: i32) -> i32 {
        (page + 1..=self.summary).find(|i| !self.skipped.contains(i)).unwrap_or(page)
    }

    fn prev(&self, page: i32) -> i32 {
        (0..page).rev().find(|i| !self.skipped.contains(i)).unwrap_or(page)
    }

    #[must_use]
    pub fn first(&self) -> i32 {
        self.next(-1)
    }
}

#[tracing::instrument(skip_all)]
pub fn populate(cfg: &mut Config) {
    cfg.should_exit = !should_run(&cfg.taidan0);
    cfg.read_os_release();
    tracing::trace!("Populated config: {cfg:#?}");
}

fn finish_interrupted_install() {
    use crate::ui::block_on_thread;
    tracing::warn!("the system has a user but is still marked unconfigured, fixing");
    if block_on_thread(libtaidan::mark_configured()).join().is_err() {
        tracing::error!("marking the system configured panicked");
    }
    if !matches!(block_on_thread(libtaidan::tweaks::clear_on_record()).join(), Ok(Ok(()))) {
        tracing::error!("cannot remove the tweaks record");
    }
}

fn should_run(taidan0: &libtaidan::cfg::Taidan0Config) -> bool {
    if taidan0.continue_if_user_exists {
        tracing::info!("taidan0.continue_if_user_exists detected");
    } else if user_already_exists() {
        if libtaidan::marked_unconfigured() {
            finish_interrupted_install();
        }
        tracing::warn!("taidan0.continue_if_user_exists is false, exiting");
        return false;
    }
    true
}

static CFG_JSON: std::sync::LazyLock<serde_json::Value> =
    std::sync::LazyLock::new(|| serde_json::to_value(&*crate::CFG).expect("cannot serialize cfg"));

#[must_use]
pub fn get_val(id: &str) -> Option<&'static serde_json::Value> {
    id.split('.').try_fold(&*CFG_JSON, |val, component| match val {
        serde_json::Value::Array(arr) => arr.get(component.parse::<usize>().ok()?),
        serde_json::Value::Object(obj) => obj.get(component),
        _ => None,
    })
}

type HeroBuffer = slint::SharedPixelBuffer<slint::Rgba8Pixel>;

const THEME_CARD_WIDTH_REM: f32 = 12.;
const DESIGN_PX_PER_REM: f32 = 14.;
const MIN_THUMB_WIDTH: f32 = 64.;
const MAX_THUMB_WIDTH: f32 = 1920.;
const MAX_HERO_SIZE: u32 = 1920;
const FALLBACK_THUMB_WIDTH: u32 = 480;

fn hero_thumb_width(ui_scale: f32, scale_factor: f32) -> u32 {
    let width = THEME_CARD_WIDTH_REM * DESIGN_PX_PER_REM * ui_scale * scale_factor;
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped")]
    if width.is_finite() {
        width.ceil().clamp(MIN_THUMB_WIDTH, MAX_THUMB_WIDTH) as u32
    } else {
        FALLBACK_THUMB_WIDTH
    }
}

fn to_buffer(img: &image::DynamicImage) -> HeroBuffer {
    let rgba = img.to_rgba8();
    HeroBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height())
}

fn decode_hero(dark: bool) -> image::DynamicImage {
    let (key, wallpaper, bundled): (_, _, &[u8]) = if dark {
        (
            "welcome.hero_image_dark",
            crate::wallpaper::DARK,
            include_bytes!("../data/icons/background-dark.webp"),
        )
    } else {
        (
            "welcome.hero_image",
            crate::wallpaper::LIGHT,
            include_bytes!("../data/icons/background-light.webp"),
        )
    };
    let img = get_val(key)
        .and_then(serde_json::Value::as_str)
        .into_iter()
        .chain([wallpaper])
        .find_map(|path| {
            image::open(path).inspect_err(|e| tracing::warn!(?path, ?e, "cannot load image")).ok()
        })
        .unwrap_or_else(|| image::load_from_memory(bundled).expect("invalid bundled hero image"));
    if img.width() > MAX_HERO_SIZE || img.height() > MAX_HERO_SIZE {
        img.resize(MAX_HERO_SIZE, MAX_HERO_SIZE, image::imageops::FilterType::Triangle)
    } else {
        img
    }
}

fn hero_thumb(img: &image::DynamicImage, width: u32) -> HeroBuffer {
    // slint's renderers sample without mipmaps, so a full-size image aliases in a small card
    if img.width() > width {
        to_buffer(&img.resize(width, u32::MAX, image::imageops::FilterType::Lanczos3))
    } else {
        to_buffer(img)
    }
}

fn load_hero_images(ui: slint::Weak<crate::ui::AppWindow>) {
    use slint::ComponentHandle;
    crate::ui::spawn_then(
        ui,
        || {
            let (light, dark) = (decode_hero(false), decode_hero(true));
            let buffers = (to_buffer(&light), to_buffer(&dark));
            (light, dark, buffers)
        },
        |app, (light, dark, (light_buf, dark_buf))| {
            let cfg = app.global::<crate::ui::Cfg<'_>>();
            cfg.set_hero_light(slint::Image::from_rgba8(light_buf));
            cfg.set_hero_dark(slint::Image::from_rgba8(dark_buf));
            let width = hero_thumb_width(app.get_ui_scale(), app.window().scale_factor());
            tracing::debug!(width, "Making the theme thumbnails");
            crate::ui::spawn_then(
                app.as_weak(),
                move || (hero_thumb(&light, width), hero_thumb(&dark, width)),
                |app, (light, dark)| {
                    let cfg = app.global::<crate::ui::Cfg<'_>>();
                    cfg.set_hero_light_thumb(slint::Image::from_rgba8(light));
                    cfg.set_hero_dark_thumb(slint::Image::from_rgba8(dark));
                },
            );
        },
    );
}

fn config_color(path: &str) -> Option<slint::Color> {
    let serde_json::Value::String(s) = get_val(path)? else {
        tracing::error!(?path, "invalid color, expected a string");
        return None;
    };
    let color = parse_color(s);
    if color.is_none() {
        tracing::error!(?path, s, "invalid color, expected #aarrggbb or #rrggbb");
    }
    color
}

pub(crate) fn parse_color(code: &str) -> Option<slint::Color> {
    let digits = code.strip_prefix('#')?;
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    match digits.len() {
        8 => Some(slint::Color::from_argb_encoded(value)),
        6 => Some(slint::Color::from_argb_encoded(0xff00_0000 | value)),
        _ => None,
    }
}

pub fn initialize_ui(
    ui: slint::Weak<crate::ui::AppWindow>,
    cfg: &crate::ui::Cfg<'_>,
    nav: PageNav,
) {
    use slint::ToSharedString;
    cfg.set_version(env!("CARGO_PKG_VERSION").into());
    cfg.set_distro(crate::CFG.distro.to_shared_string());
    cfg.set_dev_session(crate::install::is_dev_session());
    cfg.set_skip_tweaks(!shows_page(SkipPage::Tweaks));
    cfg.on_next_page({
        let nav = nav.clone();
        move |page| nav.next(page)
    });
    cfg.on_prev_page(move |page| nav.prev(page));
    load_hero_images(ui);
    cfg.on_get_string(|path, default| {
        if let Some(serde_json::Value::String(s)) = get_val(&path) {
            s.to_shared_string()
        } else {
            default
        }
    });
    cfg.on_get_image(|path, default| {
        if let Some(serde_json::Value::String(path)) = get_val(&path) {
            slint::Image::load_from_path(std::path::Path::new(path))
                .inspect_err(|e| tracing::error!(?path, ?e, "cannot load image"))
                .unwrap_or(default)
        } else {
            default
        }
    });
    cfg.on_get_brush(|path, default| config_color(&path).map_or(default, slint::Brush::SolidColor));
    cfg.on_get_color(|path, default| config_color(&path).unwrap_or(default));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors() {
        assert_eq!(parse_color("#00078f"), Some(slint::Color::from_rgb_u8(0, 7, 0x8f)));
        assert_eq!(parse_color("#8000078f"), Some(slint::Color::from_argb_u8(0x80, 0, 7, 0x8f)));
        for bad in ["00078f", "#00078", "#zz078f", "#", "blue", "#+0078f", "#-0078f"] {
            assert_eq!(parse_color(bad), None, "{bad}");
        }
    }

    #[test]
    fn thumbs_follow_the_scale() {
        assert_eq!(hero_thumb_width(1., 1.), 168);
        assert_eq!(hero_thumb_width(1.36, 2.), 457);
        assert_eq!(hero_thumb_width(f32::NAN, 1.), 480);
        assert_eq!(hero_thumb_width(0., 1.), 64);
    }

    #[test]
    fn navigation_skips_pages() {
        let nav = PageNav { skipped: vec![0, 8, 9], summary: 11 };
        assert_eq!(nav.first(), 1);
        assert_eq!(nav.next(7), 10);
        assert_eq!(nav.prev(10), 7);
        assert_eq!(nav.prev(1), 1);
        assert_eq!(nav.next(11), 11);
    }
}
