crate::generate_page!(Codecs:
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
            set_label: &gettext("Codecs and Drivers"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_use_markup: true,
            set_justify: gtk::Justification::Center,
            set_label: &gettext("\
                Install proprietary media codecs and drivers for your device.\n\n\
                Consult the \
                <a href='https://wiki.ultramarine-linux.org/en/setup/postinstall/'>wiki</a> \
                if you don't have an Internet connection."),
        },

        #[template] crate::ui::SwitchBox {
            set_title: &gettext("Install Codecs and Drivers"),
            set_subtitle: &gettext("Press next to skip installation"),
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

impl crate::ui::PageTrig for CodecsPage {}
