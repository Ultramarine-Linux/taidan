use relm4::{RelmRemoveAllExt, SharedState};

use crate::backend::i18n;

fn miniblk(row: &gtk::ListBoxRow) -> libhelium::MiniContentBlock {
    row.child().unwrap().dynamic_cast().unwrap()
}

static SEARCH_LAYOUT: SharedState<libhelium::glib::GString> = SharedState::new();
static SEARCH_VARIANT: SharedState<libhelium::glib::GString> = SharedState::new();
static CHOSEN_LANG: SharedState<String> = SharedState::new();

crate::generate_page!(InputMethod {
    langbox: gtk::ListBox,
    imbox: gtk::ListBox,
}:
    init[langbox imbox](root, sender, model, widgets) {
        i18n::STR_TO_LANG.entries()
            .map(|(id, lang)| libhelium::MiniContentBlock::builder().title(lang.display()).subtitle(*id).build())
            .map(|mini_content_block| gtk::ListBoxRow::builder().child(&mini_content_block).build())
            .for_each(|row| model.langbox.append(&row));
        // model.langbox.select_row(langbox.first_child().as_ref());
        let langbox2 = langbox.clone();
        let imbox2 = imbox.clone();
        widgets.searchlayout.internal_entry().connect_changed(move |en| {
            *SEARCH_LAYOUT.write() = en.text();
            langbox2.invalidate_filter();
        });
        langbox.set_filter_func(move |row| {
            let search = SEARCH_LAYOUT.read().to_ascii_lowercase();
            miniblk(row).title().contains(&search) || miniblk(row).subtitle().contains(&search)
        });
        widgets.searchvariant.internal_entry().connect_changed(move |en| {
            *SEARCH_VARIANT.write() = en.text();
            imbox2.invalidate_filter();
        });
        imbox.set_filter_func(move |row| {
            let search = SEARCH_VARIANT.read().to_ascii_lowercase();
            miniblk(row).title().contains(&search) || miniblk(row).subtitle().contains(&search)
        });
    }
    update(self, message, sender) {
        LangSelected => {
            self.imbox.remove_all();
            let row = self.langbox.selected_row().unwrap();
            let lang = miniblk(&row).subtitle().to_string();
            i18n::IMS[&lang].entries()
                .filter(|(_, im)| im.available())
                .map(|(imname, im)| im.make_listboxrow(imname))
                .for_each(|row| self.imbox.append(&row));
            CHOSEN_LANG.write().clone_from(&lang);
        },
    } => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_margin_bottom: 16,
        set_valign: gtk::Align::Fill,
        set_halign: gtk::Align::Fill,

        gtk::Image {
            set_icon_name: Some("input-keyboard-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &gettext("Input Method"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_valign: gtk::Align::Center,
            set_use_markup: true,
            set_wrap: true,
            set_wrap_mode: gtk::pango::WrapMode::Word,
            // TRANSLATORS: the section regarding Chinese input methods is optional. You may choose
            // to not translate this part.
            set_label: &gettext("You may <b>optionally</b> choose to add an \
            <a href='https://wiki.ultramarine-linux.org/en/usage/l10n/#inputting-in-another-language'>\
                input method editor (IME)\
            </a>. This allows you to type in other specific languages. \
            This change will take effect after you login into your user account.\n\n\
            More Chinese input method options may be available with the \
            <a href='https://rime.im/'>Rime</a> engine, but this requires advanced configuration \
            and is therefore not recommended to beginners.\n\n\
            For Japanese IMEs, we recommend Mozc for KDE Plasma users. Users on other editions and \
            DEs may choose both Anthy and Mozc. Anthy is outdated and unmaintained, but it matches \
            the user interface provided by IBus, while Mozc provides better conversions.\n\n\
            You may find out more information on \
            <a href='https://wiki.ultramarine-linux.org/en/usage/l10n/'>the wiki</a>."),
            set_justify: gtk::Justification::Center,
        },
    },

    gtk::Box {
        set_orientation: gtk::Orientation::Horizontal,
        set_spacing: 4,
        set_vexpand: true,
        set_hexpand: true,

        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            set_vexpand: true,
            set_hexpand: true,
            set_halign: gtk::Align::Fill,

            #[name(searchlayout)]
            libhelium::TextField {
                set_is_search: true,
                set_is_outline: true,
                set_margin_top: 6,
                set_margin_bottom: 6,
                set_prefix_icon: Some("system-search-symbolic"),
                #[watch]
                set_placeholder_text: Some(&gettext("Search language…")),
            },
            gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk::PolicyType::Never,
                #[local_ref]
                langbox -> gtk::ListBox {
                    add_css_class: "content-list",
                    set_selection_mode: gtk::SelectionMode::Single,
                    set_vexpand: true,
                    connect_selected_rows_changed => Self::Input::LangSelected,
                }
            },
        },

        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            set_vexpand: true,
            set_hexpand: true,
            set_halign: gtk::Align::Fill,

            #[name(searchvariant)]
            libhelium::TextField {
                set_is_search: true,
                set_is_outline: true,
                set_margin_top: 6,
                set_margin_bottom: 6,
                set_prefix_icon: Some("system-search-symbolic"),
                #[watch]
                set_placeholder_text: Some(&gettext("Search IMs/IMEs…")),
            },
            gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk::PolicyType::Never,
                #[local_ref]
                imbox -> gtk::ListBox {
                    add_css_class: "content-list",
                    set_selection_mode: gtk::SelectionMode::None,
                    set_vexpand: true,
                }
            },
        },
    },

    #[name(prev_next_btns)]
    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            connect_clicked => Self::Input::Nav(NavAction::Next),
        },
    }
);
