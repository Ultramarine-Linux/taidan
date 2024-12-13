use relm4::RelmRemoveAllExt;

crate::generate_page!(Browser {
    browser_rows: Vec<relm4::Controller<BrowserRow>>,
    optlist: gtk::ListBox,
}:
    init[optlist](root, sender, model, widgets) {
        let browser_category = CFG.catalogue.iter()
            .find(|category| category.name == "Browser")
            .expect("No browser category");
        model.browser_rows = browser_category.choices.iter().cloned().enumerate()
            .map(|(index, choice)| {
                BrowserRow::builder()
                    .launch(BrowserRow { index, choice })
                    .forward(sender.input_sender(), Self::Input::BrowserRowSel)
            })
            .collect();
        model.browser_rows.iter().for_each(|x| widgets.viewdual.browsers.append(x.widget()));
    }
    update(self, message, sender) {
        BrowserRowSel(index: usize) => {
            self.optlist.remove_all();
            let row = (self.browser_rows.get(index))
                .expect("browser row not exist called browser page");
            row.model().populate_optlist(&self.optlist);
        },
    } => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        // set_spacing: 16,
        // set_margin_horizontal: 80,
        // set_vexpand: true,
        set_margin_bottom: 16,
        set_hexpand: true,
        set_valign: gtk::Align::Fill,
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

    #[name(viewdual)] #[template] crate::ui::Category {
        #[template_child] optlist {
            #[local_ref] optlist ->
            gtk::ListBox {
                add_css_class: "content-list",
                set_selection_mode: gtk::SelectionMode::Single,
                set_vexpand: true,
                set_hexpand: true,
                set_valign: gtk::Align::Center,
                set_halign: gtk::Align::Center,
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

            add_controller: ctl,
        },
    }

    #[allow(clippy::renamed_function_params)]
    fn init(
        model: Self::Init,
        root: Self::Root,
        sender: relm4::prelude::ComponentSender<Self>,
    ) -> relm4::prelude::ComponentParts<Self> {
        let index = model.index;
        let ctl = gtk::GestureClick::new();
        ctl.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
        ctl.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            sender.output(index).unwrap();
        });
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    // fn update(&mut self, message: Self::Input, sender: relm4::prelude::ComponentSender<Self>) {}
}

impl BrowserRow {
    /// # Panics
    /// this assumes there are at least 1 element in each checkbox list
    fn populate_optlist(&self, list: &gtk::ListBox) {
        self.choice.options.iter().for_each(|opt| {
            let inneroptlist = gtk::Box::new(gtk::Orientation::Vertical, 8);
            match opt {
                crate::cfg::ChoiceOption::Checkbox(list) => list
                    .iter()
                    .map(|s| gtk::CheckButton::builder().label(s).build())
                    .for_each(|btn| inneroptlist.append(&btn)),
                crate::cfg::ChoiceOption::Radio(list) => {
                    let btnlist = list
                        .iter()
                        .map(|s| gtk::CheckButton::builder().label(s).build())
                        .collect_vec();
                    let firstbtn = btnlist.first().expect("No first checkbox?");
                    btnlist
                        .iter()
                        .skip(1)
                        .for_each(|btn| btn.set_group(Some(firstbtn)));
                    btnlist.iter().for_each(|btn| inneroptlist.append(btn));
                }
            }
            list.append(&inneroptlist);
        });
    }
}
