use crate::prelude::*;
generate_page!(Tweaks:
    init(root, sender, model, widgets) {

    }
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
            set_icon_name: Some("systemsettings-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &t!("page-location"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        #[template] crate::ui::SwitchBox {
            set_title: &t!("page-location"),
            #[template_child] switch {
                connect_state_set[sender] => move |_, state| {
                    sender.input(Self::Input::Switch(state));
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
