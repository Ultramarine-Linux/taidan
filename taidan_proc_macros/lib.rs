use itertools::Itertools;
use proc_macro::TokenStream;
use quote::quote;

/// Generate the `KEYMAP` static map.
///
/// This proc_macro is modified from this message:
/// <https://discord.com/channels/273534239310479360/1120124565591425034/1324394178133884928>
///
/// Big thanks to (gh) @circles-png for helping!
#[proc_macro]
pub fn keymap(static_name: TokenStream) -> TokenStream {
    let static_name = quote::format_ident!("{}", static_name.to_string());
    let file = std::fs::read_to_string("/usr/share/X11/xkb/rules/evdev.lst").unwrap();
    let mut sections = file.split("\n\n");
    let layouts = sections.nth(1).unwrap();
    let variants = sections.next().unwrap();
    let variants = variants
        .lines()
        .skip(1)
        .map(|line| {
            let line = line.trim();
            let variant_id = line.split_once(" ").unwrap().0;
            let second_column = line.split_once(" ").unwrap().1.trim();
            let (layout_id, name) = second_column.split_once(": ").unwrap();
            (variant_id, layout_id, name)
        })
        .into_group_map_by(|(_, layout_id, _)| *layout_id);
    let layouts = layouts
        .lines()
        .skip(1)
        .map(|line| {
            let line = line.trim();
            let id = line.split_once(" ").unwrap().0;
            let name = line.split_once(" ").unwrap().1.trim();
            let variants = variants
                .get(id)
                .map(|variants| {
                    variants
                        .iter()
                        .map(|(variant_id, _, name)| {
                            quote! {
                                #variant_id => #name,
                            }
                        })
                        .collect_vec()
                })
                .unwrap_or_default();
            quote! {
                #id => Layout {
                    name: #name,
                    variants: phf::phf_map! {
                        #(#variants)*
                    },
                },
            }
        })
        .collect_vec();
    quote! {
        #[derive(Debug)]
        pub struct Layout {
            pub name: &'static str,
            pub variants: phf::Map<&'static str, &'static str>,
        }
        pub static #static_name: phf::Map<&'static str, Layout> = phf::phf_map! {
            #(#layouts)*
        };
    }
    .into()
}
