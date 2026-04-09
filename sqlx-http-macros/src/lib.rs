use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields, LitStr,
};

// ---------------------------------------------------------------------------
// Attribute helpers
// ---------------------------------------------------------------------------

struct ContainerAttrs {
    rename_all: Option<String>,
    default: bool,
    transparent: bool,
}

struct FieldAttrs {
    rename: Option<String>,
    skip: bool,
    flatten: bool,
    default: bool,
}

fn parse_container_attrs(attrs: &[Attribute]) -> syn::Result<ContainerAttrs> {
    let mut result = ContainerAttrs {
        rename_all: None,
        default: false,
        transparent: false,
    };

    for attr in attrs {
        if !attr.path().is_ident("sqlx") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                result.rename_all = Some(lit.value());
            } else if meta.path.is_ident("default") {
                result.default = true;
            } else if meta.path.is_ident("transparent") {
                result.transparent = true;
            }
            Ok(())
        })?;
    }

    Ok(result)
}

fn parse_field_attrs(attrs: &[Attribute]) -> syn::Result<FieldAttrs> {
    let mut result = FieldAttrs {
        rename: None,
        skip: false,
        flatten: false,
        default: false,
    };

    for attr in attrs {
        if !attr.path().is_ident("sqlx") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                result.rename = Some(lit.value());
            } else if meta.path.is_ident("skip") {
                result.skip = true;
            } else if meta.path.is_ident("flatten") {
                result.flatten = true;
            } else if meta.path.is_ident("default") {
                result.default = true;
            }
            Ok(())
        })?;
    }

    Ok(result)
}

fn parse_variant_rename(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if !attr.path().is_ident("sqlx") {
            continue;
        }
        let mut rename = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                rename = Some(lit.value());
            }
            Ok(())
        })?;
        if rename.is_some() {
            return Ok(rename);
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// Rename helpers
// ---------------------------------------------------------------------------

fn apply_rename_all_field(rename_all: &Option<String>, name: &str) -> String {
    match rename_all.as_deref() {
        Some("lowercase") => name.to_lowercase(),
        Some("UPPERCASE") => name.to_uppercase(),
        Some("camelCase") => snake_to_camel(name),
        Some("PascalCase") => snake_to_pascal(name),
        Some("snake_case") => name.to_string(),
        Some("SCREAMING_SNAKE_CASE") => name.to_uppercase(),
        Some("kebab-case") => name.replace('_', "-"),
        _ => name.to_string(),
    }
}

fn apply_rename_all_variant(rename_all: &Option<String>, variant: &str) -> String {
    let snake = pascal_to_snake(variant);
    match rename_all.as_deref() {
        Some("lowercase") => variant.to_lowercase(),
        Some("UPPERCASE") => variant.to_uppercase(),
        Some("camelCase") => {
            let mut chars = variant.chars();
            match chars.next() {
                Some(c) => c.to_lowercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        }
        Some("PascalCase") => variant.to_string(),
        Some("snake_case") => snake,
        Some("SCREAMING_SNAKE_CASE") => snake.to_uppercase(),
        Some("kebab-case") => snake.replace('_', "-"),
        _ => variant.to_string(),
    }
}

fn snake_to_camel(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

fn snake_to_pascal(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

fn pascal_to_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.extend(c.to_lowercase());
    }
    result
}

// ---------------------------------------------------------------------------
// #[derive(FromRow)]
// ---------------------------------------------------------------------------

#[proc_macro_derive(FromRow, attributes(sqlx))]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_from_row(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_from_row(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let ty_generics = &input.generics;

    let container = parse_container_attrs(&input.attrs)?;

    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(named),
            ..
        }) => &named.named,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "FromRow can only be derived for structs with named fields",
            ));
        }
    };

    let mut field_extractions = Vec::new();
    let mut where_predicates: Vec<TokenStream2> = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let attrs = parse_field_attrs(&field.attrs)?;

        if attrs.skip {
            field_extractions.push(quote! {
                #field_name: ::std::default::Default::default()
            });
            continue;
        }

        if attrs.flatten {
            field_extractions.push(quote! {
                #field_name: ::sqlx_http::FromRow::from_row(row)?
            });
            where_predicates.push(quote! {
                #field_type: ::sqlx_http::FromRow<'r, R>
            });
            continue;
        }

        let col_name = if let Some(ref rename) = attrs.rename {
            rename.clone()
        } else {
            apply_rename_all_field(&container.rename_all, &field_name.to_string())
        };

        if container.default || attrs.default {
            field_extractions.push(quote! {
                #field_name: row.try_get(#col_name).or_else(|e| match e {
                    ::sqlx_http::Error::ColumnNotFound(_) => {
                        ::std::result::Result::Ok(::std::default::Default::default())
                    }
                    e => ::std::result::Result::Err(e),
                })?
            });
        } else {
            field_extractions.push(quote! {
                #field_name: row.try_get(#col_name)?
            });
        }

        where_predicates.push(quote! {
            #field_type: ::sqlx_http::Decode<'r, R::Database> + ::sqlx_http::Type<R::Database>
        });
    }

    where_predicates.push(quote! {
        &'r ::std::primitive::str: ::sqlx_http::ColumnIndex<R>
    });

    // Merge user-defined where clause
    let existing_where: Vec<TokenStream2> = input
        .generics
        .where_clause
        .iter()
        .flat_map(|w| w.predicates.iter().map(|p| p.to_token_stream()))
        .collect();

    let user_generics_params = &input.generics.params;
    let has_user_generics = !user_generics_params.is_empty();

    let expanded = if has_user_generics {
        quote! {
            #[automatically_derived]
            impl<'r, R: ::sqlx_http::Row, #user_generics_params> ::sqlx_http::FromRow<'r, R> for #name #ty_generics
            where
                #(#where_predicates,)*
                #(#existing_where),*
            {
                fn from_row(row: &'r R) -> ::sqlx_http::Result<Self> {
                    ::std::result::Result::Ok(#name {
                        #(#field_extractions),*
                    })
                }
            }
        }
    } else {
        quote! {
            #[automatically_derived]
            impl<'r, R: ::sqlx_http::Row> ::sqlx_http::FromRow<'r, R> for #name
            where
                #(#where_predicates),*
            {
                fn from_row(row: &'r R) -> ::sqlx_http::Result<Self> {
                    ::std::result::Result::Ok(#name {
                        #(#field_extractions),*
                    })
                }
            }
        }
    };

    Ok(expanded)
}

