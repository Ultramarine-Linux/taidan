use crate::prelude::*;
generate_page!(Location {
    pub service: bool,
}:
    update(self, message, sender) {
        Switch(active: bool) => {
            tracing::trace!("location service active: {active}");
            self.service = active;
        },
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
            set_icon_name: Some("location-active-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            // TRANSLATORS: this is unused, do NOT translate
            set_label: &t!("page-location"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_use_markup: true,
            // TRANSLATORS: this is unused, do NOT translate
            set_label: &t!("page-location-desc"),
        },

        #[template] crate::ui::SwitchBox {
            // TRANSLATORS: this is unused, do NOT translate
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
fn page_skipconfig() -> bool {
    true
}
