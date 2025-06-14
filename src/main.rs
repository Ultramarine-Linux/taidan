#![warn(rust_2018_idioms)]
pub mod backend;
pub mod cfg;
pub mod macros;
pub mod pages;
pub mod prelude;
pub mod ui;

use std::sync::LazyLock;

use crate::prelude::*;
use gtk::glib::translate::FromGlibPtrNone;
use i18n_embed::LanguageLoader;
use parking_lot::RwLock;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, RelmApp, SimpleComponent,
};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

const APPID: &str = "com.fyralabs.Taidan";
static LOCALE_SOLVER: LazyLock<poly_l10n::LocaleFallbackSolver> = LazyLock::new(Default::default);
static AVAILABLE_LANGS: LazyLock<Vec<i18n_embed::unic_langid::LanguageIdentifier>> =
    LazyLock::new(|| {
        i18n_embed::fluent::fluent_language_loader!()
            .available_languages(&Localizations)
            .unwrap()
    });

// configuration of the distro OOBE.
pub static CFG: LazyLock<cfg::Config> = LazyLock::new(|| {
    tracing::debug!("Initializing cfg::Config");
    let mut cfg = cfg::Config::new().expect("cannot init config");
    cfg.populate();
    tracing::debug!("Populated cfg::Config (turn on `trace` to see body)");
    cfg
});
pub static SETTINGS: relm4::SharedState<backend::settings::Settings> = relm4::SharedState::new();
pub static LL: std::sync::LazyLock<RwLock<i18n_embed::fluent::FluentLanguageLoader>> =
    std::sync::LazyLock::new(|| RwLock::new(handle_l10n()));

#[derive(rust_embed::RustEmbed)]
#[folder = "po/"]
#[exclude = "en-owo/*.ftl"]
struct Localizations;

generate_pages!(Page AppModel AppMsg:
    00: Language,
    01: Welcome,
    02: Keyboard,
    03: WhoAreYou,
    04: Password,
    05: Internet,
    06: Analytics,
    07: CrashReport,
    08: Tweaks,
    09: Codecs,
    10: InputMethod,
    11: NightLight,
    12: Theme,
    13: Browser,
    14: Categories,
    15: Installing,
    16: Finish,
    17: Error,
);

#[derive(Debug, Clone)]
pub enum NavAction {
    GoTo(Page),
    Next,
    Back,
    Quit,
}

#[derive(Debug)]
pub enum AppMsg {
    Nav(NavAction),
    InstallError(String),
}

