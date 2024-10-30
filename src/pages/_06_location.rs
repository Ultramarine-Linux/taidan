crate::generate_page!(Location {
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
            set_widget = &gtk::Switch {
                connect_state_set[sender] => move |_, state| {
                    sender.input(Self::Input::Switch(state));
                    glib::Propagation::Proceed
                },
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