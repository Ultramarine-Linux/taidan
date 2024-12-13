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

    pub catalogue: Vec<crate::cfg::Action>,
}
