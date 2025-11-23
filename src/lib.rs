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

fn parse_report_config(input: ParseStream) -> syn::Result<proc_macro2::TokenStream> {
    let mut kind = quote! { ariadne::ReportKind::Error };
    let mut config = quote! { None };
    let mut code = quote! { None };

    loop {
        let key = input.parse();
        if key.is_err() {
            break;
        }
        let key: Ident = key.unwrap();
        input.parse::<syn::Token![=]>()?;
        let value: Expr = input.parse()?;

        if key == "kind" {
            kind = quote! { #value };
        } else if key == "config" {
            config = quote! { Some(#value) };
        } else if key == "code" {
            code = quote! { Some(#value) };
        }

        if input.parse::<syn::Token![,]>().is_err() {
            break;
        }
    }

    Ok(quote! { (#kind, #config, #code) })
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
            #[allow(unused_variables, unused_assignments)]
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
            #[allow(unused_variables, unused_assignments)]
            match self {
                #(#arms,)*
                _ => None
            }
        }
    };

    let match_report = {
        let arms = enum_data.variants.iter().map(|variant| {
            let variant_ident = variant.ident.clone();

            let mut report_tuple = quote! { (ariadne::ReportKind::Error, None, None) };

            for attr in &variant.attrs {
                if !attr.path().is_ident("report") {
                    continue;
                }

                report_tuple = attr.parse_args_with(parse_report_config)?;
            }

            Ok(match &variant.fields {
                syn::Fields::Named(_) => quote! {
                    Self :: #variant_ident { .. } => #report_tuple
                },
                syn::Fields::Unnamed(_) => quote! {
                    Self :: #variant_ident ( .. ) => #report_tuple
                },
                syn::Fields::Unit => quote! {
                    Self :: #variant_ident => #report_tuple
                },
            })
        });

        let arms: Result<Vec<_>, syn::Error> = arms.collect();
        let arms = match arms {
            Ok(arms) => arms,
            Err(e) => return e.to_compile_error().into(),
        };

        quote! {
            pub fn report_tuple(&self) -> (
                ariadne::ReportKind<'static>,
                Option<ariadne::Config>,
                Option<usize>
            ) {
                match self {
                    #(#arms,)*
                }
            }
        }
    };

    let match_error_location = {
        let arms = enum_data.variants.iter().map(|variant| {
            let variant_ident = variant.ident.clone();
            match &variant.fields {
                syn::Fields::Named(fields) => {
                    for field in &fields.named {
                        if has_attr(&field.attrs, "here") {
                            let arg = field.ident.clone().unwrap();
                            return Ok(quote! {
                                Self :: #variant_ident { #arg, .. } => {
                                    // Type check that gets shows on the field
                                    let r : &std::ops::Range<usize> = #arg;
                                    r.clone()
                                },
                            });
                        }
                    }
                }
                syn::Fields::Unnamed(fields) => {
                    let mut found = 0;
                    let patterns = fields
                        .unnamed
                        .iter()
                        .map(|f| {
                            if has_attr(&f.attrs, "here") {
                                found += 1;
                                quote! { span, }
                            } else {
                                quote! { _, }
                            }
                        })
                        .collect::<Vec<_>>();

                    if found == 1 {
                        return Ok(quote! {
                            Self :: #variant_ident ( #(#patterns)* ) => {
                                let r : &std::ops::Range<usize> = span;
                                r.clone()
                            },
                        });
                    } else if found > 1 {
                        return Err(syn::Error::new_spanned(
                            variant_ident,
                            "Multiple #[here] attributes found in this variant",
                        ));
                    }
                }
                syn::Fields::Unit => (),
            }

            Err(syn::Error::new_spanned(
                variant_ident,
                "Missing error location via the #[here] attribute",
            ))
        });

        let arms: Result<Vec<_>, _> = arms.collect();
        let arms = match arms {
            Ok(arms) => arms,
            Err(e) => return e.to_compile_error().into(),
        };

        quote! {
            match self {
                #(#arms)*
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
            #[allow(unused_variables, unused_assignments)]
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

            pub fn error_location(&self) -> std::ops::Range<usize> {
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
                self.report_builder(filename).finish().eprint((filename, source))
            }

            pub fn report_builder<'a>(&self, filename: &'a str) -> ariadne::ReportBuilder<'static, (&'a str, std::ops::Range<usize>)> {
                let (kind, config, code) = self.report_tuple();

                let mut builder = ariadne::Report::build(
                    kind,
                    (filename, self.error_location())
                );

                if let Some(msg) = self.message() {
                    builder = builder.with_message(msg);
                }

                if let Some(code) = code {
                    builder = builder.with_code(code);
                }

                if let Some(config) = config {
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
                    builder = builder.with_note(note);
                }

                builder
            }
        }
    }
    .into()
}
