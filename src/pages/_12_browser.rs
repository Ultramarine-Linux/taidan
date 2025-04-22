#![allow(clippy::significant_drop_tightening, clippy::indexing_slicing)]
use crate::prelude::*;
use relm4::RelmRemoveAllExt;

pub(crate) const BROWSER_CATEGORY: &str = "Browsers";

generate_page!(Browser {
    browser_rows: Vec<relm4::Controller<BrowserRow>>,
    optlist: gtk::ListBox,
    btn_next: libhelium::Button,
    // img: libhelium::ContentBlockImage,
}:
    init[optlist /* img */](root, sender, model, widgets) {
        let browser_category = CFG.catalogue.iter()
            .find(|category| category.name ==BROWSER_CATEGORY)
            .expect("No browser category");
        model.browser_rows = browser_category.choices.iter().cloned().enumerate()
            .map(|(index, choice)| {
                BrowserRow::builder()
                    .launch(BrowserRow { index, choice })
                    .forward(sender.input_sender(), Self::Input::BrowserRowSel)
            })
            .collect();
        model.browser_rows.iter().for_each(|x| widgets.viewdual.browsers.append(x.widget()));
        model.btn_next = widgets.prev_next_btns.next.clone();
    }
    update(self, message, sender) {
        BrowserRowSel(index: usize) => {
            self.btn_next.set_sensitive(true);
            self.optlist.remove_all();
            let row = self.browser_rows.get(index).expect("browser row not exist called browser page");
            let mut sett = SETTINGS.write();
            let ctlg = &mut sett.catalogue;
            // let selection = row.model().choice.name.to_ascii_lowercase().replace(' ', "-");
            // self.img.set_file(&format!("resource:///com/fyralabs/Taidan/screenshots/ss-browser-{selection}.png"));
            // self.img.set_visible(true);
            if let Some(browsers) = ctlg.get_mut(BROWSER_CATEGORY) {
                // NOTE: since we only allow 1 browser choice, remove the old one
                browsers.clear();
                if let Some(opts) = browsers.get(&index) {
                    row.model().populate_optlist(&self.optlist, index, &opts.iter().copied());
                } else {
                    browsers.insert(index, vec![0;CFG.catalogue.iter().find(|c| c.name == BROWSER_CATEGORY).expect("can't find category").choices[index].options.len()]);
                    row.model().populate_optlist(&self.optlist, index, &std::iter::empty());
                }
            } else {
                let mut map = std::collections::HashMap::new();
                map.insert(index, vec![0;CFG.catalogue.iter().find(|c| c.name == BROWSER_CATEGORY).expect("can't find category").choices[index].options.len()]);
                ctlg.insert(BROWSER_CATEGORY.into(), map);
                row.model().populate_optlist(&self.optlist, index, &std::iter::empty());
            }
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
            set_label: &t!("page-browser"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },
    },

    #[name(viewdual)] #[template] crate::ui::Category {
        #[template_child] browsers {
            set_selection_mode: gtk::SelectionMode::Single,
        },
        #[template_child] optlist {
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                // BUG: gtk refuses to update the image automatically???
                // #[local_ref] img ->
                // libhelium::ContentBlockImage {
                //     set_requested_height: 150,
                //     set_requested_width: 150*1920/1080,
                //     set_valign: gtk::Align::Center,
                //     set_halign: gtk::Align::Center,
                // },

                #[local_ref] optlist ->
                gtk::ListBox {
                    add_css_class: "content-list",
                    set_vexpand: true,
                    set_hexpand: true,
                    set_valign: gtk::Align::Center,
                    set_halign: gtk::Align::Center,
                }
            },
        }
    },

    #[name(prev_next_btns)]
    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            set_sensitive: false,
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
            gesture.set_state(gtk::EventSequenceState::None);
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
    fn populate_optlist<I: Iterator<Item = usize> + Clone>(
        &self,
        list: &gtk::ListBox,
        browser_index: usize,
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
                    btn.connect_toggled(on_choice_toggled(browser_index, i, 1));
                    btn
                }),
                crate::cfg::ChoiceOption::Radio(list) => {
                    let btnlist = (list.iter().enumerate())
                        .map(|(k, s)| {
                            let btn = gtk::CheckButton::builder()
                                .label(s)
                                .active(iter.clone().contains(&k))
                                .build();
                            btn.connect_toggled(on_choice_toggled(browser_index, i, k));
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

/// # Panics
/// - if for some reason the category doesn't exist
/// - if for some reason the browser doesn't exist
#[allow(clippy::indexing_slicing)]
fn on_choice_toggled(browser_index: usize, i: usize, k: usize) -> impl Fn(&gtk::CheckButton) {
    // and I want to kill you so bad rust why do you torture me with the inability to use `hashmap[key]`
    move |b: &gtk::CheckButton| {
        if b.is_active() {
            SETTINGS
                .write()
                .catalogue
                .get_mut(BROWSER_CATEGORY)
                .unwrap()
                .get_mut(&browser_index)
                .unwrap()[i] = k;
        } else if b.css_name().as_str() != "radio" {
            SETTINGS
                .write()
                .catalogue
                .get_mut(BROWSER_CATEGORY)
                .unwrap()
                .get_mut(&browser_index)
                .unwrap()[i] = 0;
        }
    }
}
