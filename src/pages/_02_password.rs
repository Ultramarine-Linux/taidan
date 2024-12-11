crate::generate_page!(Password {
    pub passwd: String,
    btn_next: libhelium::Button,
}:
    init(root, sender, model, widgets) {
        model.btn_next = widgets.prev_next_btns.next.clone();
        let tf_repeat = widgets.tf_repeat.clone();
        widgets.tf_passwd.connect_activate(move |_| _ = tf_repeat.grab_focus());
    }
    update(self, message, sender) {
        NotifyPasswd(pass: String) => self.passwd = pass,
        NotifyRepeat(pass: String) => self.btn_next.set_sensitive(self.passwd == pass && !pass.is_empty()),
        NotifyActivate => if self.btn_next.is_sensitive() { sender.input(Self::Input::Nav(NavAction::Next)) },
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
            set_icon_name: Some("system-users-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &gettext("Create a Password"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        #[name = "tf_passwd"]
        gtk::PasswordEntry {
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            set_show_peek_icon: true,
            set_placeholder_text: Some(&gettext("Password")),
            connect_changed[sender] => move |en| sender.input(Self::Input::NotifyPasswd(en.text().to_string())),
        },

        #[name = "tf_repeat"]
        gtk::PasswordEntry {
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            set_show_peek_icon: true,
            set_placeholder_text: Some(&gettext("Repeat Password")),
            connect_changed[sender] => move |en| sender.input(Self::Input::NotifyRepeat(en.text().to_string())),
            connect_activate => Self::Input::NotifyActivate,
        },
    },

    #[name(prev_next_btns)]
    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            connect_clicked => Self::Input::Nav(NavAction::Next),
            set_sensitive: false,
        },
    }
);
