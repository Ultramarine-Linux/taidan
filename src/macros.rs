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
        set_visible: !CFG.skip_pages.contains(&$crate::Page::$name),
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

#[macro_export]
macro_rules! t {
    ($msgid:literal $($tt:tt)*) => {
        i18n_embed_fl::fl!($crate::LL, $msgid $($tt)*)
    };
}

#[macro_export]
macro_rules! generate_pages {
    ($Page:ident $AppModel:ident $AppMsg:ident: $($num:tt: $page:ident $($forward:expr)?),+$(,)?) => { ::paste::paste! {
        use pages::{$([<_$num _$page:lower>]::[<$page:camel Page>]),+};
        use pages::{$([<_$num _$page:lower>]::[<$page:camel PageOutput>]),+};


        #[derive(Debug, Default, PartialEq, Eq, Clone, Copy, serde::Deserialize)]
        #[serde(rename_all = "lowercase")]
        pub enum $Page {
            #[default]
            $([< $page:camel >]),+
        }

        impl TryFrom<usize> for $Page {
            type Error = ();

            fn try_from(value: usize) -> Result<Self, ()> {
                #[allow(clippy::zero_prefixed_literal)]
                Ok(match value {
                    $( $num => Self::[<$page:camel>], )+
                    _ => return Err(()),
                })
            }
        }
        impl From<$Page> for usize {
            fn from(val: $Page) -> Self {
                #[allow(clippy::zero_prefixed_literal)]
                match val {
                    $( $Page::[<$page:camel>] => $num, )+
                }
            }
        }

        #[derive(Debug)]
        pub struct $AppModel {
            page: $Page,
            $(
                pub [<$page:snake _page>]: ::relm4::Controller<[<$page:camel Page>]>,
            )+
        }

        impl $AppModel {
            fn _default(sender: ComponentSender<Self>) -> Self {Self {
                page: $Page::default(),
                $(
                    [<$page:snake _page>]: [<$page:camel Page>]::builder()
                        .launch(())
                        .forward(sender.input_sender(), $crate::generate_pages!(@$page $AppMsg $($forward)?)),
                )+
            }}
            fn get_page_widget(&self) -> &libhelium::ViewMono {
                match self.page {$(
                    $Page::[<$page:camel>] => self.[<$page:snake _page>].widget(),
                )+}
            }
        }
    }};
    (@$page:ident $AppMsg:ident) => { ::paste::paste! {
        |msg| match msg {
            [<$page:camel PageOutput>]::Nav(action) => $AppMsg::Nav(action),
        }
    }};
    (@$page:ident $AppMsg:ident $forward:expr) => { $forward };
}
