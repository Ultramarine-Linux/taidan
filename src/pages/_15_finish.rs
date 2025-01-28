use crate::prelude::*;
generate_page!(Finish:
    update(self, message, sender) {} => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_vexpand: true,
        set_valign: gtk::Align::Center,
        set_halign: gtk::Align::Center,

        gtk::Image {
            set_icon_name: Some("distro-icon-symbolic"),
            inline_css: "-gtk-icon-size: 128px",
        },

        gtk::Label {
            set_label: &gettext("Your System is Ready"),
            add_css_class: "view-title",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_label: &gettext("We hope you enjoy your fresh system!"),
            inline_css: "font-size: 1.25rem",
        },
    },

    libhelium::Button {
        set_is_pill: true,
        set_halign: gtk::Align::End,
        set_label: &gettext("Done"),
        inline_css: "padding-left: 48px; padding-right: 48px",
        add_css_class: "suggested-action",
        connect_clicked => Self::Input::Nav(NavAction::Quit),
    },
);

impl crate::ui::PageTrig for FinishPage {}
