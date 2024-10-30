crate::generate_page!(Welcome:
    update(self, message, sender) {} => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_vexpand: true,
        set_valign: gtk::Align::Center,
        set_halign: gtk::Align::Center,

        gtk::Image {
            set_icon_name: Some("distro-icon-name-symbolic"),
            inline_css: "-gtk-icon-size: 128px",
        },

        gtk::Label {
            set_label: &gettext("Welcome to %s").replace("%s", &CONFIG.read().distro),
            add_css_class: "view-title",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_label: &gettext("Let's get your system ready."),
            inline_css: "font-size: 1.25rem",
        },
    },

    gtk::Box {
        set_valign: gtk::Align::End,
        set_halign: gtk::Align::Center,
        set_hexpand: true,
        set_orientation: gtk::Orientation::Horizontal,

        libhelium::Button {
            set_is_pill: true,
            set_valign: gtk::Align::End,
            set_halign: gtk::Align::Center,
            set_label: &gettext("Let's Go"),
            inline_css: "padding-left: 48px; padding-right: 48px",
            add_css_class: "suggested-action",
            connect_clicked => Self::Input::Nav(NavAction::Next),
        },
    }
);
