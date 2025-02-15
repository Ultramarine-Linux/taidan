#[macro_export]
macro_rules! skipconfig_skip_page {
    ($page:ident) => {
        ::paste::paste! {
            impl $crate::ui::PageTrig for [<$page Page>] {
                fn arrive(&self) -> bool { $crate::SETTINGS.read().skipconfig }
            }
        }
    };
}

#[macro_export]
macro_rules! always_skip_page {
    ($page:ident) => {
        ::paste::paste! {
            impl $crate::ui::PageTrig for [<$page Page>] {
                fn arrive(&self) -> bool { true }
            }
        }
    };
}

kurage::kurage_gen_macros!();

kurage::generate_generator! { generate_page => [<$name Page>]:
    update: {
        Nav(action: NavAction) => $sender.output(Self::Output::Nav(action)).unwrap(),
    } => { Nav(NavAction) }

    libhelium::ViewMono {
        set_show_right_title_buttons: false,
        append = &gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 4,

            KURAGE_INNER
        },
    },
}
pub(crate) use generate_page;

#[macro_export]
macro_rules! awrite {
    ($f:ident <- $s:literal $($args:tt)*)=> {
        $f.write_all(format!($s $($args)*).as_bytes()).await
    };
}
