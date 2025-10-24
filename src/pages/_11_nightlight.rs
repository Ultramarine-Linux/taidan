use crate::prelude::*;

fn page_skipconfig() -> bool {
    SETTINGS.read().skipconfig || CFG.edition == "xfce"
}

generate_page!(NightLight:
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
            set_icon_name: Some("display-brightness-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            #[watch]
            set_label: &t!("page-nightlight"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_use_markup: true,
            #[watch]
            set_label: &t!("page-nightlight-desc"),
            set_justify: gtk::Justification::Center,
        },

        #[template] crate::ui::SwitchBox {
            #[watch]
            set_title: &t!("switch-nightlight"),
            #[template_child] switch {
                connect_state_set[sender] => move |_, state| {
                    SETTINGS.write().nightlight = state;
                    sender.oneshot_command(async move { crate::backend::theme::set_night_light(None, state).await.unwrap()});
                    glib::Propagation::Proceed
                },
            }
        },
    },

    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            #[watch]
            set_label: &t!("prev"),
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            #[watch]
            set_label: &t!("page-categories-confirm"),
            remove_css_class: "suggested-action",
            add_css_class: "destructive-action",
            connect_clicked => Self::Input::Nav(NavAction::GoTo(crate::Page::Installing)),
        },
    }
);
