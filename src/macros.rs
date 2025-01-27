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

macro_rules! kurage_page_pre {
    () => {
        use crate::prelude::*;
        use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
    };
}
pub(crate) use kurage_page_pre;

kurage::generate_generator! { generate_page =>
    libhelium::ViewMono {
        append = &gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 4,

            KURAGE_INNER
        },
    },
}
pub(crate) use generate_page;
