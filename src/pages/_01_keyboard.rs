use relm4::RelmRemoveAllExt;

use crate::backend::i18n;

crate::generate_page!(Keyboard {
    layouts: Vec<String>,
    layoutbox: gtk::ListBox,
    variantbox: gtk::ListBox,
}:
    init[layoutbox variantbox](root, sender, model, widgets) {
        model.layouts = i18n::list_layouts().unwrap();
        let layoutbox2 = layoutbox.clone();
        let variantbox2 = variantbox.clone();
        widgets.searchlayout.internal_entry().connect_changed(move |en| {
            SETTINGS.write().kb_layout = en.text().to_string();
            layoutbox2.invalidate_filter();
        });
        let layouts1 = model.layouts.clone();
        layoutbox.set_filter_func(move |row| {
            layouts1[row.index() as usize].contains(&SETTINGS.write().kb_layout)
        });
        widgets.searchvariant.internal_entry().connect_changed(move |en| {
            SETTINGS.write().kb_variant = en.text().to_string();
            variantbox2.invalidate_filter();
        });
        variantbox.set_filter_func(move |row| {
            let sett = SETTINGS.read();
            let variants = i18n::list_variants(&sett.kb_layout).unwrap();
            variants[row.index() as usize].contains(&sett.kb_variant)
        });
    }
    update(self, message, sender) {
        LayoutSelected => {
            self.variantbox.remove_all();
            let index = self.layoutbox.selected_row().unwrap().index() as usize;
            SETTINGS.write().kb_layout = self.layouts[index].clone();
            i18n::list_variants(&self.layouts[index]).expect("can't list variants").iter()
                .map(|variant| gtk::ListBoxRow::builder().child(&gtk::Label::new(Some(variant))).build())
                .for_each(|row| self.variantbox.append(&row));
        },
        VariantSelected => {
            let mut sett = SETTINGS.write();
            let variants = i18n::list_variants(&sett.kb_layout).unwrap();
            let index = self.variantbox.selected_row().unwrap().index() as usize;
            sett.kb_variant = variants[index].clone();
        },
    } => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_vexpand: true,
        set_valign: gtk::Align::Center,
        set_halign: gtk::Align::Center,

        gtk::Image {
            set_icon_name: Some("input-keyboard-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &gettext("Keyboard Layout"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },
    },

    gtk::Box {
        set_orientation: gtk::Orientation::Horizontal,
        set_spacing: 4,
        set_vexpand: true,

        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            set_vexpand: true,
            set_hexpand: true,
            set_halign: gtk::Align::Center,

            #[name(searchlayout)]
            libhelium::TextField {
                set_is_search: true,
                set_is_outline: true,
                set_margin_top: 6,
                set_margin_bottom: 6,
                set_prefix_icon: Some("system-search-symbolic"),
                #[watch]
                set_placeholder_text: Some(&gettext("Search keyboard layout…")),
            },
            gtk::ScrolledWindow {
                #[local_ref]
                layoutbox -> gtk::ListBox {
                    add_css_class: "content-list",
                    set_selection_mode: gtk::SelectionMode::Single,
                    set_vexpand: true,
                    set_hexpand: true,
                    set_valign: gtk::Align::Center,
                    set_halign: gtk::Align::Center,
                    connect_selected_rows_changed => Self::Input::LayoutSelected,
                }
            },
        },

        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            set_vexpand: true,
            set_hexpand: true,
            set_halign: gtk::Align::Center,

            #[name(searchvariant)]
            libhelium::TextField {
                set_is_search: true,
                set_is_outline: true,
                set_margin_top: 6,
                set_margin_bottom: 6,
                set_prefix_icon: Some("system-search-symbolic"),
                #[watch]
                set_placeholder_text: Some(&gettext("Search keyboard variant…")),
            },
            gtk::ScrolledWindow {
                #[local_ref]
                variantbox -> gtk::ListBox {
                    add_css_class: "content-list",
                    set_selection_mode: gtk::SelectionMode::Single,
                    set_vexpand: true,
                    set_hexpand: true,
                    set_valign: gtk::Align::Center,
                    set_halign: gtk::Align::Center,
                    connect_selected_rows_changed => Self::Input::VariantSelected,
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
            set_sensitive: false,
            connect_clicked => Self::Input::Nav(NavAction::Next),
        },
    }
);
