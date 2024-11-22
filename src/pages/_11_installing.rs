crate::generate_page!(Installing {
    main_progress_bar: gtk::ProgressBar,
}:
    init[main_progress_bar](root, sender, model, widgets) {
    }
    update(self, message, sender) {
        // handle UI updates here.
        // NOTE: main.rs should call the start_install() fns.
        UpdLbl(stage: crate::backend::Stage) => {
            self.main_progress_bar.set_text(Some(&*String::from(stage)));
        },
        Finish => sender.output(Self::Output::Nav(NavAction::Next)).unwrap(),
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
            set_icon_name: Some("distro-icon-symbolic"),
            inline_css: "-gtk-icon-size: 128px",
        },

        gtk::Label {
            set_label: &gettext("Installing your Apps"),
            add_css_class: "view-title",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_label: &gettext("This won't take long!"),
            inline_css: "font-size: 1.25rem",
        },
    },

    // FIXME: libhelium::ProgressBar
    #[local_ref] main_progress_bar ->
    gtk::ProgressBar {
        set_text: Some(&*gettext("Loading…")),
    },

    gtk::ProgressBar {

    },
);
