#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KConfigEntry {
    pub file: &'static str,
    pub group: &'static str,
    pub key: &'static str,
    pub value: Option<String>,
    pub write_as_bool: bool,
}

impl KConfigEntry {
    #[must_use]
    pub fn bool(file: &'static str, group: &'static str, key: &'static str, on: bool) -> Self {
        Self { file, group, key, value: Some(on.to_string()), write_as_bool: true }
    }

    #[must_use]
    pub fn value(
        file: &'static str,
        group: &'static str,
        key: &'static str,
        value: Option<impl Into<String>>,
    ) -> Self {
        Self { file, group, key, value: value.map(Into::into), write_as_bool: false }
    }

    #[must_use]
    pub const fn delete(file: &'static str, group: &'static str, key: &'static str) -> Self {
        Self { file, group, key, value: None, write_as_bool: false }
    }

    #[must_use]
    pub fn args(&self, file: &str) -> Vec<String> {
        let mut args =
            ["--file", file, "--group", self.group, "--key", self.key].map(str::to_owned).to_vec();
        match &self.value {
            Some(v) => {
                if self.write_as_bool {
                    args.push("--type=bool".to_owned());
                }
                args.extend(["--".to_owned(), v.clone()]);
            }
            None => args.push("--delete".to_owned()),
        }
        args
    }

    #[must_use]
    pub fn command(&self, config_dir: Option<&std::path::Path>) -> Vec<String> {
        let file = config_dir.map_or_else(
            || self.file.to_owned(),
            |dir| dir.join(self.file).to_string_lossy().into_owned(),
        );
        std::iter::once("kwriteconfig6".to_owned()).chain(self.args(&file)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kwriteconfig_args() {
        let on = KConfigEntry::bool("kaccessrc", "Keyboard", "StickyKeys", true);
        assert_eq!(
            on.args("/home/u/.config/kaccessrc"),
            [
                "--file",
                "/home/u/.config/kaccessrc",
                "--group",
                "Keyboard",
                "--key",
                "StickyKeys",
                "--type=bool",
                "--",
                "true"
            ]
        );
        let off = KConfigEntry::delete("kdeglobals", "KDE", "AnimationDurationFactor");
        assert_eq!(off.args("kdeglobals").last().map(String::as_str), Some("--delete"));
        let dir = std::path::Path::new("/skel/.config");
        assert_eq!(
            on.command(Some(dir))[..3],
            ["kwriteconfig6", "--file", "/skel/.config/kaccessrc"]
        );
        assert_eq!(on.command(None)[2], "kaccessrc");
    }
}
