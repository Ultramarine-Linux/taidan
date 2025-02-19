use crate::prelude::*;
generate_page!(Analytics:
    update(self, message, sender) {} => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_margin_horizontal: 80,
        set_vexpand: true,
        set_hexpand: true,
        set_valign: gtk::Align::Center,
        set_halign: gtk::Align::Fill,

        gtk::Image {
            set_icon_name: Some("eye-open-negative-filled-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            // TRANSLATORS: this is unused, do NOT translate
            set_label: &t!("page-analytics"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            // TODO: description for analytics
        }

        // TODO: buttons for confirming / rejecting analytics
    },
);

crate::always_skip_page!(Analytics);
