use libtaidan::passwd::user_already_exists;

pub const ACTION_TYPES: usize = 5;

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(skip)]
    pub distro: String,
    #[serde(default)]
    pub edition: String,

    // TODO: skip_pages
    // #[serde(default)]
    // pub skip_pages: Vec<crate::Page>,
    #[serde(default = "_default_org")]
    pub org: String,

    #[serde(default)]
    pub i18n: libtaidan::i18n::I18nCfg,

    #[serde(default)]
    pub taidan0: Taidan0Config,

    #[serde(default = "_default_internet_retry_interval")]
    pub internet_retry_interval: u64,

    #[serde(default = "_default_internet_timeout")]
    pub internet_timeout: u64,

    /// keys only the UI reads (e.g. `accent_color`, `welcome.*`), looked up through `Cfg` in Slint
    #[serde(flatten)]
    pub ui: serde_json::Map<String, serde_json::Value>,
}

fn _default_org() -> String {
    "Fyra Labs".into()
}

const fn _default_internet_retry_interval() -> u64 {
    5
}

const fn _default_internet_timeout() -> u64 {
    20
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

fn load_image(path: &str) -> Option<slint::Image> {
    slint::Image::load_from_path(std::path::Path::new(path))
        .inspect_err(|e| tracing::error!(?path, ?e, "cannot load image"))
        .ok()
}

pub fn initialize_ui(ui: slint::Weak<crate::ui::AppWindow>, cfg: &crate::ui::Cfg<'_>) {
    use slint::ToSharedString;
    cfg.set_version(env!("CARGO_PKG_VERSION").into());
    cfg.set_distro(crate::CFG.distro.to_shared_string());
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
    cfg.on_get_brush(|path, default| {
        if let Some(serde_json::Value::String(s)) = get_val(&path) {
            if let Some(code) = s.strip_prefix('#') {
                let Ok(bytes) = hex::decode(code).inspect_err(|e| tracing::error!(?path, s, ?e))
                else {
                    return default;
                };
                return slint::Brush::SolidColor(match bytes[..] {
                    [a, r, g, b] => slint::Color::from_argb_u8(a, r, g, b),
                    [r, g, b] => slint::Color::from_rgb_u8(r, g, b),
                    _ => {
                        tracing::error!(?path, s, "invalid length, expected #aarrggbb or #rrggbb");
                        return default;
                    }
                });
            }
            tracing::error!(?path, s, "invalid brush/color, expected #aarrggbb or #rrggbb");
            return default;
        }
        default
    });
}
