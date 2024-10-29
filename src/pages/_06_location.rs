crate::generate_page!(Location {
    pub service: bool,
}:
    update(self, message, sender) {
        On => {
            tracing::trace!("location service on");
            self.service = true;
        },
        Off => {
            tracing::trace!("location service off");
            self.service = false;
        },
    } => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_margin_horizontal: 128,
        set_vexpand: true,
        set_hexpand: true,
        set_valign: gtk::Align::Center,
        set_halign: gtk::Align::Fill,

        gtk::Image {
            set_icon_name: Some("question-round-outline-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &gettext("Location Services"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_use_markup: true,
            set_label: &gettext("Allow apps to request your approximate location with [Mozilla Location Services]"),
        },

        libhelium::MiniContentBlock {
            set_title: &gettext("Location Services"),

            #[wrap(Some)]
            #[name = "switch"]
            set_widget = &libhelium::Switch {
                connect_left_icon_notify => Self::Input::On,
                connect_right_icon_notify => Self::Input::Off,
            }
        }
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
