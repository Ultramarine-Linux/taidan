const WIKI_POSTINSTALL: &str = "https://wiki.ultramarine-linux.org/en/setup/postinstall/";

use crate::prelude::*;
skipconfig!();
generate_page!(Codecs:
    init(root, sender, model, widgets) {
        if CFG.edition == "xfce" {
            let next = &widgets.prev_next_btns.next;
            next.set_label(&t!("page-categories-confirm"));
            next.remove_css_class("suggested-action");
            next.add_css_class("destructive-action");
        }

        // Add keyboard event handler for Enter key
        let sender_clone = sender.clone();
        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Return {
                // Navigate to next page when Enter is pressed
                if CFG.edition == "xfce" {
                    sender_clone.input(Self::Input::Nav(NavAction::GoTo(crate::Page::Installing)));
                } else {
                    sender_clone.input(Self::Input::Nav(NavAction::Next));
                }
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });
        root.add_controller(key_controller);
    }
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
            set_icon_name: Some("computer-laptop-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            #[watch]
            set_label: &t!("page-codecs"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_use_markup: true,
            set_justify: gtk::Justification::Center,
            // FIXME: someone tell me how to do this properly
            set_label: &format!("{}\n{}", t!("page-codecs-desc1"), t!("page-codecs-desc2", wiki = format!("<a href='{WIKI_POSTINSTALL}'>{}</a>", t!("page-codecs-wiki")))),
        },

        #[template] crate::ui::SwitchBox {
            #[watch]
            set_title: &t!("switch-codecs"),
            #[watch]
            set_subtitle: &t!("switch-codecs-desc"),
            #[template_child] switch {
                set_active: true,
                connect_state_set => move |_, state| {
                    SETTINGS.write().install_codecs_drivers = state;
                    glib::Propagation::Proceed
                },
            }
        },
    },

    #[name(prev_next_btns)]
    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            #[watch]
            set_label: &t!("prev"),
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            #[watch]
            set_label: &t!("next"),
            connect_clicked => if CFG.edition == "xfce" { Self::Input::Nav(NavAction::GoTo(crate::Page::Installing)) } else { Self::Input::Nav(NavAction::Next) },
        },
    }
);
