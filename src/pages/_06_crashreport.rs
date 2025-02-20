use crate::prelude::*;
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
            set_label: &t!("page-crashreport"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            // TRANSLATORS: this is unused, do NOT translate
            set_label: &t!("page-crashreport", "desc", org = "Fyra Labs"),
        },

        gtk::Label {
            // TRANSLATORS: this is unused, do NOT translate
            set_label: &format!("{}\n{}", t!("page-crashreport", "lbl1"), t!("page-crashreport", "lbl2")),
        },

        libhelium::MiniContentBlock {
            // TRANSLATORS: this is unused, do NOT translate
            set_title: &t!("switch-crashreport"),
            // TRANSLATORS: this is unused, do NOT translate
            set_subtitle: &t!("switch-crashreport", "desc"),

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
