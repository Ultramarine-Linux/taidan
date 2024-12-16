use crate::backend::steps::Stage;

crate::generate_page!(Installing {
    main_progress_bar: gtk::ProgressBar,
    sub_progress_bar: gtk::ProgressBar,
    stage: Stage,
    throb_timeout: Option<glib::SourceId>,
}:
    init[main_progress_bar sub_progress_bar](root, sender, model, widgets) {
        model.throb_timeout = Some(gtk::glib::timeout_add(std::time::Duration::from_secs(1), move || {
            sender.input(Self::Input::Throb);
            gtk::glib::ControlFlow::Continue
        })); // TODO: cleanup
    }
    update(self, message, sender) {
        // handle UI updates here.
        // NOTE: main.rs should call the start_install() fns.
        UpdStage(stage: Stage) => {
            let stage_num = u8::from(stage);
            #[allow(clippy::cast_precision_loss)]
            self.main_progress_bar.set_fraction(f64::from(stage_num) / crate::backend::steps::NUM_STAGES as f64);
            let text = format!("[{stage_num}/{}] {}", crate::backend::steps::NUM_STAGES, String::from(stage));
            self.stage = stage;
            self.main_progress_bar.set_text(Some(&text));
        },
        Finish => sender.output(Self::Output::Nav(NavAction::Next)).unwrap(),
        Throb => {
            if self.stage.is_dnf() && self.sub_progress_bar.fraction() != 0.0 {
                self.throb_timeout.take().expect("throbbed by nonexistent glib::SourceId").remove();
            } else {
                self.sub_progress_bar.pulse();
            }
        },
        UpdSubProg(frac: f64) => self.sub_progress_bar.set_fraction(frac),
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

    #[local_ref] sub_progress_bar ->
    gtk::ProgressBar { },
);
