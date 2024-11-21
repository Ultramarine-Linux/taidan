use crate::prelude::*;

#[relm4::widget_template(pub)]
impl WidgetTemplate for PrevNextBtns {
    view! {
        gtk::Box {
            set_valign: gtk::Align::End,

            #[name = "prev"]
            libhelium::Button {
                set_is_pill: true,
                #[watch]
                set_label: &gettext("Previous"),
                inline_css: "padding-left: 48px; padding-right: 48px",
            },

            gtk::Box { set_hexpand: true },

            #[name = "next"]
            libhelium::Button {
                set_is_pill: true,
                #[watch]
                set_label: &gettext("Next"),
                inline_css: "padding-left: 48px; padding-right: 48px",
                add_css_class: "suggested-action",
            },
        },
    }
}

#[relm4::widget_template(pub)]
impl WidgetTemplate for SwitchBox {
    view! {
        libhelium::MiniContentBlock {
            #[wrap(Some)]
            #[name(switch)]
            set_widget = &gtk::Switch {}
        }
    }
}

#[relm4::widget_template(pub)]
impl WidgetTemplate for Category {
    view! {
        #[name(viewdual)]
        libhelium::ViewDual {
            set_valign: gtk::Align::Fill,
            set_halign: gtk::Align::Fill,
            set_vexpand: true,
            set_hexpand: true,
            set_show_handle: false,

            #[name(browsers)]
            #[wrap(Some)]
            set_child_start = &libhelium::ContentList {
                set_hexpand: true,
            },
            #[name(optlist)]
            #[wrap(Some)]
            set_child_end = &libhelium::ContentBlock {
                set_hexpand: true,
            },
        },
    }
}
