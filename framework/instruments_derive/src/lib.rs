use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse::Parse, parse_macro_input, parse_quote, Expr, Ident, ItemFn, LitStr, Token};

#[derive(Default)]
struct InstrumentArgs {
    /// Prefix to apply to the function name when reporting the instrumented function
    prefix: Option<String>,
    pre_hook: Option<Ident>,
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
                "pre_hook" => {
                    input.parse::<Token![=]>()?;
                    args.pre_hook = Some(input.parse()?);
                }
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Unknown argument for #[wind_tunnel_instrument]: {}", key),
                    ));
                }
            }

            match input.parse::<Token![,]>() {
                Ok(_) => {}
                Err(e) => {
                    if input.is_empty() {
                        break;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(args)
    }
}

#[proc_macro_attribute]
pub fn wind_tunnel_instrument(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as ItemFn);

    // Use a syntax tree traversal to transform the function body.
    let InstrumentArgs { prefix, pre_hook } = parse_macro_input!(args as InstrumentArgs);

    let target_name = prefix.unwrap_or("".to_string()) + &input.sig.ident.to_string();

    input.block.stmts.insert(
        0,
        syn::parse_quote! {
            let mut wt_operation_record = wind_tunnel_instruments::OperationRecord::new(#target_name.to_string());
        },
    );

    if let Some(pre_hook) = pre_hook {
        let args = input
            .sig
            .inputs
            .iter()
            .map(|arg| {
                match arg {
                    syn::FnArg::Receiver(_) => {
                        // Ignore the self arg
                        Ok(None)
                    }
                    syn::FnArg::Typed(pat_type) => match *pat_type.pat.clone() {
                        syn::Pat::Ident(pat_ident) => Ok(Some(pat_ident.ident.clone())),
                        _ => Err(syn::Error::new(arg.span(), "Unsupported pattern type")),
                    },
                }
            })
            .collect::<syn::Result<Vec<Option<_>>>>()
            .unwrap()
            .into_iter()
            .flatten()
            .fold(
                Punctuated::<Expr, syn::token::Comma>::new(),
                |mut punct, arg| {
                    let span = arg.span();
                    punct.push(parse_quote!(&#arg));
                    punct.push_punct(Token![,](span));

                    punct
                },
            );

        input.block.stmts.insert(
            1,
            syn::parse_quote! {
                #pre_hook(&mut wt_operation_record, #args);
            },
        );
    }

    let result = input.block.stmts.pop().unwrap();
    input.block.stmts.push(syn::parse_quote! {
        let wt_result = #result;
    });
    input.block.stmts.push(syn::parse_quote! {
        wind_tunnel_instruments::report_operation(self.reporter.clone(), wt_operation_record, &wt_result);
    });
    input.block.stmts.push(syn::parse_quote! {
        return wt_result;
    });

    // Hand the resulting function body back to the compiler.
    TokenStream::from(quote!(#input))
}
