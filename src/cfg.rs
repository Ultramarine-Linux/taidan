#[derive(Default, Clone, Debug)]
pub struct Config {
    pub distro: String,
    pub icon_name: String,
}

impl Config {
    /// Populate the [`Config`] struct.
    ///
    /// # Panics
    /// If there are any missing information, the function will immediately panic.
    pub fn populate(&mut self) {
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

        // icon_name
        option_env!("TAIDAN_ICON_NAME")
            .unwrap_or("fedora-logo-icon")
            .clone_into(&mut self.icon_name);

        tracing::debug!("Populated config: {self:#?}");
    }
}
