use std::collections::HashMap;

#[derive(Clone, Debug, Default, serde::Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub langlocale: &'static str,

    pub skipconfig: bool,
    pub nointernet: bool,

    pub kb_layout: String,
    pub kb_variant: Option<String>,

    pub fullname: String,
    pub username: String,

    pub passwd: String,

    pub install_codecs_drivers: bool,

    pub ims: Vec<&'static str>,

    pub nightlight: bool,

    pub theme_is_dark: bool,
    pub accent: Option<super::theme::AccentColor>,

    pub catalogue: HashMap<String, HashMap<usize, Vec<usize>>>,

    pub actions: [Vec<String>; crate::cfg::ACTION_TYPES],

    pub tweaks: Vec<bool>,
}
