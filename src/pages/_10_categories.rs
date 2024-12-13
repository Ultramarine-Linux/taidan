use relm4::RelmRemoveAllExt;
use std::rc::Rc;

crate::generate_page!(Categories {
    categories: Option<FactoryVecDeque<CategoryBtn>>,
    windows: Vec<Option<Rc<Controller<CategoryWindow>>>>,
}:
    init(root, sender, model, widgets) {
        let mut catfactory: FactoryVecDeque<CategoryBtn>
            = FactoryVecDeque::builder().launch(widgets.flowbox.clone()).forward(sender.input_sender(), CategoriesPageMsg::BtnClick);
        let mut catf = catfactory.guard();
        CFG.catalogue.iter().filter(|cat| cat.name != "Browser")
            .for_each(|cat| _ = catf.push_back(CategoryBtn {
                category: cat.name.clone(),
            }));
        model.windows = vec![None; catf.len()];
        drop(catf);
        model.categories = Some(catfactory);
    }
    update(self, message, sender) {
        BtnClick(index: usize) => {
            // on btn click, open new CategoryWindow
            let ctl = if let Some(ctl) = &self.windows[index] {
                Rc::clone(ctl)
            } else {
                // init new window
                let dialog = CategoryWindow::builder()
                    .launch(
                        self.categories.as_ref().unwrap().get(index).unwrap().category.clone()
                    )
                    .detach();
                let rc = Rc::new(dialog);
                self.windows[index] = Some(Rc::clone(&rc));
                rc
            };
            ctl.widget().present();
            ctl.widget().set_visible(true);
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
            remove_css_class: "suggested-action",
            add_css_class: "destructive-action",
            connect_clicked => Self::Input::Nav(NavAction::Next),
        },
    }
);

#[derive(Debug, Default)]
struct CategoryBtn {
    category: String,
}
#[relm4::factory]
impl FactoryComponent for CategoryBtn {
    type Init = Self;
    type Input = ();
    type Output = usize;
    type CommandOutput = ();
    type ParentWidget = gtk::FlowBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            add_controller: ctl,

            libhelium::ContentBlockImage {
                // FIXME: image doesn't display, probably libhelium bug
                set_file: &format!("ctlg-{}", self.category),
                set_requested_height: 100,
                set_requested_width: 100*1920/1080,
            },
            gtk::Label {
                set_label: &*gettext(&self.category),
            },
        },
    }

    fn init_model(
        init: Self::Init,
        _index: &relm4::factory::DynamicIndex,
        _sender: relm4::FactorySender<Self>,
    ) -> Self {
        init
    }

    fn init_widgets(
        &mut self,
        index: &Self::Index,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let i = index.current_index();
        let ctl = gtk::GestureClick::new();
        ctl.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
        ctl.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            sender.output(i).unwrap();
        });
        let widgets = view_output!();
        widgets
    }
}

crate::generate_component!(CategoryWindow {
    rows: Vec<relm4::Controller<CatRow>>,
    optlist: gtk::ListBox,
}:
    init[optlist](_root, sender, model, widgets) for init: String {
        let category = (CFG.catalogue.iter())
            .find(|category| category.name == init)
            .expect("No browser category");
        model.rows = (category.choices.iter().cloned().enumerate())
            .map(|(index, choice)| {
                CatRow::builder()
                    .launch(CatRow {
                        index,
                        choice,
                        category: init.clone(),
                    })
                    .forward(sender.input_sender(), Self::Input::BrowserRowSel)
            })
            .collect();
        model
            .rows
            .iter()
            .for_each(|x| widgets.viewdual.browsers.append(x.widget()));
    }

    update(self, message, _sender) {
        BrowserRowSel(index: usize) => {
            self.optlist.remove_all();
            let row = (self.rows.get(index)).expect("browser row not exist called browser page");
            row.model().populate_optlist(&self.optlist);
        }
    } => {}

    libhelium::Window {
        #[wrap(Some)]
        set_child = &gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            libhelium::AppBar {},
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
                    },
                },
            },
        }
    }
);

crate::generate_component!(CatRow {
    index: usize,
    choice: crate::cfg::Choice,
    category: String,
}:
    init(root, sender, model, widgets) for init: Self {
        model = init;
        let index = model.index;
        let ctl = gtk::GestureClick::new();
        ctl.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
        ctl.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            sender.output(index).unwrap();
        });
        root.add_controller(ctl);
    }
    // NOTE: output `usize` index from `ctl` in `init()`
    update(self, message, _sender) { } => usize

    libhelium::MiniContentBlock {
        set_hexpand: true,
        set_title: &model.choice.name,
        set_subtitle: &model.choice.description,
        set_icon: &format!("ctlg-{}-{}", model.category, model.choice.name.to_ascii_lowercase()),

    },
);

impl CatRow {
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
