crate::generate_page!(Browser {
    browser_rows: Vec<relm4::Controller<BrowserRow>>,
}:
    init(root, sender, model, widgets) {
        let cfg = crate::CONFIG.read();
        let browser_category = cfg.catalogue.iter()
            .find(|category| category.name == "Browser")
            .expect("No browser category");
        model.browser_rows = browser_category.choices.iter().cloned().enumerate()
            .map(|(index, choice)| {
                BrowserRow::builder()
                    .launch(BrowserRow { index, choice })
                    .forward(sender.input_sender(), Self::Input::BrowserRowSel)
            })
            .collect();
        model.browser_rows.iter().for_each(|x| widgets.browsers.add(x.widget()));
        drop(cfg);
    }
    update(self, message, sender) {
        BrowserRowSel(index: usize) => {
            todo!()
        },
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
            set_icon_name: Some("web-browser-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &gettext("Browser Selection"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },
    },

    libhelium::ViewDual {
        set_valign: gtk::Align::Fill,
        set_halign: gtk::Align::Fill,
        set_vexpand: true,
        set_hexpand: true,

        #[name(browsers)]
        #[wrap(Some)]
        set_child_start = &libhelium::ContentList {
            set_hexpand: true,
        },
        #[wrap(Some)]
        set_child_end = &libhelium::ContentBlock {
            set_hexpand: true,
        },
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

#[derive(Clone, Debug)]
struct BrowserRow {
    index: usize,
    choice: crate::cfg::Choice,
}
#[relm4::component]
impl SimpleComponent for BrowserRow {
    type Input = ();
    type Output = usize; // denotes when clicked
    type Init = Self;

    view! {
        libhelium::MiniContentBlock {
            set_hexpand: true,
            set_title: &model.choice.name,
            set_subtitle: &model.choice.description,
            set_icon: &format!("ctlg-browser-{}", model.choice.name.to_ascii_lowercase()),
        },
    }

    fn init(
        model: Self::Init,
        root: Self::Root,
        sender: relm4::prelude::ComponentSender<Self>,
    ) -> relm4::prelude::ComponentParts<Self> {
        let index = model.index;
        let ctl = gtk::GestureClick::new();
        ctl.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
        let s0 = sender.clone();
        ctl.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            s0.output(index).unwrap();
        });
        root.add_controller(ctl);
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: relm4::prelude::ComponentSender<Self>) {
        todo!();
    }
}
