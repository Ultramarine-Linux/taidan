use crate::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct WhoAreYouPage;

#[derive(Debug)]
pub enum WhoAreYouPageMsg {
    Nav(NavAction),
    NotifyFullName(String),
    NotifyUsername(String),
}

#[derive(Debug)]
pub enum WhoAreYouPageOutput {
    Nav(NavAction),
}

#[relm4::component(pub)]
impl SimpleComponent for WhoAreYouPage {
    type Init = ();
    type Input = WhoAreYouPageMsg;
    type Output = WhoAreYouPageOutput;

    view! {
        libhelium::ViewMono {
            append = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 4,
                set_margin_all: 16,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 16,
                    set_margin_horizontal: 128,
                    set_vexpand: true,
                    set_hexpand: true,
                    set_valign: gtk::Align::Center,
                    set_halign: gtk::Align::Fill,

                    gtk::Image {
                        set_icon_name: Some("meeting-attending"),
                        inline_css: "-gtk-icon-size: 64px",
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &gettext("Who are You?"),
                        add_css_class: "view-subtitle",
                        inline_css: "font-weight: bold",
                    },

                    libhelium::TextField {
                        set_hexpand: true,
                        set_halign: gtk::Align::Fill,
                        set_support_text: Some(&gettext("Full Name")),
                        set_is_outline: true,
                        set_needs_validation: true,

                        connect_is_valid_notify[sender] => move |tf| sender.input(Self::Input::NotifyFullName(tf.internal_entry().text().to_string())),
                    },

                    libhelium::TextField {
                        set_hexpand: true,
                        set_halign: gtk::Align::Fill,
                        set_support_text: Some(&gettext("Username")),
                        set_is_outline: true,
                        set_needs_validation: true,
                        set_regex: &libhelium::glib::Regex::new(r"^[a-z][-a-z0-9_]*\$?$", gtk::glib::RegexCompileFlags::DEFAULT, gtk::glib::RegexMatchFlags::DEFAULT).unwrap().unwrap(),

                        connect_is_valid_notify[sender] => move |tf| sender.input(Self::Input::NotifyUsername(tf.internal_entry().text().to_string())),
                    },
                },

                gtk::Box {
                    set_valign: gtk::Align::End,

                    libhelium::Button {
                        set_is_pill: true,
                        #[watch]
                        set_label: &gettext("Previous"),
                        inline_css: "padding-left: 48px; padding-right: 48px",
                        connect_clicked => Self::Input::Nav(NavAction::Back),
                    },

                    gtk::Box { set_hexpand: true },

                    libhelium::Button {
                        set_is_pill: true,
                        #[watch]
                        set_label: &gettext("Next"),
                        inline_css: "padding-left: 48px; padding-right: 48px",
                        add_css_class: "suggested-action",
                        connect_clicked => Self::Input::Nav(NavAction::Next),
                    },
                },
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
            WhoAreYouPageMsg::NotifyFullName(name) => todo!(),
            WhoAreYouPageMsg::NotifyUsername(user) => todo!(),
        }
    }
}
