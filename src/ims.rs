use std::rc::Rc;

use slint::{ComponentHandle, Model, VecModel};

use crate::ui::{AppWindow, ImItem, Ims};

fn imf() -> libtaidan::i18n::I18nImf {
    crate::CFG.i18n.imf(&crate::CFG.edition)
}

fn items() -> Vec<ImItem> {
    if crate::CFG.skip_pages.contains(&crate::cfg::SkipPage::InputMethod) {
        return vec![];
    }
    let imf = imf();
    (libtaidan::i18n::IMS.entries())
        .flat_map(|(lang, ims)| {
            let lang = libtaidan::i18n::STR_TO_LANG.get(lang).map_or(*lang, |l| l.native());
            (ims.entries()).filter(move |(_, im)| im.available(imf)).map(move |(name, im)| ImItem {
                name: (*name).into(),
                native: im.native_name.into(),
                lang: lang.into(),
                on: false,
            })
        })
        .collect()
}

fn names_on(items: &impl Model<Data = ImItem>) -> Vec<String> {
    items.iter().filter(|im| im.on).map(|im| im.name.into()).collect()
}

fn show_enabled(ims: &Ims<'_>, model: &VecModel<ImItem>) {
    ims.set_enabled(names_on(model).join(", ").into());
}

#[must_use]
pub fn enabled(ims: &Ims<'_>) -> Vec<String> {
    names_on(&ims.get_items())
}

pub fn follow_locale_unless_picked(ui: &AppWindow) {
    if crate::SETTINGS.read().ims_picked {
        return;
    }
    let ims = ui.global::<Ims<'_>>();
    let items = ims.get_items();
    let Some(model) = items.as_any().downcast_ref::<VecModel<ImItem>>() else { return };
    let defaults = libtaidan::i18n::default_ims(&crate::SETTINGS.read().locale, imf());
    for i in 0..model.row_count() {
        let Some(mut item) = model.row_data(i) else { continue };
        let on = defaults.iter().any(|d| d.as_str() == item.name.as_str());
        if item.on != on {
            item.on = on;
            model.set_row_data(i, item);
        }
    }
    show_enabled(&ims, model);
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, ims: &Ims<'_>) {
    let model = Rc::new(VecModel::from(items()));
    ims.set_items(slint::ModelRc::from(Rc::clone(&model)));
    ims.on_set({
        let ui = ui.clone();
        move |i, on| {
            let Ok(i) = usize::try_from(i) else { return };
            let Some(mut item) = model.row_data(i) else { return };
            item.on = on;
            model.set_row_data(i, item);
            crate::SETTINGS.write().ims_picked = true;
            if let Some(ui) = ui.upgrade() {
                show_enabled(&ui.global(), &model);
            }
        }
    });
    if let Some(ui) = ui.upgrade() {
        follow_locale_unless_picked(&ui);
    }
}
