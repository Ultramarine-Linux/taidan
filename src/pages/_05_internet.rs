use crate::prelude::*;
skipconfig!();
generate_page!(Internet {
    btn_next: libhelium::Button,
    is_online: bool,
}:
    init(root, sender, model, widgets) {
        let sender1 = sender.clone();
        sender.oneshot_command(async move { check_online(sender1).await });
        model.btn_next = widgets.prev_next_btns.next.clone();
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
            set_label: &t!("page-internet"),
            add_css_class: "view-subtitle",
            inline_css: "font-weight: bold",
        },

        gtk::Label {
            set_justify: gtk::Justification::Center,
            set_label: &t!("page-internet-desc"),
        },

        libhelium::Button {
            set_is_textual: true,
            set_halign: gtk::Align::Center,
            set_hexpand: false,
            set_label: &t!("page-internet-skip"),
            inline_css: "padding-left: 48px; padding-right: 48px",
            connect_clicked[sender] => move |_| {
                SETTINGS.write().nointernet = true;
                sender.input(Self::Input::Nav(NavAction::Next));
            },
        },

        gtk::Label {
            set_label: &t!("page-internet-ok"),
            #[watch]
            set_visible: model.is_online,
            add_css_class: "success",
        },

        gtk::Label {
            set_label: &t!("page-internet-warn"),
            add_css_class: "warning",
            #[watch]
            set_visible: !model.is_online,
        },

        libhelium::Button {
            set_is_pill: true,
            set_halign: gtk::Align::Center,
            // set_icon: Some("network-wireless-symbolic"),
            set_label: &t!("page-internet-open"),
            connect_clicked[sender] => move |_| sender.oneshot_command(async { crate::backend::steps::acmd("netto", &[]).await.unwrap() }),
        },

        libhelium::Button {
            set_is_tint: true,
            set_halign: gtk::Align::Center,
            set_label: &t!("page-internet-portal"),
            connect_clicked[sender] => move |_| sender.oneshot_command(async { crate::backend::steps::acmd("xdg-open", &["http://detectportal.firefox.com/canonical.html"]).await.unwrap() }),
            #[watch]
            set_visible: !model.is_online,
        },
    },

    #[name(prev_next_btns)]
    #[template] crate::ui::PrevNextBtns {
        #[template_child] prev {
            connect_clicked => Self::Input::Nav(NavAction::Back),
        },
        #[template_child] next {
            set_sensitive: false,
            connect_clicked[sender] => move |_| {
                SETTINGS.write().nointernet = false;
                sender.input(Self::Input::Nav(NavAction::Next));
            },
        },
    }
);

#[allow(clippy::equatable_if_let)]
async fn check_online(sender: ComponentSender<InternetPage>) {
    let arch = std::env::consts::ARCH;
    let edition = &CFG.edition;
    loop {
        let mut req = http_types::Request::post("https://plausible.fyralabs.com/api/event");
        req.insert_header(http_types::headers::CONTENT_TYPE, "application/json");
        req.set_body(format!(
            r#"
                {{
                    "name": "pageview",
                    "url": "app://internet/{arch}/{edition}",
                    "domain": "taidan",
                    "props": {{
                        "arch": "{arch}",
                        "edition": "{edition}"
                    }}
                }}
            "#
        ));
        if crate::a::https_req(req)
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            break;
        }
        async_io::Timer::after(std::time::Duration::from_secs(5)).await;
    }
    sender.input(InternetPageMsg::IsOnline);
}