// ---------------------------------------------------------------------------
// #[derive(Type)]
// ---------------------------------------------------------------------------

#[proc_macro_derive(Type, attributes(sqlx))]
pub fn derive_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_type(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_type(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let container = parse_container_attrs(&input.attrs)?;

    if container.transparent {
        return expand_type_transparent(input);
    }

    match &input.data {
        Data::Enum(data) => expand_type_enum(name, &container, data),
        _ => Err(syn::Error::new_spanned(
            input,
            "Type can only be derived for enums or #[sqlx(transparent)] newtypes",
        )),
    }
}

fn expand_type_transparent(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let inner_type = get_transparent_inner_type(input)?;

    Ok(quote! {
        #[automatically_derived]
        impl ::sqlx_http::Type<::sqlx_http::HttpDb> for #name {
            fn type_info() -> ::sqlx_http::HttpTypeInfo {
                <#inner_type as ::sqlx_http::Type<::sqlx_http::HttpDb>>::type_info()
            }

            fn compatible(ty: &::sqlx_http::HttpTypeInfo) -> bool {
                <#inner_type as ::sqlx_http::Type<::sqlx_http::HttpDb>>::compatible(ty)
            }
        }
    })
}

fn expand_type_enum(
    name: &syn::Ident,
    _container: &ContainerAttrs,
    data: &DataEnum,
) -> syn::Result<TokenStream2> {
    for variant in &data.variants {
        if !variant.fields.is_empty() {
            return Err(syn::Error::new_spanned(
                variant,
                "Type derive for enums only supports unit variants",
            ));
        }
    }

    Ok(quote! {
        #[automatically_derived]
        impl ::sqlx_http::Type<::sqlx_http::HttpDb> for #name {
            fn type_info() -> ::sqlx_http::HttpTypeInfo {
                ::sqlx_http::HttpTypeInfo::Text
            }

            fn compatible(ty: &::sqlx_http::HttpTypeInfo) -> bool {
                <::std::string::String as ::sqlx_http::Type<::sqlx_http::HttpDb>>::compatible(ty)
            }
        }
    })
}

// ---------------------------------------------------------------------------
// #[derive(Encode)]
// ---------------------------------------------------------------------------

#[proc_macro_derive(Encode, attributes(sqlx))]
pub fn derive_encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_encode(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_encode(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let container = parse_container_attrs(&input.attrs)?;

    if container.transparent {
        return expand_encode_transparent(input);
    }

    match &input.data {
        Data::Enum(data) => expand_encode_enum(name, &container, data),
        _ => Err(syn::Error::new_spanned(
            input,
            "Encode can only be derived for enums or #[sqlx(transparent)] newtypes",
        )),
    }
}

fn expand_encode_transparent(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let inner_type = get_transparent_inner_type(input)?;

    Ok(quote! {
        #[automatically_derived]
        impl ::sqlx_http::Encode<'_, ::sqlx_http::HttpDb> for #name {
            fn encode_by_ref(
                &self,
                buf: &mut ::std::vec::Vec<::sqlx_http::__private::serde_json::Value>,
            ) -> ::std::result::Result<::sqlx_http::__private::encode::IsNull, ::sqlx_http::__private::error::BoxDynError>
            {
                <#inner_type as ::sqlx_http::Encode<::sqlx_http::HttpDb>>::encode_by_ref(&self.0, buf)
            }
        }
    })
}

