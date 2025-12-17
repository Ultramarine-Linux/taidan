use crate::prelude::*;
generate_page!(Language {
    btnfactory: BtnFactory,
    search: libhelium::TextField,
}:
    init[search btnfactory { model.btnfactory.0.widget().clone() }](root, sender, model, widgets) {
        let btnfactory2 = btnfactory.clone();
        search.internal_entry().connect_changed(move |en| {
            *SEARCH_STATE.write() = en.text();
            btnfactory2.invalidate_filter();
            tracing::trace!(?en, "Search Changed!");
        });
        let btnfactory3 = model.btnfactory.0.clone();
        btnfactory.set_filter_func(move |row| {
            let s = SEARCH_STATE.read().as_str().to_ascii_lowercase();
            #[allow(clippy::cast_sign_loss)]
            let lang = btnfactory3.get(row.index() as usize).unwrap();
            lang.locale.to_ascii_lowercase().starts_with(&s)
                || lang.native_name.to_ascii_lowercase().contains(&s)
                || lang.name.to_ascii_lowercase().starts_with(&s)
        });
        btnfactory.select_row(btnfactory.iter_children().next().as_ref());
        
        // Add keyboard event handler for Enter key
        let sender_clone = sender.clone();
        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Return {
                // Navigate to next page when Enter is pressed
                sender_clone.input(Self::Input::Nav(NavAction::Next));
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });
        btnfactory.add_controller(key_controller);
    }

    update(self, message, sender) {
        Selected => {
            if let Some(row) = self.btnfactory.selected_row() {
                #[allow(clippy::cast_sign_loss)]
                let lang = self.btnfactory.0.get(row.index() as usize).unwrap();
                if lang.locale == "en-owo" {
                    let loader = i18n_embed::fluent::fluent_language_loader!();
                    loader
                        .load_languages(&crate::Localizations, &["en-Xowo".parse().unwrap()])
                        .expect("fail to load languages");
                    *crate::LL.write() = loader;
                    *crate::backend::l10n::PO_LOADER.write() = crate::backend::l10n::new_loader(vec!["en-Xowo".parse().unwrap()]);
                    SETTINGS.write().langlocale = "en_US";
                } else {
                    set_lang(lang);
                }
            }
        }
    } => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_margin_bottom: 16,
        set_valign: gtk::Align::Fill,
        set_halign: gtk::Align::Fill,

        gtk::Image {
            set_icon_name: Some("globe-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            #[watch]
            set_label: &t!("page-language"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },
    },

    #[local_ref]
    search -> libhelium::TextField {
        set_is_search: true,
        set_is_outline: true,
        set_margin_top: 6,
        set_margin_bottom: 6,
        set_prefix_icon: Some("system-search-symbolic"),
        #[watch]
        set_placeholder_text: Some(&t!("page-language-search-lang")),
    },
    gtk::ScrolledWindow {
        #[local_ref] btnfactory ->
        gtk::ListBox {
            add_css_class: "content-list",
            set_selection_mode: gtk::SelectionMode::Single,
            set_vexpand: true,
            set_hexpand: true,
            set_valign: gtk::Align::Center,
            set_halign: gtk::Align::Center,
            connect_selected_rows_changed => LanguagePageMsg::Selected,
        }
    },

    libhelium::Button {
        set_valign: gtk::Align::End,
        set_halign: gtk::Align::Fill,
        set_is_pill: true,
        set_halign: gtk::Align::End,
        #[watch]
        set_label: &t!("next"),
        inline_css: "padding-left: 48px; padding-right: 48px",
        add_css_class: "suggested-action",
        connect_clicked => Self::Input::Nav(NavAction::Next),
    },
);
use i18n_embed::LanguageLoader;
use relm4::RelmIterChildrenExt;
use relm4::SharedState;
use std::rc::Rc;

static SEARCH_STATE: SharedState<gtk::glib::GString> = SharedState::new();

#[derive(Clone, Debug)]
struct LanguageRow {
    locale: &'static str,
    name: &'static str,
    native_name: &'static str,
}

taidan_proc_macros::comptime_localedef_langrows!(LANGUAGE_ROWS);

#[relm4::factory]
impl relm4::factory::FactoryComponent for &'static LanguageRow {
    type Widgets = LanguageRowWidgets;
    type Init = &'static LanguageRow;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = relm4::gtk::ListBox;

    view! {
        #[root]
        gtk::ListBoxRow {
            libhelium::MiniContentBlock {
                set_title: &self.name,
                set_subtitle: &self.native_name,
            }
        }
    }

    fn init_model(
        init: Self::Init,
        _index: &relm4::factory::DynamicIndex,
        _sender: relm4::FactorySender<Self>,
    ) -> Self {
        init
    }
}

#[derive(Debug)]
struct BtnFactory(Rc<relm4::factory::FactoryVecDeque<&'static LanguageRow>>);

impl Default for BtnFactory {
    #[allow(clippy::needless_for_each)]
    fn default() -> Self {
        let mut btnfactory = relm4::factory::FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .detach();

        let mut btns = btnfactory.guard();
        LANGUAGE_ROWS.iter().for_each(|x| _ = btns.push_back(x));
        drop(btns);
        Self(Rc::new(btnfactory))
    }
}

impl std::ops::Deref for BtnFactory {
    type Target = gtk::ListBox;

    fn deref(&self) -> &Self::Target {
        self.0.widget()
    }
}
impl AsRef<gtk::ListBox> for BtnFactory {
    fn as_ref(&self) -> &gtk::ListBox {
        self
    }
}
impl AsRef<gtk::Widget> for BtnFactory {
    fn as_ref(&self) -> &gtk::Widget {
        self.0.widget().upcast_ref()
    }
}

fn set_lang(lang: &LanguageRow) {
    if let Ok(locale) = (lang.locale)
        .split_once('.')
        .map_or(lang.locale, |(left, _)| left)
        .to_owned()
        .parse::<i18n_embed::unic_langid::LanguageIdentifier>()
        .inspect_err(|e| tracing::error!(?e, ?lang, "Cannot apply language"))
    {
        tracing::info!(?locale, lang.locale, "Using selected locale");
        let mut locales = crate::LOCALE_SOLVER
            .solve_locale(locale)
            .into_iter()
            .filter(|li| crate::AVAILABLE_LANGS.contains(li))
            .collect_vec();
        let loader = i18n_embed::fluent::fluent_language_loader!();
        if locales.is_empty() {
            locales.push("en-US".parse().unwrap());
        }
        loader
            .load_languages(&crate::Localizations, &locales)
            .expect("fail to load languages");
        *crate::LL.write() = loader;
        *crate::backend::l10n::PO_LOADER.write() = crate::backend::l10n::new_loader(locales);
        SETTINGS.write().langlocale = lang.locale;
    }
}
