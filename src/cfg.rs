pub use libtaidan::cfg::{Config, SkipPage};
use libtaidan::passwd::user_already_exists;

/// where `skip_pages` and Next and Back go, from the `Pages` global in `ui/page.slint`
#[derive(Clone, Debug)]
pub struct PageNav {
    /// indices of the pages `skip_pages` hides
    skipped: Vec<i32>,
    /// the last page Next and Back go through, the summary
    last: i32,
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
                    // not pages
                    SkipPage::Tweaks | SkipPage::InputMethod => return None,
                })
            })
            .collect();
        Self { skipped, last: pages.get_summary() }
    }

    /// the page Next goes to from `page`, past the skipped ones, `page` itself if there is none
    fn next(&self, page: i32) -> i32 {
        (page + 1..=self.last).find(|i| !self.skipped.contains(i)).unwrap_or(page)
    }

    /// the page Back goes to from `page`, past the skipped ones, `page` itself if there is none
    fn prev(&self, page: i32) -> i32 {
        (0..page).rev().find(|i| !self.skipped.contains(i)).unwrap_or(page)
    }

    /// the page shown first
    #[must_use]
    pub fn first(&self) -> i32 {
        self.next(-1)
    }
}

/// fills in what `/etc/taidan.toml` doesn't say, and whether the OOBE should run at all
#[tracing::instrument(skip_all)]
pub fn populate(cfg: &mut Config) {
    cfg.should_exit = !should_run(&cfg.taidan0);
    cfg.read_os_release();
    tracing::trace!("Populated config: {cfg:#?}");
}

/// whether the OOBE should run, false once the system already has a user
fn should_run(taidan0: &libtaidan::cfg::Taidan0Config) -> bool {
    if taidan0.continue_if_user_exists {
        tracing::info!("taidan0.continue_if_user_exists detected");
    } else if user_already_exists() {
        // an install that died between removing its user record and the markers would
        // otherwise start the OOBE on every boot. On a thread, since this may run on `RT`
        if libtaidan::marked_unconfigured() {
            tracing::warn!("the system has a user but is still marked unconfigured, fixing");
            let marked = std::thread::spawn(|| crate::RT.block_on(libtaidan::mark_configured()));
            if marked.join().is_err() {
                tracing::error!("marking the system configured panicked");
            }
            // nor should a later rerun undo the tweaks that install turned on
            let cleared =
                std::thread::spawn(|| crate::RT.block_on(libtaidan::tweaks::clear_on_record()));
            if !matches!(cleared.join(), Ok(Ok(()))) {
                tracing::error!("cannot remove the tweaks record");
            }
        }
        tracing::warn!("taidan0.continue_if_user_exists is false, exiting");
        return false;
    }
    true
}

static CFG_JSON: std::sync::LazyLock<serde_json::Value> =
    std::sync::LazyLock::new(|| serde_json::to_value(&*crate::CFG).expect("cannot serialize cfg"));

/// value at the dotted `id` in `/etc/taidan.toml`, e.g. `welcome.t0`
#[must_use]
pub fn get_val(id: &str) -> Option<&'static serde_json::Value> {
    id.split('.').try_fold(&*CFG_JSON, |val, component| match val {
        serde_json::Value::Array(arr) => arr.get(component.parse::<usize>().ok()?),
        serde_json::Value::Object(obj) => obj.get(component),
        _ => None,
    })
}

type HeroBuffer = slint::SharedPixelBuffer<slint::Rgba8Pixel>;

/// width of the theme page's preview cards in `rem`, see `ui/pages/0035_theme.slint`
const THEME_CARD_REM: f32 = 12.;
/// design pixels in a `rem`, `default-font-size` in `ui/app.slint`
const REM_UNITS: f32 = 14.;

/// how wide the theme page's thumbnails have to be to fill their cards, in physical pixels, at
/// the UI's `scale` (`Tokens.scale`) on a screen with `scale_factor`
fn hero_thumb_width(scale: f32, scale_factor: f32) -> u32 {
    let width = THEME_CARD_REM * REM_UNITS * scale * scale_factor;
    // a broken scale mustn't ask for a 0 or a huge thumbnail
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped")]
    if width.is_finite() { width.ceil().clamp(64., 1920.) as u32 } else { 480 }
}

fn to_buffer(img: &image::DynamicImage) -> HeroBuffer {
    let rgba = img.to_rgba8();
    HeroBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height())
}

