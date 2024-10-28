use crate::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct WelcomePage;

#[derive(Debug)]
pub enum WelcomePageMsg {
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
                set_vexpand: true,

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

                    gtk::Label {
                        set_label: &gettext("Welcome to %s").replace("%s", &CONFIG.read().distro),
                        add_css_class: "view-title",
                        inline_css: "font-weight: bold",
                    },

                    gtk::Label {
                        set_label: &gettext("Let's get your system ready."),
                        inline_css: "font-size: 1.25rem",
                    },
                },

                gtk::Box {
                    set_valign: gtk::Align::End,
                    set_halign: gtk::Align::Center,
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Horizontal,

                    libhelium::Button {
                        set_is_pill: true,
                        set_valign: gtk::Align::End,
                        set_halign: gtk::Align::Center,
                        set_label: &gettext("Let's Go"),
                        inline_css: "padding-left: 48px; padding-right: 48px",
                        add_css_class: "suggested-action",
                        connect_clicked => Self::Input::Nav(NavAction::Next),
                    },
                }
            },
        },
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {};
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            Self::Input::Nav(action) => {
                sender.output(Self::Output::Nav(action)).unwrap();
            }
        }
    }
}
