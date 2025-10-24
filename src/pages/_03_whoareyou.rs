use crate::prelude::*;
use regex::Regex;

// /// Check if the entry (assume inside [`libhelium::TextField`]) is valid.
// ///
// /// # Panics
// /// Panic if it is not an `internal_entry` inside [`libhelium::TextField`].
// fn valid_entry(en: &gtk::Text) -> bool {
//     en.parent()
//         .and_then(|x| x.parent())
//         .and_then(|x| x.parent())
//         .and_then(|x| x.parent())
//         .unwrap()
//         .dynamic_cast::<libhelium::TextField>()
//         .unwrap()
//         .is_valid()
// }

fn valid_entry(s: &str) -> bool {
    let re = Regex::new("^[a-z][-a-z0-9_]*$").unwrap();
    re.is_match(s)
}

generate_page!(WhoAreYou {
    lbl_error: gtk::Label,
    btn_next: libhelium::Button,
}:
    init[lbl_error](root, sender, model, widgets) {
        let s0 = sender.clone();
        let s1 = sender.clone();

        model.btn_next = widgets.prev_next_btns.next.clone();
    }
    update(self, message, sender) {
        NotifyFullName(name: String) => {
            let mut settings = SETTINGS.write();
            settings.fullname = if name.is_empty() {
                settings.username.clone()
            } else {
                name
            }
        },
        NotifyUsername(user: String) => {
            let mut settings = SETTINGS.write();
            settings.username = user.clone();
            if settings.fullname.is_empty() {
                settings.fullname = user.clone();
            }
            self.lbl_error.set_visible(false);
            self.btn_next.set_sensitive(true);
        },
        InvalidUsername => {
            self.lbl_error.set_visible(true);
            self.btn_next.set_sensitive(false);
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
            #[watch]
            set_label: &t!("page-whoareyou"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        #[name = "tf_fullname"]
        gtk::Entry {
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            #[watch]
            set_placeholder_text: Some(&t!("page-whoareyou-fullname")),
            connect_changed[sender] => move |e| sender.input(Self::Input::NotifyFullName(e.text().to_string())),
        },

        #[name = "tf_username"]
        gtk::Entry {
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            #[watch]
            set_placeholder_text: Some(&t!("page-whoareyou-username")),
            connect_changed[sender] => move |e| if valid_entry(e.text().as_str()) {
                sender.input(Self::Input::NotifyUsername(e.text().to_string()))
            } else {
                sender.input(Self::Input::InvalidUsername)
            },
            connect_activate[sender] => move |e| if valid_entry(e.text().as_str()) { sender.output(Self::Output::Nav(NavAction::Next)).unwrap() },
        },

        // TODO(lleyton): libhelium::TextField currently has broken tabbing behavior, so we'll use a gtk::Entry for now

        // #[name = "tf_fullname"]
        // libhelium::TextField {
        //     set_hexpand: true,
        //     set_halign: gtk::Align::Fill,
        //     #[watch]
        //     set_placeholder_text: Some(&t!("page-whoareyou-fullname")),
        //     set_is_outline: true,
        // },


        // #[name = "tf_username"]
        // libhelium::TextField {
        //     set_hexpand: true,
        //     set_halign: gtk::Align::Fill,
        //     #[watch]
        //     set_placeholder_text: Some(&t!("page-whoareyou-username")),
        //     set_is_outline: true,
        //     set_needs_validation: true,
        //     set_regex: &libhelium::glib::Regex::new("^[a-z][-a-z0-9_]*$", gtk::glib::RegexCompileFlags::DEFAULT, gtk::glib::RegexMatchFlags::DEFAULT).unwrap().unwrap(),

        //     connect_activate[sender] => move |en| if en.is_valid() { sender.output(Self::Output::Nav(NavAction::Next)).unwrap(); },
        // },

        #[local_ref]
        lbl_error -> gtk::Label {
            #[watch]
            set_label: &t!("page-whoareyou-error"),
            set_use_markup: true,
            set_visible: false,
            add_css_class: "error",
        }
    },

    #[name(prev_next_btns)]
    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            #[watch]
            set_label: &t!("prev"),
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            #[watch]
            set_label: &t!("next"),
            set_sensitive: false,
            connect_clicked => Self::Input::Nav(NavAction::Next),
        },
    }
);
