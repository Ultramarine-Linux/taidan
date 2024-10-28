#![warn(rust_2018_idioms)]

pub mod cfg;
pub mod pages;
pub mod prelude;

use crate::prelude::*;
use gtk::glib::translate::FromGlibPtrNone;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, RelmApp, SharedState,
    SimpleComponent,
};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

const APPID: &str = "com.fyralabs.Readymade";

pub static CONFIG: SharedState<cfg::Config> = SharedState::new();

macro_rules! generate_pages {
    ($Page:ident $AppModel:ident $AppMsg:ident: $($num:tt: $page:ident $($forward:expr)?),+$(,)?) => {paste::paste! {
        use pages::{$([<_$num _$page:lower>]::[<$page:camel Page>]),+};
        use pages::{$([<_$num _$page:lower>]::[<$page:camel PageOutput>]),+};


        #[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
        pub enum $Page {
            #[default]
            $([< $page:camel >]),+
        }

        impl TryFrom<usize> for $Page {
            type Error = ();

            fn try_from(value: usize) -> Result<Self, Self::Error> {
                Ok(match value {
                    $( $num => Self::[<$page:camel>], )+
                    _ => return Err(()),
                })
            }
        }
        impl From<$Page> for usize {
            fn from(val: $Page) -> Self {
                match val {
                    $( $Page::[<$page:camel>] => $num, )+
                }
            }
        }

        struct $AppModel {
            page: $Page,
            $(
                [<$page:snake _page>]: relm4::Controller<[<$page:camel Page>]>,
            )+
        }

        impl $AppModel {
            fn _default(sender: ComponentSender<Self>) -> Self {Self {
                page: $Page::default(),
                $(
                    [<$page:snake _page>]: [<$page:camel Page>]::builder()
                        .launch(())
                        .forward(sender.input_sender(), generate_pages!(@$page $AppMsg $($forward)?)),
                )+
            }}
        }
    }};
    (@$page:ident $AppMsg:ident) => {paste::paste! {
        |msg| match msg {
            [<$page:camel PageOutput>]::Nav(action) => $AppMsg::Nav(action),
        }
    }};
    (@$page:ident $AppMsg:ident $forward:expr) => { $forward };
}

generate_pages!(Page AppModel AppMsg:
    00: Welcome,
    01: WhoAreYou,
);

#[derive(Debug)]
pub enum NavAction {
    GoTo(Page),
    Next,
    Back,
    Quit,
}

#[derive(Debug)]
enum AppMsg {
    Nav(NavAction),
}

#[allow(clippy::str_to_string)]
#[relm4::component]
impl SimpleComponent for AppModel {
    type Init = ();

    type Input = AppMsg;
    type Output = ();

    view! {
        libhelium::ApplicationWindow {
            set_title: Some(&gettext("Welcome to %s").replace("%s", &CONFIG.read().distro)),
            set_default_width: 600,
            set_default_height: 500,
            set_vexpand: true,
            set_align: gtk::Align::Fill,

            #[wrap(Some)]
            set_child = &gtk::Box {
                set_vexpand: true,
                set_align: gtk::Align::Fill,
                set_orientation: gtk::Orientation::Vertical,

                #[transition = "SlideLeftRight"]
                #[name = "stack"]
                match model.page {
                    Page::Welcome => *model.welcome_page.widget(),
                    Page::WhoAreYou => *model.who_are_you_page.widget(),
                }
            }
        }
    }

    // Initialize the UI.
    fn init(
        (): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // TODO: make libhelium force this
        let settings = gtk::Settings::for_display(&gtk::gdk::Display::default().unwrap());
        settings.set_gtk_icon_theme_name(Some("Hydrogen"));

        let model = Self::_default(sender);

        let widgets = view_output!();

        widgets.stack.set_vexpand(true);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::Nav(NavAction::GoTo(page)) => self.page = page,
            AppMsg::Nav(NavAction::Quit) => std::process::exit(0),
            AppMsg::Nav(NavAction::Next) => {
                self.page = usize::from(self.page)
                    .wrapping_add(1)
                    .try_into()
                    .expect("No next page!");
            }
            AppMsg::Nav(NavAction::Back) => {
                self.page = usize::from(self.page)
                    .wrapping_sub(1)
                    .try_into()
                    .expect("No prev page!");
            }
        }
    }
}

#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
fn main() -> std::io::Result<()> {
    let _guard = setup_logs_and_install_panic_hook();
    CONFIG.write().populate();

    gettextrs::textdomain(APPID)?;
    gettextrs::bind_textdomain_codeset(APPID, "UTF-8")?;

    let app = libhelium::Application::builder()
        .application_id(APPID)
        .flags(libhelium::gtk::gio::ApplicationFlags::default())
        // SAFETY: placeholder
        .default_accent_color(unsafe {
            &libhelium::RGBColor::from_glib_none(std::ptr::from_mut(
                &mut libhelium::ffi::HeRGBColor {
                    r: 0.0,
                    g: 7.0,
                    b: 143.0,
                },
            ))
        })
        .build();

    tracing::debug!("Starting Taidan");
    RelmApp::from_app(app).run::<AppModel>(());
    Ok(())
}

/// Returns a logging guard.
///
/// # Panics
/// - cannot install `color_eyre`
/// - cannot create readymade tempdir
#[allow(clippy::cognitive_complexity)]
fn setup_logs_and_install_panic_hook() -> impl std::any::Any {
    color_eyre::install().expect("install color_eyre");
    let temp_dir = tempfile::Builder::new()
        .prefix("taidan-logs")
        .tempdir()
        .expect("create logs tempdir")
        .into_path();
    // create dir
    std::fs::create_dir_all(&temp_dir).expect("create logs tempdir");
    let file_appender = tracing_appender::rolling::never(&temp_dir, "taidan.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(fmt::layer().pretty())
        .with(EnvFilter::from_env("TAIDAN_LOG"))
        .with(
            tracing_journald::layer()
                .unwrap()
                .with_syslog_identifier("taidan".to_owned()),
        )
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .compact(),
        )
        .init();

    if cfg!(debug_assertions) {
        tracing::info!("Running in debug mode");
    }
    tracing::info!("Taidan {version}", version = env!("CARGO_PKG_VERSION"));
    tracing::info!("Logging to journald");
    tracing::info!(
        "Logging to {tmp}/taidan.log",
        tmp = temp_dir.to_string_lossy()
    );
    guard
}
