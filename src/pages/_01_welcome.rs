use crate::prelude::*;
generate_page!(Welcome:
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
            set_label: &t!("page-welcome", distro = CFG.distro.clone()),
            add_css_class: "view-title",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_label: &t!("page-welcome-ready"),
            inline_css: "font-size: 1.25rem",
        },
    },

    gtk::Box {
        set_valign: gtk::Align::End,
        set_halign: gtk::Align::Fill,
        set_hexpand: true,
        set_orientation: gtk::Orientation::Horizontal,

        // https://github.com/Ultramarine-Linux/taidan/issues/79
        // libhelium::Button {
        //     set_is_textual: true,
        //     set_halign: gtk::Align::Start,
        //     set_label: &t!("page-welcome-skipcfg"),
        //     connect_clicked[sender] => move |_| {
        //         SETTINGS.write().skipconfig = true;
        //         sender.input(Self::Input::Nav(NavAction::Next));
        //     },
        // },

        gtk::Box { set_halign: gtk::Align::Fill, set_hexpand: true },

        libhelium::Button {
            set_is_pill: true,
            set_halign: gtk::Align::End,
            set_label: &t!("page-welcome-go"),
            inline_css: "padding-left: 48px; padding-right: 48px",
            add_css_class: "suggested-action",
            connect_clicked[sender] => move |_| {
                SETTINGS.write().skipconfig = false;
                sender.input(Self::Input::Nav(NavAction::Next));
            },
        },
    }
);
