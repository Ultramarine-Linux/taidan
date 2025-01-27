use crate::macros::{generate_page, kurage_page_pre};
generate_page!(Password {
    btn_next: libhelium::Button,
    tf_repeat: gtk::PasswordEntry,
}:
    init[tf_repeat](root, sender, model, widgets) {
        model.btn_next = widgets.prev_next_btns.next.clone();
        let tfr = tf_repeat.clone();
        widgets.tf_passwd.connect_activate(move |_| _ = tfr.grab_focus());
    }
    update(self, message, sender) {
        SetBtnSensitive(b: bool) => self.btn_next.set_sensitive(b),
        Enter => {
            if self.btn_next.is_sensitive() {
                sender.input(Self::Input::Nav(NavAction::Next));
            }
        }
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
            connect_changed[sender, tf_repeat] => move |en| {
                sender.input(Self::Input::SetBtnSensitive(en.text() == tf_repeat.text() && !en.text().is_empty()));
                SETTINGS.write().passwd = en.text().to_string();
            },
        },

        #[local_ref] tf_repeat ->
        gtk::PasswordEntry {
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            set_show_peek_icon: true,
            set_placeholder_text: Some(&gettext("Repeat Password")),
            connect_changed[sender] => move |en| {
                let pass = en.text().to_string();
                sender.input(Self::Input::SetBtnSensitive(SETTINGS.read().passwd == pass && !pass.is_empty()));
            },
            connect_activate => Self::Input::Enter,
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

impl crate::ui::PageTrig for PasswordPage {}