/// welcome page image: `welcome.hero_image[_dark]`, else the system wallpaper, else the bundled one
fn decode_hero(dark: bool) -> image::DynamicImage {
    let (key, wallpaper, bundled): (_, _, &[u8]) = if dark {
        (
            "welcome.hero_image_dark",
            "/usr/share/backgrounds/default-dark.png",
            include_bytes!("../data/icons/background-dark.webp"),
        )
    } else {
        (
            "welcome.hero_image",
            "/usr/share/backgrounds/default.png",
            include_bytes!("../data/icons/background-light.webp"),
        )
    };
    let configured = match get_val(key) {
        Some(serde_json::Value::String(path)) => Some(path.as_str()),
        _ => None,
    };
    let img = configured
        .into_iter()
        .chain([wallpaper])
        .find_map(|path| {
            image::open(path).inspect_err(|e| tracing::warn!(?path, ?e, "cannot load image")).ok()
        })
        .unwrap_or_else(|| image::load_from_memory(bundled).expect("invalid bundled hero image"));
    // wallpapers are often 4K but the hero never needs more than this
    if img.width() > 1920 || img.height() > 1920 {
        img.resize(1920, 1920, image::imageops::FilterType::Triangle)
    } else {
        img
    }
}

/// a copy of the hero `width` pixels wide for the theme page. The renderers sample without
/// mipmaps, so drawing the full image into a small card skips most of its pixels and aliases.
/// Filter it down here instead.
fn hero_thumb(img: &image::DynamicImage, width: u32) -> HeroBuffer {
    if img.width() > width {
        to_buffer(&img.resize(width, u32::MAX, image::imageops::FilterType::Lanczos3))
    } else {
        to_buffer(img)
    }
}

/// decodes the welcome images on a separate thread, since decoding a 4K wallpaper on the UI thread
/// when the page first shows would stall its slide-in animation. The theme page's thumbnails come
/// after, once the event loop runs and the UI's scale is known
fn load_hero_images(ui: slint::Weak<crate::ui::AppWindow>) {
    std::thread::spawn(move || {
        let (light, dark) = (decode_hero(false), decode_hero(true));
        let (light_buf, dark_buf) = (to_buffer(&light), to_buffer(&dark));
        let set = ui.upgrade_in_event_loop(move |app| {
            use slint::ComponentHandle;
            let cfg = app.global::<crate::ui::Cfg<'_>>();
            cfg.set_hero_light(slint::Image::from_rgba8(light_buf));
            cfg.set_hero_dark(slint::Image::from_rgba8(dark_buf));
            let width = hero_thumb_width(app.get_ui_scale(), app.window().scale_factor());
            tracing::debug!(width, "Making the theme thumbnails");
            let ui = app.as_weak();
            std::thread::spawn(move || {
                let (light, dark) = (hero_thumb(&light, width), hero_thumb(&dark, width));
                let set = ui.upgrade_in_event_loop(move |app| {
                    let cfg = app.global::<crate::ui::Cfg<'_>>();
                    cfg.set_hero_light_thumb(slint::Image::from_rgba8(light));
                    cfg.set_hero_dark_thumb(slint::Image::from_rgba8(dark));
                });
                if let Err(e) = set {
                    tracing::error!(?e, "cannot set the theme thumbnails");
                }
            });
        });
        if let Err(e) = set {
            tracing::error!(?e, "cannot set hero images");
        }
    });
}

/// the `#rrggbb` or `#aarrggbb` color at `path` in `/etc/taidan.toml`, [`None`] if it isn't set or
/// isn't valid
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

fn parse_color(code: &str) -> Option<slint::Color> {
    let bytes = hex::decode(code.strip_prefix('#')?).ok()?;
    match bytes[..] {
        [alpha, red, green, blue] => Some(slint::Color::from_argb_u8(alpha, red, green, blue)),
        [red, green, blue] => Some(slint::Color::from_rgb_u8(red, green, blue)),
        _ => None,
    }
}

fn load_image(path: &str) -> Option<slint::Image> {
    slint::Image::load_from_path(std::path::Path::new(path))
        .inspect_err(|e| tracing::error!(?path, ?e, "cannot load image"))
        .ok()
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
    cfg.set_skip_tweaks(crate::CFG.skip_pages.contains(&SkipPage::Tweaks));
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
            load_image(path).unwrap_or(default)
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
        for bad in ["00078f", "#00078", "#zz078f", "#", "blue"] {
            assert_eq!(parse_color(bad), None, "{bad}");
        }
    }

    #[test]
    fn thumbs_follow_the_scale() {
        // 12rem of 14 design pixels each
        assert_eq!(hero_thumb_width(1., 1.), 168);
        assert_eq!(hero_thumb_width(1.36, 2.), 457);
        assert_eq!(hero_thumb_width(f32::NAN, 1.), 480);
        assert_eq!(hero_thumb_width(0., 1.), 64);
    }

    #[test]
    fn navigation_skips_pages() {
        // the accessibility, network and location pages
        let nav = PageNav { skipped: vec![0, 8, 9], last: 11 };
        assert_eq!(nav.first(), 1);
        assert_eq!(nav.next(7), 10);
        assert_eq!(nav.prev(10), 7);
        assert_eq!(nav.prev(1), 1);
        assert_eq!(nav.next(11), 11);
    }
}
