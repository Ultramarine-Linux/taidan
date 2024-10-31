crate::generate_page!(Theme {
    pub is_dark: bool,
    pub accent: usize,
}:
    init(root, sender, model, widgets) {
        let (ctl_light, ctl_dark) = (gtk::GestureClick::new(), gtk::GestureClick::new());
        ctl_light.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
        ctl_dark.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
        let (s0, s1) = (sender.clone(), sender);
        ctl_light.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            s0.input(Self::Input::ClickLight);
        });
        ctl_dark.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            s1.input(Self::Input::ClickDark);
        });
        widgets.lightbox.add_controller(ctl_light);
        widgets.darkbox.add_controller(ctl_dark);
    }
    update(self, message, sender) {
        ClickLight => self.is_dark = false,
        ClickDark => self.is_dark = true,
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
            set_icon_name: Some("dialog-question-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &gettext("Choose your theme"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_use_markup: true,
            set_label: &gettext("Make this system your own with a splash of colour.\nYou can change this option later in settings."),
            set_justify: gtk::Justification::Center,
        },

        gtk::Label {
            set_use_markup: true,
            set_label: &gettext("Some apps may not respect this preference."),
            add_css_class: "caption",
        },

        gtk::Box {
            set_spacing: 32,
            set_orientation: gtk::Orientation::Horizontal,
            set_halign: gtk::Align::Center,

            #[name(lightbox)]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                gtk::Frame {
                    set_expand: false,
                    gtk::Image {
                        set_from_file: Some("/usr/share/backgrounds/default.png"),
                        set_pixel_size: 200,
                        // inline_css: "border-radius: 25pt;",
                    },
                },
                gtk::Label {
                    set_label: &*gettext("Light"),
                },
            },
            #[name(darkbox)]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                gtk::Frame {
                    set_expand: false,
                    gtk::Image {
                        set_from_file: Some("/usr/share/backgrounds/default-dark.png"),
                        set_pixel_size: 200,
                        // inline_css: "border-radius: 25pt;",
                    },
                },
                gtk::Label {
                    set_label: &*gettext("Dark"),
                },
            },
        },

        // TODO: list of colour buttons
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
