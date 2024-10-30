crate::generate_page!(WhoAreYou {
    pub name: String,
    pub user: String,
    lbl_error: gtk::Label,
    btn_next: libhelium::Button,
}:
    init[lbl_error](root, sender, model, widgets) {
        let s1 = sender.clone();
        widgets.tf_fullname.internal_entry().connect_changed(move |en| {
            s1.input(Self::Input::NotifyFullName(en.text().to_string()));
        });

        widgets.tf_username.internal_entry().connect_changed(move |en| {
            if en.parent().and_then(|x| x.parent()).and_then(|x| x.parent()).and_then(|x| x.parent()).unwrap().dynamic_cast::<libhelium::TextField>().unwrap().is_valid() {
                sender.input(Self::Input::NotifyUsername(en.text().to_string()));
            } else {
                sender.input(Self::Input::InvalidUsername);
            }
        });

        model.btn_next = widgets.prev_next_btns.next.clone();
    }
    update(self, message, sender) {
        NotifyFullName(name: String) => {
            tracing::trace!(?name, "FullName Input");
            self.name = name;
        },
        NotifyUsername(user: String) => {
            tracing::trace!(?user, "Username Input");
            self.user = user;
            self.lbl_error.set_visible(false);
            self.btn_next.set_sensitive(!self.name.is_empty());
        },
        InvalidUsername => {
            self.lbl_error.set_visible(true);
            self.btn_next.set_sensitive(false);
        }
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
            set_icon_name: Some("system-users-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &gettext("Who are You?"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        #[name = "tf_fullname"]
        libhelium::TextField {
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            set_placeholder_text: Some(&gettext("Full Name")),
            set_is_outline: true,
        },

        #[name = "tf_username"]
        libhelium::TextField {
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            set_placeholder_text: Some(&gettext("Username")),
            set_is_outline: true,
            set_needs_validation: true,
            set_regex: &libhelium::glib::Regex::new(r"^[a-z][-a-z0-9_]*\$?$", gtk::glib::RegexCompileFlags::DEFAULT, gtk::glib::RegexMatchFlags::DEFAULT).unwrap().unwrap(),
        },

        #[local_ref]
        lbl_error -> gtk::Label {
            set_label: &gettext("Username \n- must start with lowercase letters\n- must contain only alphanumericals, underscore (<tt>_</tt>) or dash (<tt>-</tt>)\n- may optionally end with a dollar sign (<tt>$</tt>)"),
            set_use_markup: true,
            set_visible: false,
            add_css_class: "destructive-action",
            inline_css: "color: orange",
        }
    },

    #[name(prev_next_btns)]
    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            set_sensitive: false,
            connect_clicked => Self::Input::Nav(NavAction::Next),
        },
    }
);
