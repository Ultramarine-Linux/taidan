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
                    SETTINGS.write().langlocale = "en-US".to_owned();
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
// This is a list of languages sorted by total speakers:
// https://en.wikipedia.org/wiki/List_of_languages_by_total_number_of_speakers
// (2024-08-17)
//
// These are filtered by our Ultramarine website plausible statistics and the 5 most popular
// langauges in the world.
const POPULAR_LANGS: [&str; 9] = [
    "en_US", "zh_CN", "zh_TW", "hi_IN", "es_ES", "ar_AE", "fr_FR", "pt_BR", "de_DE",
];

#[derive(Clone, Debug)]
struct LanguageRow {
    locale: String,
    name: String,
    native_name: &'static str,
}

impl LanguageRow {
    fn list(f: impl FnMut(Self)) {
        // FIXME: maybe use some C API instead?
        let mut cmd = std::process::Command::new("localedef");
        cmd.arg("--list-archive")
            .stdout(std::process::Stdio::piped());
        let stdout = cmd.output().expect("cannot run localedef").stdout;
        (stdout.split(|&b| b == b'\n'))
            .filter(|v| !v.contains(&b'.') && !v.contains(&b'@'))
            .filter_map(|locale| {
                poly_l10n::LanguageIdentifier::from_bytes(locale)
                    .ok()
                    .map(|l| (l, locale))
            })
            .filter_map(|(langid, locale)| {
                let locale = core::str::from_utf8(locale).ok()?.to_owned();
                let lang = isolang::Language::from_locale(&locale)?;
                let name = langid
                    .region
                    .and_then(|r| iso3166::Country::from_alpha2_ignore_case(r.as_str()))
                    .as_ref()
                    .map_or_else(
                        || lang.to_name().to_owned(),
                        |region| format!("{} ({})", lang.to_name(), region.name),
                    );

                Some(Self {
                    locale,
                    name,
                    native_name: lang.to_autonym().unwrap_or_default(),
                })
            })
            .for_each(f);
    }
}

#[relm4::factory]
impl relm4::factory::FactoryComponent for LanguageRow {
    type Init = Self;
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
struct BtnFactory(Rc<relm4::factory::FactoryVecDeque<LanguageRow>>);

impl Default for BtnFactory {
    fn default() -> Self {
        let mut btnfactory = relm4::factory::FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .detach();

        let mut btns = btnfactory.guard();
        LanguageRow::list(|x| _ = btns.push_back(x));
        btns.push_back(LanguageRow {
            locale: "en-owo".to_owned(),
            name: "English (OWO)".to_owned(),
            native_name: "OWO",
        });
        btns.drop();

        // sort the popular languages, put to top
        for lang in POPULAR_LANGS.iter().rev() {
            let Some(index) = btnfactory
                .iter()
                .position(|l: &LanguageRow| l.locale.starts_with(lang))
            else {
                continue;
            };
            let Some(x) = btnfactory.guard().remove(index) else {
                unreachable!()
            };
            btnfactory.guard().push_front(x);
        }

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
    tracing::info!(lang.locale, "Using selected locale");
    if let Ok(locale) = lang
        .locale
        .split_once('.')
        .map_or(&*lang.locale, |(left, _)| left)
        .to_owned()
        .parse::<i18n_embed::unic_langid::LanguageIdentifier>()
        .inspect_err(|e| tracing::error!(?e, "Cannot apply language"))
    {
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
        tracing::debug!(lang=?loader.current_languages(), welcome=loader.get_args_concrete("page-welcome", std::iter::once(("distro", "Ultramarine Linux".into())).collect()), "new loader");
        *crate::LL.write() = loader;
        SETTINGS.write().langlocale.clone_from(&lang.locale);
    }
}