#[allow(clippy::str_to_string)]
#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Init = ();

    type Input = AppMsg;
    type Output = ();

    view! {
        libhelium::ApplicationWindow {
            set_title: Some(&t!("page-welcome", distro = CFG.distro.clone())),
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
                    Page::Language => *model.language_page.widget(),
                    Page::Welcome => *model.welcome_page.widget(),
                    Page::Keyboard => *model.keyboard_page.widget(),
                    Page::WhoAreYou => *model.who_are_you_page.widget(),
                    Page::Password => *model.password_page.widget(),
                    Page::Internet => *model.internet_page.widget(),
                    Page::Analytics => *model.analytics_page.widget(),
                    Page::CrashReport => *model.crash_report_page.widget(),
                    Page::Tweaks => *model.tweaks_page.widget(),
                    Page::Codecs => *model.codecs_page.widget(),
                    Page::InputMethod => *model.input_method_page.widget(),
                    Page::NightLight => *model.night_light_page.widget(),
                    Page::Theme => *model.theme_page.widget(),
                    Page::Browser => *model.browser_page.widget(),
                    Page::Categories => *model.categories_page.widget(),
                    Page::Installing => *model.installing_page.widget(),
                    Page::Finish => *model.finish_page.widget(),
                    Page::Error => *model.error_page.widget(),
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
        let display = gtk::gdk::Display::default().unwrap();
        let settings = gtk::Settings::for_display(&display);
        settings.set_gtk_icon_theme_name(Some("Hydrogen"));
        let theme = gtk::IconTheme::for_display(&display);
        theme.add_resource_path("/com/fyralabs/Taidan/icons/symbolic/actions");

        let model = Self::_default(sender);

        let widgets = view_output!();

        widgets.stack.set_vexpand(true);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        tracing::trace!(?message, "AppModel: Received message");
        match message {
            AppMsg::Nav(NavAction::Next)
                if self.page == Page::Password && SETTINGS.read().skipconfig =>
            {
                self.page = Page::Installing;
                self.run_install(sender, backend::start_simple_install);
            }
            AppMsg::Nav(NavAction::Next)
                if self.page == Page::Theme && SETTINGS.read().nointernet =>
            {
                self.page = Page::Installing;
                self.run_install(sender, backend::start_install);
            }
            AppMsg::Nav(NavAction::GoTo(page)) => {
                self.page = page;
                if page == Page::Installing {
                    self.run_install(sender, backend::start_install);
                }
            }
            AppMsg::Nav(NavAction::Quit) => {
                sender.oneshot_command(async {
                    if let Err(e) = backend::pkexec("root", "rm", &["-rf", "/.unconfigured"]).await
                    {
                        tracing::error!(?e, "cannot remove /.unconfigured; exiting anyway");
                    } else {
                        tracing::debug!("removed /.unconfigured");
                    }
                });
                relm4::main_application().quit();
            }
            AppMsg::Nav(NavAction::Next) => {
                while {
                    self.page = usize::from(self.page)
                        .wrapping_add(1)
                        .try_into()
                        .unwrap_or(self.page);
                    !self.get_page_widget().is_visible()
                } {}
                if self.page == Page::Installing {
                    self.run_install(sender, backend::start_install);
                }
            }
            AppMsg::Nav(NavAction::Back) => {
                while {
                    self.page = usize::from(self.page)
                        .wrapping_sub(1)
                        .try_into()
                        .unwrap_or(self.page);
                    !self.get_page_widget().is_visible()
                } {}
            }
            AppMsg::InstallError(msg) => {
                self.page = Page::Error;
                self.error_page
                    .sender()
                    .send(pages::ErrorPageMsg::Receive(msg))
                    .expect("sender dropped?");
            }
        }
    }
}

type Sd = relm4::Sender<pages::InstallingPageMsg>;
impl AppModel {
    #[allow(clippy::needless_pass_by_value)]
    fn run_install<Fut, F>(&self, sender: ComponentSender<Self>, f: F)
    where
        Fut: std::future::Future<Output = color_eyre::Result<()>> + Send,
        F: Fn(backend::settings::Settings, Sd) -> Fut + Send + 'static,
    {
        let inst_sender = self.installing_page.sender().clone();
        let (ss, sett) = (sender.clone(), SETTINGS.read().clone());
        sender.oneshot_command(async move {
            if let Err(e) = f(sett, inst_sender).await {
                ss.input(AppMsg::InstallError(strip_ansi_escapes::strip_str(
                    format!("{e:?}"),
                )));
            }
        });
    }
}

fn handle_l10n() -> i18n_embed::fluent::FluentLanguageLoader {
    use i18n_embed::{fluent::fluent_language_loader, LanguageLoader};
    let loader = fluent_language_loader!();
    let locale_solver = poly_l10n::LocaleFallbackSolver::<poly_l10n::Rulebook>::default();
    let available_langs = loader.available_languages(&Localizations).unwrap();
    let mut langs = poly_l10n::system_want_langids()
        .flat_map(|li| locale_solver.solve_locale(li))
        .filter(|li| available_langs.contains(li))
        .collect_vec();
    if langs.is_empty() {
        langs = vec![loader.fallback_language().clone()];
    }
    loader.load_languages(&Localizations, &langs).unwrap();
    loader
}

#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
fn main() {
    let _guard = setup_logs_and_install_panic_hook();
    std::sync::LazyLock::force(&LL);
    // FIXME: temp hack to set nointernet to true here (should be default value), please refactor
    SETTINGS.write().nointernet = true;

    gtk::gio::resources_register_include!("icons.gresource").unwrap();

    let app = libhelium::Application::builder()
        .application_id(APPID)
        .flags(gtk::gio::ApplicationFlags::default())
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
}

/// Returns a logging guard.
///
/// # Panics
/// - cannot install `color_eyre`
/// - cannot create taidan tempdir
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
        .with(
            EnvFilter::builder()
                .with_default_directive(tracing::level_filters::LevelFilter::DEBUG.into())
                .with_env_var("TAIDAN_LOG")
                .from_env_lossy(),
        )
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
        .with(tracing_error::ErrorLayer::default())
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
