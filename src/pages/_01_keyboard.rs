use relm4::{RelmIterChildrenExt, RelmRemoveAllExt, SharedState};

use crate::backend::i18n;

fn miniblk(row: &gtk::ListBoxRow) -> libhelium::MiniContentBlock {
    row.child().unwrap().dynamic_cast().unwrap()
}

static SEARCH_LAYOUT: SharedState<libhelium::glib::GString> = SharedState::new();
static SEARCH_VARIANT: SharedState<libhelium::glib::GString> = SharedState::new();

crate::generate_page!(Keyboard {
    layoutbox: gtk::ListBox,
    variantbox: gtk::ListBox,
}:
    init[layoutbox variantbox](root, sender, model, widgets) {
        i18n::LAYOUTS.entries()
            .map(|(&layout, i18n::Layout { name, .. })| libhelium::MiniContentBlock::builder().subtitle(*name).title(layout).build())
            .map(|mini_content_block| gtk::ListBoxRow::builder().child(&mini_content_block).build())
            .for_each(|row| model.layoutbox.append(&row));
        model.layoutbox.select_row(model.layoutbox.iter_children().find(|child| miniblk(child).title() == "us").as_ref());
        let layoutbox2 = layoutbox.clone();
        let variantbox2 = variantbox.clone();
        widgets.searchlayout.internal_entry().connect_changed(move |en| {
            *SEARCH_LAYOUT.write() = en.text();
            layoutbox2.invalidate_filter();
        });
        layoutbox.set_filter_func(move |row| {
            let search = SEARCH_LAYOUT.read().to_ascii_lowercase();
            miniblk(row).title().contains(&search) || miniblk(row).subtitle().contains(&search)
        });
        widgets.searchvariant.internal_entry().connect_changed(move |en| {
            *SEARCH_VARIANT.write() = en.text();
            variantbox2.invalidate_filter();
        });
        variantbox.set_filter_func(move |row| {
            let search = SEARCH_VARIANT.read().to_ascii_lowercase();
            miniblk(row).title().contains(&search) || miniblk(row).subtitle().contains(&search)
        });
    }
    update(self, message, sender) {
        LayoutSelected => {
            self.variantbox.remove_all();
            self.variantbox.append(&gtk::ListBoxRow::builder().child(&libhelium::MiniContentBlock::builder().subtitle(gettext("Default")).build()).build());
            let row = self.layoutbox.selected_row().unwrap();
            let layout = miniblk(&row).title().to_string();
            i18n::LAYOUTS[&layout].variants.entries()
                .map(|(&variant, &desc)| gtk::ListBoxRow::builder().child(&libhelium::MiniContentBlock::builder().subtitle(desc).title(variant).build()).build())
                .for_each(|row| self.variantbox.append(&row));
            SETTINGS.write().kb_layout.clone_from(&layout);
            sender.oneshot_command(async move { i18n::set_keymap(None, &layout, None).await.expect("cannot set keymap") });
        },
        VariantSelected => {
            let Some(row) = self.variantbox.selected_row() else { return };
            let variant = (miniblk(&row).subtitle() != "Default").then(|| miniblk(&row).title().to_string());
            SETTINGS.write().kb_variant.clone_from(&variant);
            let layout = SETTINGS.read().kb_layout.clone();
            sender.oneshot_command(async move { i18n::set_keymap(None, &layout, variant.as_deref()).await.expect("cannot set keymap") });
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
            set_label: &gettext("Keyboard Layout"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
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
                set_placeholder_text: Some(&gettext("Search keyboard layout…")),
            },
            gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk::PolicyType::Never,
                #[local_ref]
                layoutbox -> gtk::ListBox {
                    add_css_class: "content-list",
                    set_selection_mode: gtk::SelectionMode::Single,
                    set_vexpand: true,
                    connect_selected_rows_changed => Self::Input::LayoutSelected,
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
                set_placeholder_text: Some(&gettext("Search keyboard variant…")),
            },
            gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk::PolicyType::Never,
                #[local_ref]
                variantbox -> gtk::ListBox {
                    add_css_class: "content-list",
                    set_selection_mode: gtk::SelectionMode::Single,
                    set_vexpand: true,
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
            connect_clicked => Self::Input::Nav(NavAction::Next),
        },
    }
);

impl crate::ui::PageTrig for KeyboardPage {}
