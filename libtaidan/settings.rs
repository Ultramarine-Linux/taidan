#[derive(Clone, Debug, Default, serde::Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub langlocale: &'static str,

    pub skipconfig: bool,
    pub nointernet: bool,

    pub kb_layout: String,
    pub kb_variant: Option<String>,

    pub device_name: String,
    pub hostname: String,

    pub fullname: String,
    pub username: String,

    pub passwd: String,

    pub install_codecs_drivers: bool,

    pub ims: Vec<&'static str>,
    pub imf: crate::i18n::I18nImf,

    pub actions: [Vec<String>; crate::cfg::ACTION_TYPES],

    pub tweaks: Vec<bool>,
}

impl Settings {
    pub fn new() -> Self {
        let mut ret = Self::default();
        ret.tweaks = vec![false; crate::tweaks::TWEAKS.len()];
        ret
    }
}
