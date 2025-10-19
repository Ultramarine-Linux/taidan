use crate::{backend::tweaks::TWEAKS, prelude::*};

fn page_skipconfig() -> bool {
    // https://github.com/Ultramarine-Linux/taidan/issues/81
    true
    // TWEAKS.is_empty()
}

struct TweaksFactory(FactoryVecDeque<TweakBox>);

impl Default for TweaksFactory {
    fn default() -> Self {
        let mut factory = FactoryVecDeque::builder()
            .launch(gtk::FlowBox::default())
            .detach();
        (0..TWEAKS.len()).for_each(|_| _ = factory.guard().push_back(()));
        Self(factory)
    }
}
impl std::fmt::Debug for TweaksFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TweaksFactory")
            .field("len", &self.0.len())
            .finish()
    }
}

generate_page!(Tweaks {
    factory: TweaksFactory,
}:
    init[factory_widget { model.factory.0.widget() }](root, sender, model, widgets) {}
    update(self, message, sender) {} => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_margin_horizontal: 80,
        set_vexpand: true,
        set_hexpand: true,
        set_valign: gtk::Align::Center,
        set_halign: gtk::Align::Fill,

        gtk::Image {
            set_icon_name: Some("systemsettings-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &t!("page-tweaks"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },
    },

    gtk::ScrolledWindow {
        #[local_ref] factory_widget ->
        gtk::FlowBox {
            set_selection_mode: gtk::SelectionMode::None,
            set_orientation: gtk::Orientation::Horizontal,
            set_vexpand: true,
            set_homogeneous: true,
            set_valign: gtk::Align::Center,
            set_min_children_per_line: 1,
            set_max_children_per_line: 2,
            set_column_spacing: 6,
            set_row_spacing: 6,
            add_css_class: "content-flowbox",
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

#[derive(Clone, Copy, Debug)]
struct TweakBox(usize);

#[relm4::factory]
impl relm4::factory::FactoryComponent for TweakBox {
    type Init = ();
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = relm4::gtk::FlowBox;

    view! {
        // BUG: relm4 doesn't allow templates as roots?
        #[root]
        libhelium::MiniContentBlock {
            #[watch]
            set_title: &TWEAKS[self.0].name(),
            #[watch]
            set_subtitle: &TWEAKS[self.0].desc(),
            #[wrap(Some)]
            set_widget = &gtk::Switch {
                set_halign: gtk::Align::End,
                set_hexpand: true,
                connect_state_set[idx = self.0] => move |_, state| {
                    crate::SETTINGS.write().tweaks[idx] = state;
                    glib::Propagation::Proceed
                },
            }
        }
    }

    fn init_model(
        (): Self::Init,
        index: &relm4::factory::DynamicIndex,
        sender: relm4::FactorySender<Self>,
    ) -> Self {
        SETTINGS.subscribe(sender.input_sender(), |_| ());
        Self(index.current_index())
    }
}
