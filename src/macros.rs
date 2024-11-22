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

        #[derive(Debug)]
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
    ($page:ident $({$($model:tt)+})? $(as $modelname:ident)?:
        $(
        init$([$($local_ref:ident)+])?($root:ident, $initsender:ident, $initmodel:ident, $initwidgets:ident) $initblock:block
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
                    #[allow(unused_mut)]
                    let mut model = Self::default();

                    $($($(let $local_ref = &model.$local_ref;)+)?)?

                    // HACK: invoking view_output!() directly gives `()` when $init* is given.
                    // I don't know why this fixes the issue. — mado
                    let widgets = [<view _output>]!();

                    $(
                        #[allow(unused_mut)]
                        // HACK: this solves variable name obfuscation in macro_rules! {}
                        let mut $initmodel = model;
                        let $initwidgets = widgets;
                        #[allow(unused_variables)]
                        let $initsender = $sender;

                        $initblock

                        let model = $initmodel;
                        let widgets = $initwidgets;
                    )?

                    ComponentParts { model, widgets }
                }

                fn update(&mut $self, $message: Self::Input, $sender: ComponentSender<Self>) {
                    tracing::trace!(?$message, "{}", const_format::concatcp!(stringify!($page), "Page: received message"));
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
}

#[macro_export]
macro_rules! generate_component {
    ($comp:ident $({$($model:tt)+})?:
        $(
        init$([$($local_ref:ident)+])?($root:ident, $initsender:ident, $initmodel:ident, $initwidgets:ident) $(for $init:ident: $InitType:ty)? $initblock:block
        )?
        update($self:ident, $message:ident, $sender:ident) {
            $( $msg:ident$(($($param:ident: $paramtype:ty),+$(,)?))? => $msghdl:expr ),*$(,)?
        }
        => $out:tt
        $($viewtt:tt)+
    ) => { ::paste::paste! {
        $crate::generate_component!{ @model $comp $($($model)+)?}
        #[derive(Debug)]
        pub enum [<$comp Msg>] {
            $($msg$(($($paramtype),+))?),*
        }

        $crate::generate_component!(@out $comp $out);

        #[relm4::component(pub)]
        impl SimpleComponent for $comp {
            #[allow(unused_parens)]
            type Init = ($($($InitType)?)?);
            type Input = [<$comp Msg>];
            type Output = $crate::generate_component!(@outty $comp $out);

            view! { $($viewtt)+ }


            #[allow(clippy::used_underscore_binding)]
            fn init(
                init: Self::Init,
                root: Self::Root,
                $sender: ComponentSender<Self>,
            ) -> ComponentParts<Self> {
                $crate::generate_component!(@default {
                    let model = Self::default();
                } $({
                    #[allow(unused_mut)]
                    let mut $initmodel = Self::default();
                })?);

                $($($(let $local_ref = &$initmodel.$local_ref;)?)?)+

                $(
                    let $root = root.clone();
                    $(let $init = init;)?
                )?

                // HACK: invoking view_output!() directly gives `()` when $init* is given.
                // I don't know why this fixes the issue. — mado
                let widgets = [<view _output>]!();

                $(
                #[allow(unused_mut)]
                // HACK: this solves variable name obfuscation in macro_rules! {}
                let $initwidgets = widgets;
                #[allow(unused_variables)]
                let $initsender = $sender;

                $initblock

                let model = $initmodel;
                let widgets = $initwidgets;
                )?

                ComponentParts { model, widgets }
            }

            fn update(&mut $self, $message: Self::Input, $sender: ComponentSender<Self>) {
                tracing::trace!(?$message, "{}", const_format::concatcp!(stringify!($comp), ": received message"));
                match $message {
                    $(Self::Input::$msg$(($($param),+))? => $msghdl),*
                }
            }
        }
    }};
    (@model $comp:ident $($model:tt)+) => {paste::paste! {
        #[derive(Debug, Default)]
        pub struct [<$comp>] {$($model)+}
    }};
    (@model $comp:ident) => {paste::paste! {
        #[derive(Debug, Default)]
        pub struct [<$comp>];
    }};
    (@out $comp:ident {$( $out:pat ),*}) => { ::paste::paste! {
        #[derive(Debug)]
        pub enum [<$comp Output>] {
            $($out),*
        }
    }};
    (@out $comp:ident $outty:ty) => { };
    (@outty $comp:ident {$( $out:pat ),*}) => { ::paste::paste! { [<$comp Output>] }};
    (@outty $comp:ident $outty:ty) => { $outty };

    (@default {$($default:tt)+} {$($if:tt)+}) => { $($if)+ };
    (@default {$($default:tt)+}) => { $($default)+ };

    (@init $body:block {$($inner:tt)+}($sender:ident)) => {
        fn init(
            init: Self::Init,
            root: Self::Root,
            $sender: ComponentSender<Self>,
        ) -> ComponentParts<Self> { $body }
    };
    (@init $body:block {$($inner:tt)+}($sender:ident $root:ident)) => {
        fn init(
            init: Self::Init,
            $root: Self::Root,
            $sender: ComponentSender<Self>,
        ) -> ComponentParts<Self> { $body }
    };
    (@init $body:block {$($inner:tt)+}($sender:ident $root:ident $init:ident)) => {
        fn init(
            $init: Self::Init,
            $root: Self::Root,
            $sender: ComponentSender<Self>,
        ) -> ComponentParts<Self> { $body }
    };
    (@do_nothing $($tt:tt)+) => { $($tt:tt)+ };
}
