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
                // TRANSLATORS: this means "Previous page"
                set_label: &gettext("Previous"),
                inline_css: "padding-left: 48px; padding-right: 48px",
            },

            gtk::Box { set_hexpand: true },

            #[name = "next"]
            libhelium::Button {
                set_is_pill: true,
                #[watch]
                // TRANSLATORS: this means "Next page"
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
        // libhelium::ViewDual
        #[name(viewdual)]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_valign: gtk::Align::Fill,
            set_halign: gtk::Align::Fill,
            set_vexpand: true,
            set_hexpand: true,
            // set_show_handle: false,

            // #[wrap(Some)]
            // set_child_start = &gtk::ScrolledWindow {
            gtk::ScrolledWindow {
                #[name(browsers)]
                gtk::ListBox {
                    add_css_class: "content-list",
                    set_selection_mode: gtk::SelectionMode::Multiple,
                    set_vexpand: true,
                    set_hexpand: true,
                    set_valign: gtk::Align::Fill,
                    set_halign: gtk::Align::Fill,
                }
            },
            #[name(optlist)]
            // #[wrap(Some)]
            // set_child_end = &gtk::ScrolledWindow {
            gtk::ScrolledWindow {
                // #[name(optlist)]
                // gtk::ListBox {
                //     add_css_class: "content-list",
                //     set_selection_mode: gtk::SelectionMode::Single,
                //     set_vexpand: true,
                //     set_hexpand: true,
                //     set_valign: gtk::Align::Center,
                //     set_halign: gtk::Align::Center,
                // },
            }
        }
    }
}
