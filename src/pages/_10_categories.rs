#![allow(clippy::significant_drop_tightening, clippy::indexing_slicing)]
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
                        self.categories.as_ref().unwrap()[index].category.clone()
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
    category: String,
    rows: Vec<relm4::Controller<CatRow>>,
    optlist: gtk::ListBox,
    last_select_row: Option<usize>,
}:
    init[optlist](_root, sender, model, widgets) for init: String {
        let category = (CFG.catalogue.iter())
            .find(|category| category.name == init)
            .expect("no such category");
        model.rows = (category.choices.iter().cloned().enumerate())
            .map(|(index, choice)| {
                CatRow::builder()
                    .launch(CatRow {
                        index,
                        choice,
                        category: init.clone(),
                    })
                    .forward(sender.input_sender(), Self::Input::RowSel)
            })
            .collect();
        model.rows.iter()
            .for_each(|x| widgets.viewdual.browsers.append(x.widget()));
        model.category = init;
    }

    update(self, message, _sender) {
        RowSel(index: usize) => {
            if self.last_select_row.is_some_and(|i| i == index) {
                // deselect this
                self.last_select_row = None;
                SETTINGS.write().catalogue.get_mut(&self.category).unwrap().remove(&index);
                self.optlist.remove_all();
                return;
            }
            self.last_select_row = Some(index);
            self.optlist.remove_all();
            let row = self.rows.get(index).expect("row not exist called window");
            let mut sett = SETTINGS.write();
            let ctlg = &mut sett.catalogue;
            if let Some(apps) = ctlg.get_mut(&self.category) {
                if let Some(opts) = apps.get(&index) {
                    row.model().populate_optlist(&self.optlist, &self.category, index, &opts.iter().copied());
                } else {
                    apps.insert(index, vec![0;CFG.catalogue.iter().find(|c| c.name == self.category).expect("can't find category").choices[index].options.len()]);
                    row.model().populate_optlist(&self.optlist, &self.category,index, &std::iter::empty());
                }
            } else {
                let mut map = std::collections::HashMap::new();
                map.insert(index, vec![0;CFG.catalogue.iter().find(|c| c.name == self.category).expect("can't find category").choices[index].options.len()]);
                ctlg.insert(self.category.clone(), map);
                row.model().populate_optlist(&self.optlist, &self.category, index, &std::iter::empty());
            }
        }
    } => {}

    libhelium::Window {
        set_default_width: 500,
        set_default_height: 450,
        set_vexpand: true,
        set_hexpand: true,
        set_align: gtk::Align::Fill,

        #[wrap(Some)]
        set_child = &gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            // BUG: the close button destroys the window.
            // If the window is destroyed, the window cannot be shown properly next time.
            libhelium::AppBar {},

            gtk::Label {
                set_label: &gettext(&init),
                add_css_class: "view-subtitle",
                inline_css: "font-weight: bold",
                set_margin_bottom: 20,
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
                    },
                },
            },
        },
    }
);

crate::generate_component!(CatRow {
    index: usize,
    choice: crate::cfg::Choice,
    category: String,
}:
    init(root, sender, model, widgets) for init: Self {
        let index = init.index;
        let ctl = gtk::GestureClick::new();
        ctl.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
        ctl.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            sender.output(index).unwrap();
        });
        root.add_controller(ctl);
        model = init;
    }
    // NOTE: output `usize` index from `ctl` in `init()`
    update(self, message, _sender) { } => usize

    libhelium::MiniContentBlock {
        set_hexpand: true,
        set_title: &init.choice.name,
        set_subtitle: &init.choice.description,
        set_icon: &format!("ctlg-{}-{}", init.category, init.choice.name.to_ascii_lowercase()),

    },
);

impl CatRow {
    /// # Panics
    /// this assumes there are at least 1 element in each checkbox list
    fn populate_optlist<I: Iterator<Item = usize> + Clone>(
        &self,
        list: &gtk::ListBox,
        cat: &str,
        cat_index: usize,
        optlist: &I,
    ) {
        self.choice.options.iter().enumerate().for_each(|(i, opt)| {
            let inneroptlist = gtk::Box::new(gtk::Orientation::Vertical, 8);
            let mut iter = optlist.clone().skip(i);
            match opt {
                crate::cfg::ChoiceOption::Checkbox(lbl) => inneroptlist.append(&{
                    let btn = gtk::CheckButton::builder()
                        .label(lbl)
                        .active(iter.next().is_some_and(|i| i == 1))
                        .build();
                    btn.connect_toggled(on_choice_toggled(cat, cat_index, i, 1));
                    btn
                }),
                crate::cfg::ChoiceOption::Radio(list) => {
                    let active_index = iter.next().expect("no such option");
                    let btnlist = (list.iter().enumerate())
                        .map(|(k, s)| {
                            let btn = gtk::CheckButton::builder()
                                .label(s)
                                .active(k == active_index)
                                .build();
                            btn.connect_toggled(on_choice_toggled(cat, cat_index, i, k));
                            btn
                        })
                        .collect_vec();
                    let firstbtn = btnlist.first().expect("No first checkbox?");
                    (btnlist.iter().skip(1)).for_each(|btn| btn.set_group(Some(firstbtn)));
                    btnlist.iter().for_each(|btn| inneroptlist.append(btn));
                }
            }
            list.append(&inneroptlist);
        });
    }
}

#[allow(clippy::missing_panics_doc)]
fn on_choice_toggled(
    cat: &str,
    cat_index: usize,
    i: usize,
    k: usize,
) -> impl Fn(&gtk::CheckButton) {
    let cat = cat.to_owned();
    move |b: &gtk::CheckButton| {
        if b.is_active() {
            SETTINGS
                .write()
                .catalogue
                .get_mut(&cat)
                .unwrap()
                .get_mut(&cat_index)
                .unwrap()[i] = k;
        } else if b.css_name().as_str() != "radio" {
            SETTINGS
                .write()
                .catalogue
                .get_mut(&cat)
                .unwrap()
                .get_mut(&cat_index)
                .unwrap()[i] = 0;
        }
    }
}
