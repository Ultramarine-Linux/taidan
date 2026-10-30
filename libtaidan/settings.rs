#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub skipconfig: bool,
    pub nointernet: bool,

    pub kb_layout: String,
    pub kb_variant: Option<String>,

    pub device_name: String,
    pub hostname: String,

    pub fullname: String,
    pub username: String,

    pub passwd: String,
    /// image to use as the user's icon
    pub avatar: Option<std::path::PathBuf>,

    pub install_codecs_drivers: bool,

    pub ims: Vec<String>,
    pub imf: crate::i18n::I18nImf,

    pub actions: [Vec<String>; crate::cfg::ACTION_TYPES],

    pub tweaks: Vec<bool>,
}

impl Settings {
    pub fn new() -> Self {
        Settings { tweaks: vec![false; crate::tweaks::TWEAKS.len()], ..Default::default() }
    }
}
