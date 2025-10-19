const WIKI_POSTINSTALL: &str = "https://wiki.ultramarine-linux.org/en/setup/postinstall/";

use crate::prelude::*;
skipconfig!();
generate_page!(Codecs:
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
            set_icon_name: Some("computer-laptop-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &t!("page-codecs"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_use_markup: true,
            set_justify: gtk::Justification::Center,
            // FIXME: someone tell me how to do this properly
            set_label: &format!("{}\n{}", t!("page-codecs-desc1"), t!("page-codecs-desc2", wiki = format!("<a href='{WIKI_POSTINSTALL}'>{}</a>", t!("page-codecs-wiki")))),
        },

        #[template] crate::ui::SwitchBox {
            set_title: &t!("switch-codecs"),
            set_subtitle: &t!("switch-codecs-desc"),
            #[template_child] switch {
                set_active: true,
                connect_state_set => move |_, state| {
                    SETTINGS.write().install_codecs_drivers = state;
                    glib::Propagation::Proceed
                },
            }
        },
    },

    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            connect_clicked => Self::Input::Nav(NavAction::Next),
        },
    }
);
