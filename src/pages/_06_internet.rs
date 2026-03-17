use crate::backend::steps;
use crate::prelude::*;

const DETECT_INTERNET_SCRIPT: &str =
    const_format::formatcp!("/etc/{}/detect-internet", crate::APPID);

skipconfig!();
generate_page!(Internet {
    btn_next: libhelium::Button,
    is_online: bool,
}:
    init(root, sender, model, widgets) {
        let sender1 = sender.clone();
        sender.oneshot_command(async move { check_online(sender1).await });
        model.btn_next = widgets.prev_next_btns.next.clone();

        // Add keyboard event handler for Enter key
        let sender_clone = sender.clone();
        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Return {
                // Navigate to next page when Enter is pressed
                SETTINGS.write().nointernet = false;
                sender_clone.input(Self::Input::Nav(NavAction::Next));
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });
        root.add_controller(key_controller);
    }
    update(self, message, sender) {
        IsOnline => {
            self.is_online = true;
            self.btn_next.set_sensitive(true);
        }
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
            set_icon_name: Some("network-wireless-symbolic"),
            inline_css: "-gtk-icon-size: 64px",
        },

        gtk::Label {
            #[watch]
            set_label: &t!("page-internet"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_justify: gtk::Justification::Center,
            #[watch]
            set_label: &t!("page-internet-desc"),
        },

        libhelium::Button {
            set_is_textual: true,
            set_halign: gtk::Align::Center,
            set_hexpand: false,
            #[watch]
            set_label: &t!("page-internet-skip"),
            inline_css: "padding-left: 48px; padding-right: 48px",
            #[watch]
            set_visible: !model.is_online,
            connect_clicked[sender] => move |_| {
                SETTINGS.write().nointernet = true;
                sender.input(Self::Input::Nav(NavAction::Next));
            },
        },

        gtk::Label {
            #[watch]
            set_label: &t!("page-internet-ok"),
            #[watch]
            set_visible: model.is_online,
            add_css_class: "success",
        },

        gtk::Label {
            #[watch]
            set_label: &t!("page-internet-warn"),
            add_css_class: "warning",
            #[watch]
            set_visible: !model.is_online,
        },

        libhelium::Button {
            set_is_pill: true,
            set_halign: gtk::Align::Center,
            // set_icon: Some("network-wireless-symbolic"),
            #[watch]
            set_label: &t!("page-internet-open"),
            #[watch]
            set_visible: !model.is_online,
            connect_clicked[sender] => move |_| sender.oneshot_command(async { crate::backend::steps::acmd("netto", &[]).await.unwrap() }),
        },

        libhelium::Button {
            set_is_tint: true,
            set_halign: gtk::Align::Center,
            #[watch]
            set_label: &t!("page-internet-portal"),
            connect_clicked[sender] => move |_| sender.oneshot_command(async { crate::backend::steps::acmd("xdg-open", &["http://detectportal.firefox.com/canonical.html"]).await.unwrap() }),
            #[watch]
            set_visible: !model.is_online,
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
            set_sensitive: false,
            connect_clicked[sender] => move |_| {
                SETTINGS.write().nointernet = false;
                sender.input(Self::Input::Nav(NavAction::Next));
            },
        },
    }
);

async fn check_online(sender: ComponentSender<InternetPage>) {
    let retry_interval = std::time::Duration::from_secs(CFG.internet_retry_interval);
    if std::path::Path::new(DETECT_INTERNET_SCRIPT).exists() {
        while let Err(e) = steps::acmd(DETECT_INTERNET_SCRIPT, &[]).await {
            tracing::warn!(?e, "Internet detection script failed, continuing anyway");
            tokio::time::sleep(retry_interval).await;
        }
    } else {
        let timeout = CFG.internet_timeout.to_string();
        while !tokio::process::Command::new("ping")
            .args(["-c", "1", "-W", &timeout, "1.1.1.1"])
            .status()
            .await
            .is_ok_and(|r| r.success())
        {
            tokio::time::sleep(retry_interval).await;
        }
    }
    sender.input(InternetPageMsg::IsOnline);
}
