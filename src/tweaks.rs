use std::rc::Rc;

use slint::{ComponentHandle, Model, VecModel};

use crate::ui::{AppWindow, TweakItem, Tweaks};

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, tweaks: &Tweaks<'_>) {
    let available = crate::RT.block_on(libtaidan::tweaks::available(crate::install::can_use_dnf()));
    let items = available
        .into_iter()
        .map(|t| TweakItem {
            id: t.id().to_string_lossy().as_ref().into(),
            name: t.ftl_name.as_str().into(),
            desc: t.ftl_desc.as_str().into(),
            on: false,
            needs_internet: t.needs_internet,
        })
        .collect::<Vec<_>>();
    let model = Rc::new(VecModel::from(items));
    tweaks.set_items(slint::ModelRc::from(Rc::clone(&model)));
    tweaks.on_set({
        let ui = ui.clone();
        move |i, on| {
            let Ok(i) = usize::try_from(i) else { return };
            let Some(mut item) = model.row_data(i) else { return };
            item.on = on;
            model.set_row_data(i, item);
            let enabled = model.iter().filter(|t| t.on).collect::<Vec<_>>();
            if let Some(ui) = ui.upgrade() {
                let names = |offline: bool| {
                    let names = enabled.iter().filter(|t| !(offline && t.needs_internet));
                    slint::ModelRc::new(VecModel::from(
                        names.map(|t| t.name.clone()).collect::<Vec<_>>(),
                    ))
                };
                let tweaks = ui.global::<Tweaks<'_>>();
                tweaks.set_enabled_names_online(names(false));
                tweaks.set_enabled_names_offline(names(true));
            }
        }
    });
}

#[must_use]
pub fn enabled(tweaks: &Tweaks<'_>) -> libtaidan::settings::TweakIds {
    tweaks.get_items().iter().filter(|t| t.on).map(|t| t.id.to_string()).collect()
}
