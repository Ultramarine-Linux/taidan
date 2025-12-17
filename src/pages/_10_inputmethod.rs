use crate::prelude::*;
use relm4::{RelmRemoveAllExt, SharedState};

use crate::backend::i18n;

fn miniblk(row: &gtk::ListBoxRow) -> libhelium::MiniContentBlock {
    row.child().unwrap().dynamic_cast().unwrap()
}

static SEARCH_LAYOUT: SharedState<libhelium::glib::GString> = SharedState::new();
static SEARCH_VARIANT: SharedState<libhelium::glib::GString> = SharedState::new();
static CHOSEN_LANG: SharedState<String> = SharedState::new();
// const UMWIKI_INPUT_OTHER_LANG: &str =
//     "https://wiki.ultramarine-linux.org/en/usage/l10n/#inputting-in-another-language";
const UMWIKI_L10N: &str = "https://wiki.ultramarine-linux.org/en/usage/l10n/";

kurage::generate_component!(MoreBox {
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
        
        // Add keyboard event handler for Enter key on language box
        let sender_clone = sender.clone();
        let key_controller_lang = gtk::EventControllerKey::new();
        key_controller_lang.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Return {
                // Navigate to next page when Enter is pressed
                sender_clone.output(NavAction::Next);
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });
        langbox.add_controller(key_controller_lang);
        
        // Add keyboard event handler for Enter key on IM box
        let sender_clone = sender.clone();
        let key_controller_im = gtk::EventControllerKey::new();
        key_controller_im.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Return {
                // Navigate to next page when Enter is pressed
                sender_clone.output(NavAction::Next);
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });
        imbox.add_controller(key_controller_im);
    }
    update(self, message, _sender) {
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
    } => NavAction

    gtk::Grid {
        set_row_spacing: 4,
        set_column_spacing: 10,
        set_vexpand: true,
        set_hexpand: true,
        set_column_homogeneous: true,

        #[name(searchlayout)]
        attach[0, 0, 1, 1] = &libhelium::TextField {
            set_is_search: true,
            set_is_outline: true,
            set_margin_top: 6,
            set_margin_bottom: 6,
            set_prefix_icon: Some("system-search-symbolic"),
            set_placeholder_text: Some(&t!("page-inputmethod-search-lang")),
        },
        attach[0, 1, 1, 1] = &gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            #[local_ref]
            langbox -> gtk::ListBox {
                add_css_class: "content-list",
                set_selection_mode: gtk::SelectionMode::Single,
                set_vexpand: true,
                connect_selected_rows_changed => Self::Input::LangSelected,
            }
        },

        #[name(searchvariant)]
        attach[1, 0, 1, 1] = &libhelium::TextField {
            set_is_search: true,
            set_is_outline: true,
            set_margin_top: 6,
            set_margin_bottom: 6,
            set_prefix_icon: Some("system-search-symbolic"),
            set_placeholder_text: Some(&t!("page-inputmethod-search-ims")),
        },
        attach[1, 1, 1, 1] = &gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            #[local_ref]
            imbox -> gtk::ListBox {
                add_css_class: "content-list",
                set_selection_mode: gtk::SelectionMode::None,
                set_vexpand: true,
            }
        },
    }
);

skipconfig!();
generate_page!(InputMethod {
    more: bool,
    morebox: gtk::Box,
}:
    init[morebox { model.morebox.clone() }](root, sender, model, widgets) {
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
        root.add_controller(key_controller);
    }
    update(self, message, sender) {
        More => {
            self.more = true;
            let mut morebox = MoreBox::builder().launch(()).forward(sender.input_sender(), Self::Input::Nav);
            morebox.detach_runtime();
            self.morebox.append(morebox.widget());
        },
    } => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_margin_bottom: 16,
        #[watch]
        set_valign: if model.more { gtk::Align::Fill } else { gtk::Align::Center },
        set_halign: gtk::Align::Fill,
        set_vexpand: true,
        set_hexpand: true,

        gtk::Image {
            set_icon_name: Some("input-keyboard-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            #[watch]
            set_label: &t!("page-inputmethod"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        #[transition = "SlideLeft"]
        if model.more {
            #[local]
            morebox -> gtk::Box {}
        } else {
            gtk::Box {
                set_spacing: 20,
                set_orientation: gtk::Orientation::Vertical,

                gtk::Label {
                    set_valign: gtk::Align::Center,
                    set_use_markup: true,
                    set_wrap: true,
                    set_wrap_mode: gtk::pango::WrapMode::Word,
                    #[watch]
                    set_label: &t!("page-inputmethod-desc", wiki = format!("<a href='{UMWIKI_L10N}'>{}</a>", t!("page-inputmethod-wiki"))),
                    set_justify: gtk::Justification::Center,
                },

                libhelium::Button {
                    set_color: libhelium::ButtonColor::Secondary,
                    #[watch]
                    set_label: &t!("page-inputmethod-add"),
                    set_is_pill: true,
                    set_halign: gtk::Align::Center,
                    connect_clicked => Self::Input::More,
                    inline_css: "padding-left: 48px; padding-right: 48px",
                },
            }
        },
    },

    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            #[watch]
            set_label: &t!("prev"),
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            #[watch]
            set_label: &if model.more { t!("next") } else { t!("skip") },
            connect_clicked => Self::Input::Nav(NavAction::Next),
        },
    }
);
