//! the tweaks page: distro tweaks to turn on, see [`libtaidan::tweaks::Tweak`]
use std::rc::Rc;

use slint::{ComponentHandle, Model, VecModel};

use crate::ui::{AppWindow, TweakItem, Tweaks};

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, tweaks: &Tweaks<'_>) {
    let items = libtaidan::tweaks::TWEAKS
        .iter()
        .map(|t| TweakItem {
            id: t.id().to_string_lossy().as_ref().into(),
            name: t.ftl_name.as_str().into(),
            desc: t.ftl_desc.as_str().into(),
            on: false,
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
            crate::SETTINGS.write().tweaks = enabled.iter().map(|t| t.id.to_string()).collect();
            if let Some(ui) = ui.upgrade() {
                let names = enabled.into_iter().map(|t| t.name).collect::<Vec<_>>();
                ui.global::<Tweaks<'_>>()
                    .set_enabled_names(slint::ModelRc::new(VecModel::from(names)));
            }
        }
    });
}
