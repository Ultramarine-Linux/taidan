use libtaidan::passwd::user_already_exists;

pub const ACTION_TYPES: usize = 5;

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(skip)]
    pub distro: String,
    #[serde(default)]
    pub edition: String,

    /// pages the distro doesn't want shown, see [`SkipPage`]
    #[serde(default, deserialize_with = "de_skip_pages")]
    pub skip_pages: Vec<SkipPage>,
    #[serde(default = "_default_org")]
    pub org: String,

    #[serde(default)]
    pub i18n: libtaidan::i18n::I18nCfg,

    #[serde(default)]
    pub taidan0: Taidan0Config,

    /// keys only the UI reads (e.g. `accent_color`, `welcome.*`), looked up through `Cfg` in Slint,
    /// this also takes keys older versions had (`internet_timeout`, …) so those configs still parse
    #[serde(flatten)]
    pub ui: serde_json::Map<String, serde_json::Value>,
}

fn _default_org() -> String {
    "Fyra Labs".into()
}

/// a page that `skip_pages` in `/etc/taidan.toml` can hide
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkipPage {
    Accessibility,
    Language,
    Keyboard,
    Welcome,
    Theme,
    Timezone,
    Network,
    Location,
    /// the drivers and codecs page, also turns installing them off by default
    Drivers,
    /// not a page anymore, but the tweaks button on the summary page
    Tweaks,
}

/// index of the last page that Next and Back go through (the summary), as in `ui/app.slint`
const LAST_PAGE: i32 = 11;

impl SkipPage {
    /// parses a page name, including the ones from before the Slint rewrite (`internet`, `codecs`)
    fn parse(name: &str) -> Result<Self, &'static str> {
        Ok(match name {
            "accessibility" => Self::Accessibility,
            "language" => Self::Language,
            "keyboard" => Self::Keyboard,
            "welcome" => Self::Welcome,
            "theme" => Self::Theme,
            "timezone" => Self::Timezone,
            "network" | "internet" => Self::Network,
            "location" => Self::Location,
            "drivers" | "codecs" => Self::Drivers,
            "tweaks" => Self::Tweaks,
            "devicename" | "analytics" | "crashreport" | "inputmethod" => {
                return Err("this page no longer exists");
            }
            // the install can't create the account without these
            "whoareyou" | "password" | "summary" | "installing" | "finish" | "done" | "error" => {
                return Err("this page can't be skipped");
            }
            _ => return Err("unknown page"),
        })
    }

    /// position in `ui/app.slint`, `None` if it's not a page
    const fn index(self) -> Option<i32> {
        Some(match self {
            Self::Accessibility => 0,
            Self::Language => 1,
            Self::Keyboard => 2,
            Self::Welcome => 3,
            Self::Theme => 4,
            Self::Timezone => 7,
            Self::Network => 8,
            Self::Location => 9,
            Self::Drivers => 10,
            Self::Tweaks => return None,
        })
    }
}

/// reads `skip_pages`, ignoring (with a warning) names it doesn't know instead of refusing the
/// whole config
fn de_skip_pages<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Vec<SkipPage>, D::Error> {
    let names = <Vec<String> as serde::Deserialize>::deserialize(de)?;
    Ok(names
        .iter()
        .filter_map(|name| {
            SkipPage::parse(&name.to_lowercase())
                .inspect_err(|reason| tracing::warn!(name, reason, "ignoring skip_pages entry"))
                .ok()
        })
        .collect())
}

fn is_skipped(cfg: &Config, page: i32) -> bool {
    cfg.skip_pages.iter().any(|p| p.index() == Some(page))
}

/// the page Next goes to from `page`, past the skipped ones, `page` itself if there is none
fn next_page(cfg: &Config, page: i32) -> i32 {
    (page + 1..=LAST_PAGE).find(|&i| !is_skipped(cfg, i)).unwrap_or(page)
}

/// the page Back goes to from `page`, past the skipped ones, `page` itself if there is none
fn prev_page(cfg: &Config, page: i32) -> i32 {
    (0..page).rev().find(|&i| !is_skipped(cfg, i)).unwrap_or(page)
}

/// the page shown first
#[must_use]
pub fn first_page() -> i32 {
    next_page(&crate::CFG, -1)
}

impl Config {
    /// Create a new [`Config`] by reading from `/etc/taidan.toml`, else [`Self::default()`].
    ///
    /// Remember to [`Self::populate`] afterwards.
    ///
    /// # Errors
    /// Failure to parse `/etc/taidan.toml`.
    pub fn new() -> Result<Self, basic_toml::Error> {
        let Ok(s) = std::fs::read_to_string("/etc/taidan.toml")
            .inspect_err(|err| tracing::warn!(?err, "cannot read /etc/taidan.toml"))
        else {
            return Ok(Self::default());
        };
        basic_toml::from_str(&s)
    }

