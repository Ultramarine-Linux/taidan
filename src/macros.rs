#[macro_export]
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

#[macro_export]
macro_rules! generate_page {
    ($page:ident $({$($model:tt)+})?:
        $(
        init($root:ident, $initsender:ident) $initblock:block
        )?
        update($self:ident, $message:ident, $sender:ident) {
            $( $msg:ident$(($($param:ident: $paramtype:ty),+$(,)?))? => $msghdl:expr ),*$(,)?
        }
        => {$( $out:pat ),*}
        $($viewtt:tt)+
    ) => {
        use $crate::prelude::*;
        use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

        ::paste::paste! {
            $crate::generate_page!{ @model $page $($($model)+)? }

            #[derive(Debug)]
            pub enum [<$page PageMsg>] {
                Nav(NavAction),
                $($msg$(($($paramtype),+))?),*
            }

            #[derive(Debug)]
            pub enum [<$page PageOutput>] {
                Nav(NavAction),
                $($out),*

            }

            #[relm4::component(pub)]
            impl SimpleComponent for [<$page Page>] {
                type Init = ();
                type Input = [<$page PageMsg>];
                type Output = [<$page PageOutput>];

                view! {
                    libhelium::ViewMono {
                        append = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 4,
                            set_margin_all: 16,

                            $($viewtt)+
                        }
                    }
                }


                fn init(
                    _init: Self::Init,
                    root: Self::Root,
                    $sender: ComponentSender<Self>,
                ) -> ComponentParts<Self> {
                    let model = Self::default();
                    let widgets = view_output!();

                    ComponentParts { model, widgets }
                }

                fn update(&mut $self, $message: Self::Input, $sender: ComponentSender<Self>) {
                    match $message {
                        Self::Input::Nav(action) => {
                            $sender.output(Self::Output::Nav(action)).unwrap();
                        }
                        $(Self::Input::$msg$(($($param),+))? => $msghdl),*
                    }
                }
            }
        }
    };
    (@model $page:ident $($model:tt)+) => {paste::paste! {
        #[derive(Debug, Default)]
        pub struct [<$page Page>] {$($model)+}
    }};
    (@model $page:ident) => {paste::paste! {
        #[derive(Debug, Default)]
        pub struct [<$page Page>];
    }};
    (@init $sender:ident) => {
        fn init(
            _init: Self::Init,
            root: Self::Root,
            $sender: ComponentSender<Self>,
        ) -> ComponentParts<Self> {
            let model = Self::default();
            let widgets = view_output!();

            ComponentParts { model, widgets }
        }
    };
    (@init $sender:ident $root:ident $initsender:ident $initblock:block) => {
        fn init(
            _init: Self::Init,
            $root: Self::Root,
            $initsender: ComponentSender<Self>,
        ) -> ComponentParts<Self> $initblock
    };
}
