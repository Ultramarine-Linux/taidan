use relm4::SharedState;

static SEARCH_STATE: SharedState<gtk::glib::GString> = SharedState::new();

crate::generate_page!(Keyboard {
    btnbox: gtk::ListBox,
}:
    init[btnbox](root, sender, model, widgets) {
        widgets.search.internal_entry().connect_changed(move |en| {
            *SEARCH_STATE.write() = en.text();
            btnfactory.widget().invalidate_filter();
            tracing::trace!(?en, "Search Changed!");
        });
    }
    update(self, message, sender) {
        Selected => todo!(),
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
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 4,
        set_vexpand: true,

        #[name(search)]
        libhelium::TextField {
            set_is_search: true,
            set_is_outline: true,
            set_margin_top: 6,
            set_margin_bottom: 6,
            set_prefix_icon: Some("system-search-symbolic"),
            #[watch]
            set_placeholder_text: Some(&gettext("Search…")),
        },
        gtk::ScrolledWindow {
            #[local_ref]
            btnbox -> gtk::ListBox {
                add_css_class: "content-list",
                set_selection_mode: gtk::SelectionMode::Single,
                set_vexpand: true,
                set_hexpand: true,
                set_valign: gtk::Align::Center,
                set_halign: gtk::Align::Center,
                connect_selected_rows_changed => Self::Input::Selected,
            }
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
