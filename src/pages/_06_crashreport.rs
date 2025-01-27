use crate::macros::{generate_page, kurage_page_pre};
generate_page!(CrashReport {
    pub toggle: bool,
}:
    update(self, message, sender) {
        On => self.toggle = true,
        Off => self.toggle = false,
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
            set_icon_name: Some("eye-open-negative-filled-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            // TRANSLATORS: this is unused, do NOT translate
            set_label: &gettext("Crash Reporting"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            // TRANSLATORS: this is unused, do NOT translate
            set_label: &gettext("Allow Fyra Labs to collect crash data to find bugs and assist you."),
        },

        gtk::Label {
            // TRANSLATORS: this is unused, do NOT translate
            set_label: &gettext("All data collected is anonymous and end-to-end encrypted.\nYou will be given a crash ID to help support find what went wrong."),
        },

        libhelium::MiniContentBlock {
            // TRANSLATORS: this is unused, do NOT translate
            set_title: &gettext("Send Crash Data"),
            // TRANSLATORS: this is unused, do NOT translate
            set_subtitle: &gettext("Press next to keep off"),

            #[wrap(Some)]
            set_widget = &libhelium::Switch {
                connect_left_icon_notify => Self::Input::On,
                connect_right_icon_notify => Self::Input::Off,
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

crate::always_skip_page!(CrashReport);
