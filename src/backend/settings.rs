use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub skipconfig: bool,
    pub nointernet: bool,

    pub fullname: String,
    pub username: String,

    pub passwd: String,

    pub nightlight: bool,

    pub theme_is_dark: bool,
    pub accent: Option<super::theme::AccentColor>,

    pub catalogue: HashMap<String, HashMap<usize, Vec<usize>>>,

    pub actions: [Vec<String>; crate::cfg::ACTION_TYPES],
}
