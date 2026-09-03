use slint::{Model, ToSharedString};

slint::include_modules!();

pub fn run() {
    tracing::debug!("Starting Taidan");
    let ui = AppWindow::new().expect("cannot create app");
    // TODO: refactor
    let theme = ui.global::<Theme<'_>>();
    theme.set_mode(ThemeMode::Light);
    crate::l10n::initialize_ui(ui.as_weak(), ui.global::<Lang<'_>>());
    let cfg: Cfg<'_> = ui.global();
    cfg.set_version(env!("CARGO_PKG_VERSION").into());
    cfg.on_get_string(|path, default| {
        if let Some(serde_json::Value::String(s)) = cfg_get_val(&path) {
            s.to_shared_string()
        } else {
            default
        }
    });
    cfg.on_get_image(|path, default| {
        if let Some(serde_json::Value::String(path)) = cfg_get_val(&path) {
            slint::Image::load_from_path(std::path::Path::new(&path))
                .inspect_err(|e| tracing::error!(?path, ?e, "cannot load image"))
                .unwrap_or(default)
        } else {
            default
        }
    });
    cfg.on_get_brush(|path, default| {
        if let Some(serde_json::Value::String(s)) = cfg_get_val(&path) {
            if let Some(code) = s.strip_prefix('#') {
                let Ok(v) = hex::decode(code).inspect_err(|e| tracing::error!(?path, s, ?e)) else {
                    return default;
                };
                return slint::Brush::SolidColor(match v[..] {
                    [a, r, g, b] => slint::Color::from_argb_u8(a, r, g, b),
                    [r, g, b] => slint::Color::from_rgb_u8(r, g, b),
                    _ => {
                        tracing::error!(?path, s, "invalid length, expected #aarrggbb or #rrggbb");
                        return default;
                    }
                });
            } else {
                tracing::error!(?path, s, "invalid brush/color, expected #aarrggbb or #rrggbb");
                return default;
            }
        }
        default
    });

    // autoscale(&ui);
    ui.run().expect("cannot run ui");
}

fn cfg_get_val(id: &str) -> Option<serde_json::Value> {
    let mut val = serde_json::to_value(&*crate::CFG).expect("cannot serialize cfg");
    let mut it = id.split('.');
    while let Some(component) = it.next() {
        if let Ok(i) = component.parse::<usize>()
            && let serde_json::Value::Array(arr) = val
        {
            val = arr.into_iter().nth(i)?;
        } else if let serde_json::Value::Object(mut obj) = val {
            val = obj.remove(component)?;
        } else {
            return None;
        }
    }
    Some(val)
}