    /// Populate the [`Config`] struct.
    ///
    /// # Panics
    /// If there are any missing information, the function will immediately panic.
    #[tracing::instrument]
    pub fn populate(&mut self) {
        self.taidan0.check();
        // distro
        let file = std::fs::read_to_string("/etc/os-release").expect("Cannot read /etc/os-release");
        let name = file
            .split('\n')
            .find_map(|line| line.strip_prefix("NAME="))
            .expect("Cannot find NAME=… in /etc/os-release");
        name.strip_prefix('"')
            .and_then(|name| name.strip_suffix('"'))
            .unwrap_or(name)
            .clone_into(&mut self.distro);

        if self.edition.is_empty() {
            let edition = file
                .split('\n')
                .find_map(|line| line.strip_prefix("VARIANT_ID="))
                .expect("Cannot find VARIANT_ID=… in /etc/os-release");
            edition
                .strip_prefix('"')
                .and_then(|name| name.strip_suffix('"'))
                .unwrap_or(edition)
                .clone_into(&mut self.edition);
        }

        tracing::trace!("Populated config: {self:#?}");
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[expect(clippy::struct_excessive_bools, reason = "independent config flags")]
pub struct Taidan0Config {
    #[serde(default)]
    pub continue_if_user_exists: bool,
    #[serde(default)]
    pub skip_dnf: bool,
    #[serde(default)]
    pub persist_no_internet_btn: bool,
    #[serde(default)]
    pub systemd_homed: bool,
}

impl Taidan0Config {
    pub fn check(&self) {
        if self.continue_if_user_exists {
            tracing::info!("taidan0.continue_if_user_exists detected");
        } else if user_already_exists() {
            tracing::warn!("taidan0.continue_if_user_exists is false, exiting");
            std::process::exit(0);
        }
    }
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

/// welcome page image: `welcome.hero_image[_dark]`, else the system wallpaper, else the bundled one
fn decode_hero(dark: bool) -> HeroBuffer {
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
    let img = if img.width() > 1920 || img.height() > 1920 {
        img.resize(1920, 1920, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let rgba = img.into_rgba8();
    HeroBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height())
}

/// decodes the welcome images on a separate thread, since decoding a 4K wallpaper on the UI thread
/// when the page first shows would stall its slide-in animation
fn load_hero_images(ui: slint::Weak<crate::ui::AppWindow>) {
    std::thread::spawn(move || {
        let (light, dark) = (decode_hero(false), decode_hero(true));
        let set = ui.upgrade_in_event_loop(move |ui| {
            use slint::ComponentHandle;
            let cfg = ui.global::<crate::ui::Cfg<'_>>();
            cfg.set_hero_light(slint::Image::from_rgba8(light));
            cfg.set_hero_dark(slint::Image::from_rgba8(dark));
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

pub fn initialize_ui(ui: slint::Weak<crate::ui::AppWindow>, cfg: &crate::ui::Cfg<'_>) {
    use slint::ToSharedString;
    cfg.set_version(env!("CARGO_PKG_VERSION").into());
    cfg.set_distro(crate::CFG.distro.to_shared_string());
    cfg.set_dev_session(crate::install::is_dev_session());
    cfg.set_skip_tweaks(crate::CFG.skip_pages.contains(&SkipPage::Tweaks));
    cfg.on_next_page(|page| next_page(&crate::CFG, page));
    cfg.on_prev_page(|page| prev_page(&crate::CFG, page));
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
    fn skip_pages_accepts_old_names() {
        let cfg: Config =
            basic_toml::from_str(r#"skip_pages = ["codecs", "internet", "Theme", "nope"]"#)
                .unwrap();
        assert_eq!(cfg.skip_pages, [SkipPage::Drivers, SkipPage::Network, SkipPage::Theme]);
        // not swallowed by the flattened `ui` map
        assert!(!cfg.ui.contains_key("skip_pages"));
    }

    #[test]
    fn old_internet_keys_still_parse() {
        let cfg: Config =
            basic_toml::from_str("internet_timeout = 20\ninternet_retry_interval = 5").unwrap();
        assert_eq!(cfg.skip_pages, []);
    }

    #[test]
    fn navigation_skips_pages() {
        let cfg = Config {
            skip_pages: vec![SkipPage::Accessibility, SkipPage::Network, SkipPage::Location],
            ..Config::default()
        };
        assert_eq!(next_page(&cfg, -1), 1);
        assert_eq!(next_page(&cfg, 7), 10);
        assert_eq!(prev_page(&cfg, 10), 7);
        assert_eq!(prev_page(&cfg, 1), 1);
        assert_eq!(next_page(&cfg, LAST_PAGE), LAST_PAGE);
    }
}
