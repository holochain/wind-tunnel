use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, Ident, ItemFn, LitStr, Token};

#[derive(Default)]
struct InstrumentArgs {
    /// Prefix to apply to the function name when reporting the instrumented function
    prefix: Option<String>,
}

impl Parse for InstrumentArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = InstrumentArgs::default();

        loop {
            if input.is_empty() {
                break;
            }

            let key: Ident = input.parse()?;
            match key.to_string().as_str() {
                "prefix" => {
                    input.parse::<Token![=]>()?;
                    let value: LitStr = input.parse()?;
                    args.prefix = Some(value.value());
                }
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Unknown argument for #[wind_tunnel_instrument]: {}", key),
                    ));
                }
            }
        }

        Ok(args)
    }
}

#[proc_macro_attribute]
pub fn wind_tunnel_instrument(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as ItemFn);

    // // Use a syntax tree traversal to transform the function body.
    // let output = args.fold_item_fn(input);
    let InstrumentArgs { prefix } = parse_macro_input!(args as InstrumentArgs);

    let target_name = prefix.unwrap_or("".to_string()) + &input.sig.ident.to_string();

    input.block.stmts.insert(
        0,
        syn::parse_quote! {
            let wt_operation_record = wind_tunnel_instruments::OperationRecord::new(#target_name.to_string());
        },
    );

    let result = input.block.stmts.pop().unwrap();
    input.block.stmts.push(syn::parse_quote! {
        let wt_result = #result;
    });
    input.block.stmts.push(syn::parse_quote! {
        wind_tunnel_instruments::report_operation(wt_operation_record, &wt_result);
    });
    input.block.stmts.push(syn::parse_quote! {
        return wt_result;
    });

    // Hand the resulting function body back to the compiler.
    TokenStream::from(quote!(#input))
}
