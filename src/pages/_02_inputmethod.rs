crate::generate_page!(InputMethod:
    init(root, sender, model, widgets) {}
    update(self, message, sender) {} => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_margin_bottom: 16,
        set_valign: gtk::Align::Fill,
        set_halign: gtk::Align::Fill,

        gtk::Image {
            set_icon_name: Some("input-keyboard-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &gettext("Input Method"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },
    },

    #[name(prev_next_btns)]
    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            connect_clicked => Self::Input::Nav(NavAction::Next),
        },
    }
);
