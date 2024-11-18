crate::generate_page!(Categories:
    update(self, message, sender) {

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
            set_icon_name: Some("dialog-question-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &gettext("What Do You Use This Device For?"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },
    },

    #[name(flowbox)]
    gtk::FlowBox {
        set_max_children_per_line: 3,
        set_column_spacing: 10,
        set_row_spacing: 20,
        set_selection_mode: gtk::SelectionMode::Multiple,
    },

    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            set_label: &gettext("Confirm and Setup System"),
            add_css_class: "destructive-action",
            connect_clicked => Self::Input::Nav(NavAction::Next),
        },
    }
);