fn expand_encode_enum(
    name: &syn::Ident,
    container: &ContainerAttrs,
    data: &DataEnum,
) -> syn::Result<TokenStream2> {
    let mut match_arms = Vec::new();

    for variant in &data.variants {
        if !variant.fields.is_empty() {
            return Err(syn::Error::new_spanned(
                variant,
                "Encode derive for enums only supports unit variants",
            ));
        }
        let variant_ident = &variant.ident;
        let string_name = if let Some(rename) = parse_variant_rename(&variant.attrs)? {
            rename
        } else {
            apply_rename_all_variant(&container.rename_all, &variant_ident.to_string())
        };
        match_arms.push(quote! {
            #name::#variant_ident => #string_name
        });
    }

    Ok(quote! {
        #[automatically_derived]
        impl ::sqlx_http::Encode<'_, ::sqlx_http::HttpDb> for #name {
            fn encode_by_ref(
                &self,
                buf: &mut ::std::vec::Vec<::sqlx_http::__private::serde_json::Value>,
            ) -> ::std::result::Result<::sqlx_http::__private::encode::IsNull, ::sqlx_http::__private::error::BoxDynError>
            {
                let s: &str = match self {
                    #(#match_arms),*
                };
                <&str as ::sqlx_http::Encode<::sqlx_http::HttpDb>>::encode(s, buf)
            }
        }
    })
}

// ---------------------------------------------------------------------------
// #[derive(Decode)]
// ---------------------------------------------------------------------------

#[proc_macro_derive(Decode, attributes(sqlx))]
pub fn derive_decode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_decode(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_decode(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let container = parse_container_attrs(&input.attrs)?;

    if container.transparent {
        return expand_decode_transparent(input);
    }

    match &input.data {
        Data::Enum(data) => expand_decode_enum(name, &container, data),
        _ => Err(syn::Error::new_spanned(
            input,
            "Decode can only be derived for enums or #[sqlx(transparent)] newtypes",
        )),
    }
}

fn expand_decode_transparent(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let inner_type = get_transparent_inner_type(input)?;

    Ok(quote! {
        #[automatically_derived]
        impl<'r> ::sqlx_http::Decode<'r, ::sqlx_http::HttpDb> for #name {
            fn decode(
                value: ::sqlx_http::HttpValueRef<'r>,
            ) -> ::std::result::Result<Self, ::sqlx_http::__private::error::BoxDynError> {
                <#inner_type as ::sqlx_http::Decode<'r, ::sqlx_http::HttpDb>>::decode(value)
                    .map(#name)
            }
        }
    })
}

fn expand_decode_enum(
    name: &syn::Ident,
    container: &ContainerAttrs,
    data: &DataEnum,
) -> syn::Result<TokenStream2> {
    let mut match_arms = Vec::new();

    for variant in &data.variants {
        if !variant.fields.is_empty() {
            return Err(syn::Error::new_spanned(
                variant,
                "Decode derive for enums only supports unit variants",
            ));
        }
        let variant_ident = &variant.ident;
        let string_name = if let Some(rename) = parse_variant_rename(&variant.attrs)? {
            rename
        } else {
            apply_rename_all_variant(&container.rename_all, &variant_ident.to_string())
        };
        match_arms.push(quote! {
            #string_name => ::std::result::Result::Ok(#name::#variant_ident)
        });
    }

    let type_name_str = name.to_string();

    Ok(quote! {
        #[automatically_derived]
        impl<'r> ::sqlx_http::Decode<'r, ::sqlx_http::HttpDb> for #name {
            fn decode(
                value: ::sqlx_http::HttpValueRef<'r>,
            ) -> ::std::result::Result<Self, ::sqlx_http::__private::error::BoxDynError> {
                let s = <&str as ::sqlx_http::Decode<'r, ::sqlx_http::HttpDb>>::decode(value)?;
                match s {
                    #(#match_arms,)*
                    _ => ::std::result::Result::Err(
                        ::std::format!("unknown {} variant: {}", #type_name_str, s).into()
                    ),
                }
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_transparent_inner_type(input: &DeriveInput) -> syn::Result<&syn::Type> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(unnamed),
            ..
        }) if unnamed.unnamed.len() == 1 => Ok(&unnamed.unnamed.first().unwrap().ty),
        _ => Err(syn::Error::new_spanned(
            input,
            "#[sqlx(transparent)] requires a tuple struct with exactly one field",
        )),
    }
}
