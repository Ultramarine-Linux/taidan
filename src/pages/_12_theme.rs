#![allow(clippy::indexing_slicing)]
use crate::backend::theme;
use crate::prelude::*;

const ENABLED_ACCENT_BTN_CSS: &str = "
background: @color;
color: transparent;";

const DISABLED_ACCENT_BTN_CSS: &str = "
background: alpha(@color, 0.18);
color: transparent;
box-shadow: inset 0 0 0 3px alpha(@color, 0.32);";

skipconfig!();
generate_page!(Theme:
    init(root, sender, model, widgets) {
        let (light0, dark0) = (widgets.lightbox.clone(), widgets.darkbox.clone());
        let (light1, dark1) = (widgets.lightbox.clone(), widgets.darkbox.clone());
        let (ctl_light, ctl_dark) = (gtk::GestureClick::new(), gtk::GestureClick::new());
        let (s0, s1) = (sender.clone(), sender.clone());
        ctl_light.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
        ctl_dark.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
        let first = gtk::CheckButton::new();
        first.add_css_class("accent-mode");
        first.inline_css(&DISABLED_ACCENT_BTN_CSS.replace("@color", theme::AccentColor::all()[0].w3_color_keywords()));
        first.connect_toggled(on_accent_toggled(sender.clone(), theme::AccentColor::all()[0]));
        widgets.accentbox.append(&first);
        theme::AccentColor::all().iter().skip(1).for_each(|&color| {
            let btn = gtk::CheckButton::new();
            btn.set_group(Some(&first));
            btn.add_css_class("accent-mode");
            btn.inline_css(&DISABLED_ACCENT_BTN_CSS.replace("@color", color.w3_color_keywords()));
            btn.connect_toggled(on_accent_toggled(sender.clone(), color));
            widgets.accentbox.append(&btn);
        });
        // FIXME: WHY IS THERE NO BORDER
        let first0 = first.clone();
        ctl_light.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            SETTINGS.write().theme_is_dark = false;
            let accent = if CFG.edition == "plasma" || CFG.edition == "kde" {
                SETTINGS.write().accent = None;
                first0.set_active(true);
                first0.set_active(false);
                None
            } else {
                SETTINGS.read().accent
            };
            s0.oneshot_command(async move { theme::set_theme(None, false, accent).await.unwrap() });
            light0.inline_css("border-radius: 16px");
            dark1.inline_css("border-radius: 0px");
        });
        ctl_dark.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            SETTINGS.write().theme_is_dark = true;
            let accent = if CFG.edition == "plasma" || CFG.edition == "kde" {
                SETTINGS.write().accent = None;
                first.set_active(true);
                first.set_active(false);
                None
            } else {
                SETTINGS.read().accent
            };
            s1.oneshot_command(async move { theme::set_theme(None, true, accent).await.unwrap() });
            dark0.inline_css("border-radius: 16px");
            light1.inline_css("border-radius: 0px");
        });
        widgets.lightbox.add_controller(ctl_light);
        widgets.darkbox.add_controller(ctl_dark);
        SETTINGS.subscribe(sender.input_sender(), |_| Self::Input::Update);
    }
    update(self, message, sender) {
        Update => {},
    } => {}

    gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 16,
        set_margin_horizontal: 80,
        set_vexpand: true,
        set_hexpand: true,
        set_valign: gtk::Align::Center,
        set_halign: gtk::Align::Fill,

        gtk::Image {
            set_icon_name: Some("dialog-question-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            set_label: &t!("page-theme"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_use_markup: true,
            set_label: &t!("page-theme-desc"),
            set_justify: gtk::Justification::Center,
        },

        gtk::Label {
            set_use_markup: true,
            set_label: &t!("page-theme-note"),
            add_css_class: "caption",
        },

        gtk::Box {
            set_spacing: 32,
            set_orientation: gtk::Orientation::Horizontal,
            set_halign: gtk::Align::Center,

            #[name(lightbox)]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                libhelium::ContentBlockImage {
                    set_file: "resource:///com/fyralabs/Taidan/background-light.webp",
                    inline_css: "border-color: aqua",
                    // inline_css: "padding: unset; border-radius: 16px; background-repeat: no-repeat; background-position: center; background-size: cover",
                    // inline_css: "background-image: url(file:///usr/share/backgrounds/default.png)",
                    set_requested_height: 150,
                    set_requested_width: 150*1920/1080,
                },
                gtk::CheckButton {
                    set_label: Some(&t!("page-theme-light")),
                    #[watch]
                    set_active: !SETTINGS.read().theme_is_dark
                },
            },

            #[name(darkbox)]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                libhelium::ContentBlockImage {
                    set_file: "resource:///com/fyralabs/Taidan/background-dark.webp",
                    inline_css: "border-color: aqua",
                    // inline_css: "padding: unset; border-radius: 16px; background-repeat: no-repeat; background-position: center; background-size: cover",
                    // inline_css: "background-image: url(file:///usr/share/backgrounds/default-dark.png)",
                    set_requested_height: 150,
                    set_requested_width: 150*1920/1080,
                },

                gtk::CheckButton {
                    set_label: Some(&t!("page-theme-dark")),
                    #[watch]
                    set_active: SETTINGS.read().theme_is_dark,
                },
            },
        },
        #[name(accentbox)]
        gtk::Box {
            set_visible: ["gnome", "plasma", "kde"].contains(&&*CFG.edition),
            set_orientation: gtk::Orientation::Horizontal,
            set_halign: gtk::Align::Center,
            // ? https://github.com/tau-OS/fusebox/blob/286522b7d8f1017e8cd1379396407f29e0c83789/data/style.css#L29
            inline_css: "
                .accent-mode {
                    min-height: 48px;
                    min-width: 48px;
                    border-radius: 12px;
                    background: @surface_container_bg_color;
                }

                .accent-mode radio {
                    min-height: 42px;
                    min-width: 42px;
                    margin: 6px;
                }
                .accent-mode:checked radio,
                .accent-mode:active radio,
                .accent-mode:hover:checked radio,
                .accent-mode:active:checked radio {
                    box-shadow: 0 0 0 5px @accent_color, 0 0 0 2px @view_bg_color;
                }
            "
        }
    },

    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            set_label: &t!("page-categories-confirm"),
            remove_css_class: "suggested-action",
            add_css_class: "destructive-action",
            connect_clicked => Self::Input::Nav(NavAction::GoTo(crate::Page::Installing)),
        },
    }
);

fn on_accent_toggled(
    sender: ComponentSender<ThemePage>,
    accent: theme::AccentColor,
) -> impl Fn(&gtk::CheckButton) {
    move |b: &gtk::CheckButton| {
        if b.is_active() {
            SETTINGS.write().accent = Some(accent);
            b.inline_css(&ENABLED_ACCENT_BTN_CSS.replace("@color", accent.w3_color_keywords()));
            let is_dark = SETTINGS.read().theme_is_dark;
            sender.oneshot_command(async move {
                theme::set_theme(None, is_dark, Some(accent)).await.unwrap();
            });
        } else {
            b.inline_css(&DISABLED_ACCENT_BTN_CSS.replace("@color", accent.w3_color_keywords()));
            let is_dark = SETTINGS.read().theme_is_dark;
            sender.oneshot_command(
                async move { theme::set_theme(None, is_dark, None).await.unwrap() },
            );
        }
    }
}
