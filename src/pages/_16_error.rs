crate::generate_page!(Error {
    buf: gtk::TextBuffer,
}:
    init(root, sender, model, widgets) {}
    update(self, message, sender) {
        Receive(msg: String) => self.buf.set_text(&msg),
    } => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_vexpand: true,
        set_valign: gtk::Align::Center,
        set_halign: gtk::Align::Center,

        gtk::Image {
            set_icon_name: Some("dialog-error-symbolic"),
            inline_css: "-gtk-icon-size: 128px",
        },

        gtk::Label {
            set_label: &gettext("Error"),
            add_css_class: "view-title",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_label: &gettext("We are sorry, but there is an unexpected problem."),
            inline_css: "font-size: 1.25rem",
        },
    },

    gtk::TextView {
        set_vexpand: true,
        set_hexpand: true,
        set_monospace: true,
        set_buffer: Some(&model.buf),
    },


    libhelium::Button {
        set_is_pill: true,
        set_halign: gtk::Align::End,
        set_label: &gettext("Done"),
        inline_css: "padding-left: 48px; padding-right: 48px",
        connect_clicked => Self::Input::Nav(NavAction::Quit),
    },
    gtk::Box {
        set_valign: gtk::Align::End,
        set_halign: gtk::Align::Fill,
        set_hexpand: true,
        set_orientation: gtk::Orientation::Horizontal,

        libhelium::Button {
            set_is_textual: true,
            set_halign: gtk::Align::Start,
            set_label: &gettext("Retry"),
            connect_clicked[sender] => move |_| {
                sender.input(Self::Input::Nav(NavAction::GoTo(crate::Page::Categories)));
                sender.input(Self::Input::Nav(NavAction::Next));
            },
        },

        gtk::Box { set_halign: gtk::Align::Fill, set_hexpand: true },

        libhelium::Button {
            set_is_pill: true,
            set_halign: gtk::Align::End,
            set_label: &gettext("Let's Go"),
            inline_css: "padding-left: 48px; padding-right: 48px",
            add_css_class: "suggested-action",
            connect_clicked[sender] => move |_| {
                SETTINGS.write().skipconfig = false;
                sender.input(Self::Input::Nav(NavAction::Next));
            },
        },
    },
);
