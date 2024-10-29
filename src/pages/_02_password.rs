crate::generate_page!(Password {
    pub passwd: String,
}:
    init(root, sender) {
        let model = Self::default();
        let widgets = view_output!();
        let s1 = sender.clone();
        widgets.tf_passwd.internal_entry().connect_changed(move |en| {
            s1.input(Self::Input::NotifyFullName(en.text().to_string()));
        });

        let s2 = sender.clone();
        widgets.tf_repeat.internal_entry().connect_changed(move |en| {
            s2.input(Self::Input::NotifyUsername(en.text().to_string()));
        });

        ComponentParts { model, widgets }
    }
    update(self, message, sender) {
        NotifyPasswd(pass: String) => {
            tracing::trace!(?pass, "Password Input");
            self.passwd = pass;
        },
        NotifyRepeat(pass: String) => {
            todo!()
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
            set_icon_name: Some("people-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &gettext("Create a Password"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        #[name = "tf_passwd"]
        libhelium::TextField {
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            set_support_text: Some(&gettext("Password")),
            set_is_outline: true,
        },

        #[name = "tf_repeat"]
        libhelium::TextField {
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            set_support_text: Some(&gettext("Repeat Password")),
            set_is_outline: true,
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
