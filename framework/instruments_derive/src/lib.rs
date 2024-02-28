use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn wind_tunnel_instrument(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as ItemFn);

    // // Use a syntax tree traversal to transform the function body.
    // let output = args.fold_item_fn(input);

    input.block.stmts.insert(
        0,
        syn::parse_quote! {
            println!("You are running generate agent pub key");
        },
    );

    // Hand the resulting function body back to the compiler.
    TokenStream::from(quote!(#input))
}
