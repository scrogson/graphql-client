use failure;
use heck::{CamelCase, SnakeCase};
use objects::GqlObjectField;
use proc_macro2::{Ident, Span, TokenStream};
use query::QueryContext;
use selection::*;

pub(crate) fn render_object_field(
    field_name: &str,
    field_type: &TokenStream,
    description: Option<&str>,
) -> TokenStream {
    let description = description.map(|s| quote!(#[doc = #s]));
    if field_name == "type" {
        let name_ident = Ident::new(&format!("{}_", field_name), Span::call_site());
        return quote! {
            #description
            #[serde(rename = #field_name)]
            pub #name_ident: #field_type
        };
    }

    let snake_case_name = field_name.to_snake_case();
    let rename = ::shared::field_rename_annotation(&field_name, &snake_case_name);
    let name_ident = Ident::new(&snake_case_name, Span::call_site());

    quote!(#description #rename pub #name_ident: #field_type)
}

pub(crate) fn field_impls_for_selection(
    fields: &[GqlObjectField],
    context: &QueryContext,
    selection: &Selection,
    prefix: &str,
) -> Result<Vec<TokenStream>, failure::Error> {
    selection
        .0
        .iter()
        .map(|selected| {
            if let SelectionItem::Field(selected) = selected {
                let ty = fields
                    .iter()
                    .find(|f| f.name == selected.name)
                    .ok_or_else(|| format_err!("could not find field `{}`", selected.name))?
                    .type_
                    .inner_name_string();
                let prefix = format!(
                    "{}{}",
                    prefix.to_camel_case(),
                    selected.name.to_camel_case()
                );
                context.maybe_expand_field(&ty, &selected.fields, &prefix)
            } else {
                Ok(quote!())
            }
        }).collect()
}

pub(crate) fn response_fields_for_selection(
    schema_fields: &[GqlObjectField],
    context: &QueryContext,
    selection: &Selection,
    prefix: &str,
) -> Result<Vec<TokenStream>, failure::Error> {
    selection
        .0
        .iter()
        .map(|item| match item {
            SelectionItem::Field(f) => {
                let name = &f.name;

                let schema_field = &schema_fields
                    .iter()
                    .find(|field| field.name.as_str() == name.as_str())
                    .ok_or_else(|| format_err!("Could not find field: {}", name.as_str()))?;
                let ty = schema_field.type_.to_rust(
                    context,
                    &format!("{}{}", prefix.to_camel_case(), name.to_camel_case()),
                );

                Ok(render_object_field(
                    name,
                    &ty,
                    schema_field.description.as_ref().map(|s| s.as_str()),
                ))
            }
            SelectionItem::FragmentSpread(fragment) => {
                let field_name =
                    Ident::new(&fragment.fragment_name.to_snake_case(), Span::call_site());
                let type_name = Ident::new(&fragment.fragment_name, Span::call_site());
                Ok(quote!{
                    #[serde(flatten)]
                    #field_name: #type_name
                })
            }
            SelectionItem::InlineFragment(_) => {
                Err(format_err!("inline fragment on object field"))?
            }
        }).collect()
}

/// Given the GraphQL schema name for an object/interface/input object field and
/// the equivalent rust name, produces a serde annotation to map them during
/// (de)serialization if it is necessary, otherwise an empty TokenStream.
pub(crate) fn field_rename_annotation(graphql_name: &str, rust_name: &str) -> TokenStream {
    if graphql_name != rust_name {
        quote!(#[serde(rename = #graphql_name)])
    } else {
        quote!()
    }
}
