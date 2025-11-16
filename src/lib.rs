use proc_macro::{TokenStream};
use quote::{format_ident, quote};
use syn::{parse::ParseStream, parse_macro_input, Attribute, Data, DeriveInput, Expr, Ident, LitStr};

fn has_attr(attrs: &[Attribute], target_str: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(target_str))
}

fn parse_label(input: ParseStream) -> syn::Result<(LitStr, Vec<Expr>)> {
    let fmt: LitStr = input.parse()?;
    let mut args = Vec::new();

    while input.parse::<syn::Token![,]>().is_ok() {
        let lit: Expr = input.parse()?;
        args.push(lit);
    }

    Ok((fmt, args))
}

fn parse_report_config(input: ParseStream) -> syn::Result<(proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream)> {
    let mut return_tuple = (
        quote! { Some(ariadne::ReportKind::Error) },
        quote! { None },
        quote! { None }
    );

    loop {
        let key = input.parse();
        if key.is_err() {
            break;
        }
        let key: Ident = key.unwrap();
        input.parse::<syn::Token![=]>()?;
        let value: Expr = input.parse()?;
    
        if key == "kind" {
            return_tuple.0 = quote! { Some(#value) };
        }
        else if key == "config" {
            return_tuple.1 = quote! { Some(#value) };
        }
        else if key == "code" {
            return_tuple.2 = quote! { Some(#value) };
        }

        if input.parse::<syn::Token![,]>().is_err() {
            break
        }
    }

    Ok(return_tuple)
}

#[proc_macro_derive(Ariadnenum, attributes(message, note, here, label, report, colored))]
pub fn derive_ariadnenum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident.clone();
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let enum_data = match input.data {
        Data::Enum(enum_data) => enum_data,
        _ => {
            return syn::Error::new_spanned(input, "Ariadnenum can only be derived for enums!")
                .to_compile_error()
                .into();
        }
    };

    let match_error_message = {
        let arms = enum_data.variants.iter().filter_map(|variant| {
            let variant_ident = variant.ident.clone();
            match &variant.fields {
                syn::Fields::Named(fields) => {
                    let mut args = Vec::new();
                    for field in &fields.named {
                        let ident = field.ident.clone().unwrap();
                        args.push(quote! { #ident });
                    }

                    for attr in &variant.attrs {
                        if !attr.path().is_ident("message") {
                            continue;
                        }

                        if let Ok((label, exprs)) = attr.parse_args_with(parse_label) {
                            return Some(quote! {
                                #enum_name :: #variant_ident { #(#args,)* } => Some(format!(#label, #(#exprs,)*))
                            });
                        }
                    }
                    None
                }
                syn::Fields::Unnamed(fields) => {
                    let mut args = Vec::new();
                    for i in 0..fields.unnamed.len() {
                        let ident = format_ident!("arg{}", i);
                        args.push(quote! { #ident });
                    }

                    for attr in &variant.attrs {
                        if !attr.path().is_ident("message") {
                            continue;
                        }

                        if let Ok((label, exprs)) = attr.parse_args_with(parse_label) {
                            return Some(quote! {
                                #enum_name :: #variant_ident (#(#args,)*) => Some(format!(#label, #(#exprs,)*))
                            });
                        }
                    }
                    None
                }
                syn::Fields::Unit => None,
            }
        });

        quote! {
            #[allow(unused)]
            match self {
                #(#arms,)*
                _ => None
            }
        }
    };

    let match_note = {
        let arms = enum_data.variants.iter().filter_map(|variant| {
            let variant_ident = variant.ident.clone();
            match &variant.fields {
                syn::Fields::Named(fields) => {
                    let mut args = Vec::new();
                    for field in &fields.named {
                        let ident = field.ident.clone().unwrap();
                        args.push(quote! { #ident });
                    }

                    for attr in &variant.attrs {
                        if !attr.path().is_ident("note") {
                            continue;
                        }

                        if let Ok((label, exprs)) = attr.parse_args_with(parse_label) {
                            return Some(quote! {
                                #enum_name :: #variant_ident { #(#args,)* } => Some(format!(#label, #(#exprs,)*))
                            });
                        }
                    }
                    None
                }
                syn::Fields::Unnamed(fields) => {
                    let mut args = Vec::new();
                    for i in 0..fields.unnamed.len() {
                        let ident = format_ident!("arg{}", i);
                        args.push(quote! { #ident });
                    }

                    for attr in &variant.attrs {
                        if !attr.path().is_ident("note") {
                            continue;
                        }

                        if let Ok((label, exprs)) = attr.parse_args_with(parse_label) {
                            return Some(quote! {
                                #enum_name :: #variant_ident (#(#args,)*) => Some(format!(#label, #(#exprs,)*))
                            });
                        }
                    }
                    None
                }
                syn::Fields::Unit => None,
            }
        });

        quote! {
            #[allow(unused)]
            match self {
                #(#arms,)*
                _ => None
            }
        }
    };

    let match_report = {
        let arms = enum_data.variants.iter().filter_map(|variant| {
            let variant_ident = variant.ident.clone();
            for attr in &variant.attrs {
                if !attr.path().is_ident("report") {
                    continue;
                }
                
                let expr = attr.parse_args_with(parse_report_config);
                if let Ok((kind, config, code)) = expr {
                    return Some(
                        (
                            match &variant.fields {
                                syn::Fields::Named(_) => quote! {
                                    #enum_name :: #variant_ident { .. } => #kind
                                },
                                syn::Fields::Unnamed(_) => quote! {
                                    #enum_name :: #variant_ident ( .. ) => #kind
                                },
                                syn::Fields::Unit => quote! {
                                    #enum_name :: #variant_ident => #kind
                                }
                            },
                            match &variant.fields {
                                syn::Fields::Named(_) => quote! {
                                    #enum_name :: #variant_ident { .. } => #config
                                },
                                syn::Fields::Unnamed(_) => quote! {
                                    #enum_name :: #variant_ident ( .. ) => #config
                                },
                                syn::Fields::Unit => quote! {
                                    #enum_name :: #variant_ident => #config
                                }
                            },
                            match &variant.fields {
                                syn::Fields::Named(_) => quote! {
                                    #enum_name :: #variant_ident { .. } => #code
                                },
                                syn::Fields::Unnamed(_) => quote! {
                                    #enum_name :: #variant_ident ( .. ) => #code
                                },
                                syn::Fields::Unit => quote! {
                                    #enum_name :: #variant_ident => #code
                                }
                            },
                        )
                    );
                }
            }
            None
        });

        let kinds = arms.clone().map(|t| t.0);
        let configs = arms.clone().map(|t| t.1);
        let codes = arms.map(|t| t.2);

        quote! {
            pub fn kind(&self) -> Option<ariadne::ReportKind> {
                match self {
                    #(#kinds,)*
                    _ => Some(ariadne::ReportKind::Error)
                }
            }
            
            pub fn config(&self) -> Option<ariadne::Config> {
                match self {
                    #(#configs,)*
                    _ => None
                }
            }
            
            pub fn code(&self) -> Option<usize> {
                match self {
                    #(#codes,)*
                    _ => None
                }
            }
        }
    };

    let match_error_location = {
        let arms = enum_data.variants.iter().filter_map(|variant| {
            let variant_ident = variant.ident.clone();
            match &variant.fields {
                syn::Fields::Named(fields) => {
                    for field in &fields.named {
                        if has_attr(&field.attrs, "here") {
                            let arg = field.ident.clone().unwrap();
                            return Some(quote! {
                                #enum_name :: #variant_ident { #arg, .. } => Some(#arg.clone()),
                            });
                        }
                    }
                    return None;
                },
                syn::Fields::Unnamed(fields) => {
                    let mut patterns = Vec::new();
                    let mut found = false;
                    for field in fields.unnamed.iter() {
                        if !found && has_attr(&field.attrs, "here") {
                            patterns.push(quote! { span, });
                            found = true;
                        } else {
                            patterns.push(quote! { _, });
                        };
                    };
                    if found {
                        Some(quote! {
                            #enum_name :: #variant_ident ( #(#patterns)* ) => Some(span.clone()),
                        })
                    } else {
                        None
                    }
                },
                syn::Fields::Unit => None,
            }
        });

        quote! {
            match self {
                #(#arms)*
                _ => None
            }
        }
    };

    let match_labels = {
        let arms = enum_data.variants.iter().filter_map(|variant| {
            let variant_ident = variant.ident.clone();
            match &variant.fields {
                syn::Fields::Named(fields) => {
                    let mut args = Vec::new();
                    let mut labels = Vec::new();
                    for field in &fields.named {
                        let ident = field.ident.clone().unwrap();
                        args.push(quote! { #ident });
                        let mut color = quote! { ariadne::Color::Red };
                        for attr in &field.attrs {
                            if attr.path().is_ident("colored") {
                                let expr: Result<Expr, syn::Error> = attr.parse_args();
                                if let Ok(expr) = expr {
                                    color = quote! { #expr };
                                }
                                continue;
                            } else if attr.path().is_ident("label") {
                                if let Ok((label, args)) = attr.parse_args_with(parse_label) {
                                    labels.push(quote! {
                                        (#color, format!(#label, #(#args,)*), #ident.clone()),
                                    });
                                }
                            }
                        }
                    }
                    Some(
                        quote! {
                            #enum_name :: #variant_ident { #(#args,)* } => vec![#(#labels)*]
                        }
                    )
                }
                syn::Fields::Unnamed(fields) => {
                    let mut args = Vec::new();
                    let mut labels = Vec::new();
                    for (i, field) in fields.unnamed.iter().enumerate() {
                        let ident = format_ident!("arg{}", i);
                        args.push(quote! { #ident });
                        let mut color = quote! { ariadne::Color::Red };
                        for attr in &field.attrs {
                            if attr.path().is_ident("colored") {
                                let expr: Result<Expr, syn::Error> = attr.parse_args();
                                if let Ok(expr) = expr {
                                    color = quote! { #expr };
                                }
                                continue;
                            } else if attr.path().is_ident("label") {
                                if let Ok((label, args)) = attr.parse_args_with(parse_label) {
                                    labels.push(quote! {
                                        (#color, format!(#label, #(#args,)*), #ident.clone()),
                                    });
                                }
                            }
                        }
                    }
                    Some(
                        quote! {
                            #enum_name :: #variant_ident ( #(#args,)* ) => vec![#(#labels)*]
                        }
                    )
                }
                syn::Fields::Unit => None,
            }
        });

        quote! {
            match self {
                #(#arms,)*
                _ => Vec::new()
            }
        }
    };

    quote! {
        #[automatically_derived]
        impl #impl_generics #enum_name #ty_generics #where_clause {
            #match_report

            pub fn error_location(&self) -> Option<std::ops::Range<usize>> {
                #match_error_location
            }

            pub fn message(&self) -> Option<String> {
                #match_error_message
            }
            
            pub fn note(&self) -> Option<String> {
                #match_note
            }


            pub fn labels(&self) -> Vec<(ariadne::Color, String, std::ops::Range<usize>)> {
                #match_labels
            }

            pub fn eprint_report(&self, filename: &str, source: ariadne::Source) -> Result<(), std::io::Error> {
                if self.error_location().is_none() || self.message().is_none() {
                    return Err((std::io::Error::new(std::io::ErrorKind::Other, "Missing location or message")));
                }

                let mut builder = ariadne::Report::build(
                    self.kind().unwrap(),
                    (filename, self.error_location().unwrap())
                )
                .with_message(self.message().unwrap());
                
                if let Some(code) = self.code() {
                    builder = builder.with_code(code);
                }

                if let Some(config) = self.config() {
                    builder = builder.with_config(config);
                }

                for (color, label, span) in self.labels() {
                    builder = builder.with_label(
                        ariadne::Label::new((filename, span))
                        .with_message(label)
                        .with_color(color)
                    );
                }

                if let Some(note) = self.note() {
                    builder = builder.with_note(self.note().unwrap());
                }

                builder.finish().eprint((filename, source))
            }
        }
    }
    .into()
}
