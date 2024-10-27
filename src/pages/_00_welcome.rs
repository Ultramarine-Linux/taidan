use crate::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct WelcomePage {
    lbl0: gtk::Label,
    lbl1: gtk::Label,
}

#[derive(Debug)]
pub enum WelcomePageMsg {
    #[doc(hidden)]
    Nav(NavAction),
}

#[derive(Debug)]
pub enum WelcomePageOutput {
    Nav(NavAction),
}

#[relm4::component(pub)]
impl SimpleComponent for WelcomePage {
    type Init = ();
    type Input = WelcomePageMsg;
    type Output = WelcomePageOutput;

    view! {
        libhelium::ViewMono {
            append = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 4,
                set_margin_all: 16,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 16,
                    set_vexpand: true,
                    set_valign: gtk::Align::Center,
                    set_halign: gtk::Align::Center,

                    gtk::Image {
                        set_icon_name: Some(&CONFIG.read().icon_name),
                        inline_css: "-gtk-icon-size: 128px",
                    },

                    #[local_ref]
                    lbl0 -> gtk::Label {
                        #[watch]
                        set_label: &gettext("Welcome to %s").replace("%s", &CONFIG.read().distro),
                        inline_css: "font-weight: bold; font-size: 1.75rem",
                    },

                    #[local_ref]
                    lbl1 -> gtk::Label {
                        #[watch]
                        set_label: &gettext("Let's get your system ready."),
                        set_justify: gtk::Justification::Center,
                        set_max_width_chars: 60,
                        set_wrap: true,
                    },
                },

                libhelium::Button {
                    set_is_pill: true,
                    #[watch]
                    set_label: &gettext("Install"),
                    inline_css: "padding-left: 48px; padding-right: 48px",
                    add_css_class: "suggested-action",
                    connect_clicked => WelcomePageMsg::Nav(NavAction::Next),
                },
            },
        },
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let lbl0 = gtk::Label::new(None);
        let lbl1 = gtk::Label::new(None);
        let model = Self { lbl0, lbl1 };
        let lbl0 = &model.lbl0;
        let lbl1 = &model.lbl1;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            WelcomePageMsg::Nav(action) => {
                sender.output(WelcomePageOutput::Nav(action)).unwrap();
            }
        }
    }
}
