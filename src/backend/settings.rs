use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub skipconfig: bool,

    pub fullname: String,
    pub username: String,

    pub passwd: String,

    pub nightlight: bool,

    pub theme_is_dark: bool,
    // TODO: impl accent colors
    pub accent: usize,

    pub catalogue: HashMap<String, HashMap<usize, Vec<usize>>>,

    pub actions: [Vec<String>; crate::cfg::ACTION_TYPES],
}
