use proc_macro::TokenStream;

mod struct_map;

#[proc_macro]
pub fn struct_map(input: TokenStream) -> TokenStream {
    struct_map::struct_map(input)
}
