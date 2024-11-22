crate::generate_page!(Installing:
    update(self, message, sender) {
        // handle UI updates here.
        // NOTE: main.rs should call the start_install() fns.
    } => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_margin_horizontal: 80,
        set_vexpand: true,
        set_hexpand: true,
        set_valign: gtk::Align::Center,
        set_halign: gtk::Align::Fill,

        gtk::Image {
            set_icon_name: Some("distro-icon-symbolic"),
            inline_css: "-gtk-icon-size: 128px",
        },

        gtk::Label {
            set_label: &gettext("Installing your Apps"),
            add_css_class: "view-title",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_label: &gettext("This won't take long!"),
            inline_css: "font-size: 1.25rem",
        },
    },

    // FIXME: libhelium::ProgressBar
    gtk::ProgressBar {
    }
);
