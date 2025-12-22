use regex::Regex;

use crate::prelude::*;

fn valid_entry(s: &str) -> bool {
    let re = Regex::new(r"^(([a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9\-]*[a-zA-Z0-9])\.)*([A-Za-z0-9]|[A-Za-z0-9][A-Za-z0-9\-]*[A-Za-z0-9])$").unwrap();
    re.is_match(s)
}

#[allow(irrefutable_let_patterns)]
fn autoset_hostname(en: &gtk::Entry, hostname: &str) {
    if hostname.is_empty() {
        return;
    }
    let user;
    if let first = hostname
        .split_once(|c: char| c.is_whitespace())
        .map_or(hostname, |(a, _)| a)
        && first.chars().all(|c: char| c.is_ascii_alphanumeric())
        && first.chars().next().unwrap().is_ascii_alphabetic()
    {
        user = first.to_ascii_lowercase();
    } else if let last = hostname
        .rsplit_once(|c: char| c.is_whitespace())
        .map_or(hostname, |(_, b)| b)
        && last.chars().all(|c: char| c.is_ascii_alphanumeric())
        && last.chars().next().unwrap().is_ascii_alphabetic()
    {
        user = last.to_ascii_lowercase();
    } else {
        return;
    }
    en.set_text(&user);
}

generate_page!(Computername {
    hostname_field_modified: bool,
    hostname_field_controlled: bool,
    computername: gtk::Entry,
    hostname: gtk::Entry,
    error: gtk::Label,
    next: libhelium::Button,
}:
    init[error hostname](root, sender, model, widgets) {
        model.next = widgets.prev_next_btns.next.clone();
    }

    update(self, message, sender) {
        NotifyComputername(name: String) => {
            if !self.hostname_field_modified {
                self.hostname_field_controlled = true;
                autoset_hostname(&self.hostname, &name);
            }
            let mut settings = SETTINGS.write();
            settings.computername = if name.is_empty() {
                settings.hostname.clone()
            } else {
                name
            };
        },
        NotifyHostname(name: String) => {
            if self.hostname_field_controlled {
                self.hostname_field_controlled = false;
            } else {
                self.hostname_field_modified = true;
            }
            let mut settings = SETTINGS.write();
            settings.hostname = name.clone();
            if settings.computername.is_empty() {
                settings.computername = name.clone();
            }
            self.error.set_visible(false);
            self.next.set_sensitive(true);
        },
        InvalidHostname => {
            self.error.set_visible(true);
            self.next.set_sensitive(false);
        }
    } => {
    }

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_margin_horizontal: 80,
        set_vexpand: true,
        set_hexpand: true,
        set_valign: gtk::Align::Center,
        set_halign: gtk::Align::Fill,

        gtk::Image {
            set_icon_name: Some("computer-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            #[watch]
            set_label: &t!("page-computername"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        #[name = "computername"]
        gtk::Entry {
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            #[watch]
            set_placeholder_text: Some(&t!("page-computername-computername")),
            connect_changed[sender] => move |e| sender.input(Self::Input::NotifyComputername(e.text().to_string())),
        },

        #[local_ref] hostname ->
        gtk::Entry {
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            #[watch]
            set_placeholder_text: Some(&t!("page-computername-hostname")),
            connect_changed[sender] => move |e| if valid_entry(e.text().as_str()) {
                sender.input(Self::Input::NotifyHostname(e.text().to_string()))
            } else {
                sender.input(Self::Input::InvalidHostname)
            },
            connect_activate[sender] => move |e| if valid_entry(e.text().as_str()) { sender.output(Self::Output::Nav(NavAction::Next)).unwrap() },
        },

        #[local_ref] error ->
        gtk::Label {
            #[watch]
            set_label: &t!("page-computername-error"),
            set_use_markup: true,
            set_visible: false,
            add_css_class: "error",
        },
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
