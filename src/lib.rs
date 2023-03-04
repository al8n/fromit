#![doc = include_str!("../README.md")]
#![allow(clippy::wrong_self_convention)]

use std::collections::{hash_map::Entry, HashMap};

use darling::{FromDeriveInput, FromMeta, ToTokens};
use quote::{format_ident, quote};
use syn::{parse::Parser, parse_macro_input, Attribute};

mod parser;
mod setter;
use setter::*;
mod getter;
use getter::*;
mod into;
use into::*;
mod from;
use from::*;
mod field;
use field::*;
mod structure;
use structure::*;

#[derive(Default)]
struct Attributes {
  attrs: Vec<syn::Attribute>,
}

impl core::fmt::Debug for Attributes {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let attrs = &self.attrs;
    for attr in attrs.iter() {
      quote!(#attr).to_string().fmt(f)?;
    }
    Ok(())
  }
}

impl FromMeta for Attributes {
  fn from_list(items: &[syn::NestedMeta]) -> darling::Result<Self> {
    let mut attrs = Vec::with_capacity(items.len());
    for n in items {
      attrs.extend(Attribute::parse_outer.parse2(quote! { #[#n] })?);
    }
    Ok(Attributes { attrs })
  }
}

impl ToTokens for Attributes {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    for attr in self.attrs.iter() {
      tokens.extend(quote!(#attr));
    }
  }
}

#[allow(dead_code)]
struct FinalGenerics {
  src_generics: syn::Generics,
  src_impl_generics: proc_macro2::TokenStream,
  src_ty_generics: proc_macro2::TokenStream,
  src_where_clause: Option<syn::WhereClause>,
  generics: syn::Generics,
  impl_generics: proc_macro2::TokenStream,
  ty_generics: proc_macro2::TokenStream,
  where_clause: Option<proc_macro2::TokenStream>,
  final_generics: syn::Generics,
  final_impl_generics: proc_macro2::TokenStream,
  final_ty_generics: proc_macro2::TokenStream,
  final_struct_generics: proc_macro2::TokenStream,
  final_where_clause: Option<proc_macro2::TokenStream>,
}

fn get_final_generics(
  this: Option<&Bound>,
  src_generics: &syn::Generics,
) -> syn::Result<FinalGenerics> {
  let (src_impl_generics, src_ty_generics, src_where_clause) = src_generics.split_for_impl();
  match this {
    Some(b) => {
      let self_bound = match &b.extra {
        Some(bound) => match syn::parse_str::<syn::Generics>(&format!("<{}>", bound)) {
          Ok(g) => g,
          Err(e) => return Err(e),
        },
        None => syn::Generics::default(),
      };

      if b.inherit {
        let mut ts = src_generics
          .to_token_stream()
          .to_string()
          .trim()
          .to_string();
        if ts.is_empty() {
          let (impl_generics, ty_generics, where_clause) = self_bound.split_for_impl();
          Ok(FinalGenerics {
            src_generics: src_generics.clone(),
            src_impl_generics: src_impl_generics.to_token_stream(),
            src_ty_generics: src_ty_generics.to_token_stream(),
            src_where_clause: src_where_clause.cloned(),
            generics: self_bound.clone(),
            impl_generics: impl_generics.to_token_stream(),
            ty_generics: ty_generics.to_token_stream(),
            where_clause: where_clause.map(|w| quote!(#w)),
            final_impl_generics: impl_generics.to_token_stream(),
            final_ty_generics: ty_generics.to_token_stream(),
            final_where_clause: where_clause.map(|w| quote!(#w)),
            final_struct_generics: impl_generics.to_token_stream(),
            final_generics: self_bound,
          })
        } else {
          ts = ts.trim().trim_end_matches('>').trim().to_string();
          if !ts.ends_with(',') {
            ts.push(',');
          }
          ts.push_str(
            self_bound
              .to_token_stream()
              .to_string()
              .trim_start_matches('<'),
          );

          let g = match syn::parse_str::<syn::Generics>(&ts) {
            Ok(g) => g,
            Err(e) => return Err(e),
          };

          let (impl_generics, ty_generics, where_cluase) = g.split_for_impl();
          let w = match (where_cluase, src_where_clause) {
            (None, None) => quote!(),
            (None, Some(w)) => quote!(#w),
            (Some(w), None) => quote!(#w),
            (Some(sw), Some(src_w)) => {
              let mut w = src_w.to_token_stream().to_string();
              w.push_str(" + ");
              w.push_str(sw.to_token_stream().to_string().trim_start_matches("where"));
              quote!(#w)
            }
          };
          Ok(FinalGenerics {
            src_generics: src_generics.clone(),
            src_impl_generics: src_impl_generics.to_token_stream(),
            src_ty_generics: src_ty_generics.to_token_stream(),
            src_where_clause: src_where_clause.cloned(),
            generics: g.clone(),
            impl_generics: impl_generics.to_token_stream(),
            ty_generics: ty_generics.to_token_stream(),
            where_clause: Some(w.clone()),
            final_impl_generics: impl_generics.to_token_stream(),
            final_ty_generics: ty_generics.to_token_stream(),
            final_where_clause: Some(w),
            final_struct_generics: impl_generics.to_token_stream(),
            final_generics: g,
          })
        }
      } else {
        let (self_impl_generics, self_ty_generics, self_where_cluase) = self_bound.split_for_impl();
        let i = match (
          self_impl_generics
            .to_token_stream()
            .to_string()
            .trim()
            .is_empty(),
          src_impl_generics
            .to_token_stream()
            .to_string()
            .trim()
            .is_empty(),
        ) {
          (true, true) => quote!(),
          (true, false) => quote!(#src_impl_generics),
          (false, true) => quote!(#self_impl_generics),
          (false, false) => {
            let mut ts = src_impl_generics
              .to_token_stream()
              .to_string()
              .trim()
              .trim_end_matches('>')
              .to_string();
            if !ts.ends_with(',') {
              ts.push(',');
            }
            ts.push_str(
              self_impl_generics
                .to_token_stream()
                .to_string()
                .trim()
                .trim_start_matches('<'),
            );
            match syn::parse_str::<syn::Generics>(&ts) {
              Ok(g) => {
                let (ig, _, _) = g.split_for_impl();
                quote!(#ig)
              }
              Err(e) => return Err(e),
            }
          }
        };
        let w = match (self_where_cluase, src_where_clause) {
          (None, None) => quote!(),
          (None, Some(w)) => quote!(#w),
          (Some(w), None) => quote!(#w),
          (Some(sw), Some(src_w)) => {
            let mut w = src_w.to_token_stream().to_string();
            w.push_str(" + ");
            w.push_str(sw.to_token_stream().to_string().trim_start_matches("where"));
            quote!(#w)
          }
        };
        Ok(FinalGenerics {
          src_generics: src_generics.clone(),
          src_impl_generics: src_impl_generics.to_token_stream(),
          src_ty_generics: src_ty_generics.to_token_stream(),
          src_where_clause: src_where_clause.cloned(),
          generics: self_bound.clone(),
          impl_generics: self_impl_generics.to_token_stream(),
          ty_generics: self_ty_generics.to_token_stream(),
          where_clause: self_where_cluase.map(|w| quote!(#w)),
          final_impl_generics: i,
          final_ty_generics: self_ty_generics.to_token_stream(),
          final_where_clause: Some(w),
          final_struct_generics: self_impl_generics.to_token_stream(),
          final_generics: self_bound,
        })
      }
    }
    None => {
      let (impl_generics, src_ty_generics, where_clause) = src_generics.split_for_impl();
      Ok(FinalGenerics {
        src_generics: src_generics.clone(),
        src_impl_generics: src_impl_generics.to_token_stream(),
        src_ty_generics: src_ty_generics.to_token_stream(),
        src_where_clause: src_where_clause.cloned(),
        generics: Default::default(),
        impl_generics: Default::default(),
        ty_generics: Default::default(),
        where_clause: Default::default(),
        final_impl_generics: impl_generics.to_token_stream(),
        final_ty_generics: Default::default(),
        final_where_clause: where_clause.map(|w| quote!(#w)),
        final_generics: Default::default(),
        final_struct_generics: Default::default(),
      })
    }
  }
}

struct FromIt {
  name: syn::Ident,
  struct_opts: HashMap<String, StructOpts>,
  bound: syn::Generics,
}

impl FromDeriveInput for FromIt {
  fn from_derive_input(input: &syn::DeriveInput) -> darling::Result<Self> {
    let struct_check = ::darling::util::ShapeSet::new(<[_]>::into_vec(Box::new([
      darling::util::Shape::Named,
      darling::util::Shape::Tuple,
      darling::util::Shape::Newtype,
      darling::util::Shape::Unit,
    ])));

    match &input.data {
      syn::Data::Struct(data) => {
        let mut map = HashMap::new();
        for attr in input.attrs.iter() {
          let mut struct_name: (bool, Option<syn::Ident>) = (false, None);
          let mut vis: (bool, Option<syn::Visibility>) = (false, None);
          let mut bound: (bool, Option<Bound>) = (false, None);
          let mut attributes = (false, None);
          let mut getters = (false, None);
          let mut setters = (false, None);
          let mut converter = (false, None);
          let mut extra = (false, None);
          #[allow(clippy::single_match)]
          match ToString::to_string(&attr.path.clone().into_token_stream()).as_str() {
            "fromit" => match darling::util::parse_attribute_to_meta_list(attr) {
              Ok(data) => {
                if data.nested.is_empty() {
                  continue;
                }
                let items = &data.nested;
                for item in items {
                  match item {
                    syn::NestedMeta::Meta(inner) => {
                      let name = darling::util::path_to_string(inner.path());
                      match name.as_str() {
                        "name" => crate::parser::Parser::parse(&name, inner, &mut struct_name)?,
                        "extra" => crate::parser::Parser::parse(&name, inner, &mut extra)?,
                        "attributes" => {
                          crate::parser::Parser::parse(&name, inner, &mut attributes)?
                        }
                        "getters" => crate::parser::Parser::parse(&name, inner, &mut getters)?,
                        "setters" => crate::parser::Parser::parse(&name, inner, &mut setters)?,
                        "converter" => crate::parser::Parser::parse(&name, inner, &mut converter)?,
                        "vis" => crate::parser::Parser::parse(&name, inner, &mut vis)?,
                        "bound" => crate::parser::Parser::parse(&name, inner, &mut bound)?,
                        other => {
                          return Err(
                            darling::Error::unknown_field_with_alts(
                              other,
                              &[
                                "name",
                                "attributes",
                                "getters",
                                "setters",
                                "converter",
                                "vis",
                                "bound",
                                "extra",
                              ],
                            )
                            .with_span(inner),
                          );
                        }
                      }
                    }
                    syn::NestedMeta::Lit(inner) => {
                      return Err(darling::Error::unsupported_format("literal").with_span(inner))
                    }
                  }
                }
              }
              Err(e) => {
                return Err(e);
              }
            },
            _ => {}
          }
          if !struct_name.0 {
            return Err(darling::Error::missing_field("name").with_span(&attr));
          }
          let struct_name = struct_name.1.unwrap();

          map.insert(
            struct_name.to_string(),
            StructOpts {
              name: struct_name,
              vis: vis.1.unwrap_or_else(|| input.vis.clone()),
              bound: bound.1,
              getters: getters.1.unwrap_or_default(),
              setters: setters.1.unwrap_or_default(),
              converter: converter.1.unwrap_or_default(),
              attributes: attributes.1.unwrap_or_default(),
              fields: HashMap::new(),
              extra: extra.1,
            },
          );
        }

        for (idx, field) in data.fields.iter().enumerate() {
          let named = field.ident.is_some();
          for attr in &field.attrs {
            match ToString::to_string(&attr.path.clone().into_token_stream()).as_str() {
              "fromit" => {
                let mut skip: (bool, Option<FieldLevelSkip>) = (false, None);
                let mut typ: (bool, Option<syn::Type>) = (false, None);
                let mut vis: (bool, Option<syn::Visibility>) = (false, None);
                let mut rename: (bool, Option<syn::Ident>) = (false, None);
                let mut parent: (bool, Option<syn::Ident>) = (false, None);
                let mut from: (bool, Option<FieldConverter>) = (false, None);
                let mut into: (bool, Option<FieldConverter>) = (false, None);
                let mut attributes: (bool, Option<Attributes>) = (false, None);
                let mut getter: (bool, Option<FieldLevelGetter>) = (false, None);
                let mut setter: (bool, Option<FieldLevelSetter>) = (false, None);
                match ::darling::util::parse_attribute_to_meta_list(attr) {
                  Ok(data) => {
                    if data.nested.is_empty() {
                      continue;
                    }
                    let items = &data.nested;
                    for item in items {
                      match *item {
                        syn::NestedMeta::Meta(ref inner) => {
                          let name = ::darling::util::path_to_string(inner.path());
                          match name.as_str() {
                            "default" => {
                              return Err(
                                ::darling::Error::custom(
                                  "default is only supported for extra fields",
                                )
                                .with_span(inner),
                              )
                            }
                            "skip" => crate::parser::Parser::parse(&name, inner, &mut skip)?,
                            "type" => crate::parser::Parser::parse(&name, inner, &mut typ)?,
                            "rename" => crate::parser::Parser::parse(&name, inner, &mut rename)?,
                            "parent" => crate::parser::Parser::parse(&name, inner, &mut parent)?,
                            "from" => crate::parser::Parser::parse(&name, inner, &mut from)?,
                            "into" => crate::parser::Parser::parse(&name, inner, &mut into)?,
                            "getter" => crate::parser::Parser::parse(&name, inner, &mut getter)?,
                            "setter" => crate::parser::Parser::parse(&name, inner, &mut setter)?,
                            "vis" => crate::parser::Parser::parse(&name, inner, &mut vis)?,
                            "attributes" => {
                              crate::parser::Parser::parse(&name, inner, &mut attributes)?
                            }
                            other => {
                              return Err(
                                ::darling::Error::unknown_field_with_alts(
                                  other,
                                  &[
                                    "skip",
                                    "type",
                                    "rename",
                                    "parent",
                                    "from",
                                    "into",
                                    "getter",
                                    "setter",
                                    "vis",
                                    "attributes",
                                  ],
                                )
                                .with_span(inner),
                              );
                            }
                          }
                        }
                        syn::NestedMeta::Lit(ref inner) => {
                          return Err(
                            ::darling::Error::unsupported_format("literal").with_span(inner),
                          );
                        }
                      }
                    }
                  }
                  Err(err) => {
                    return Err(err);
                  }
                }
                if map.len() > 1 && parent.1.is_none() {
                  return Err(::darling::Error::custom(
                                        "parent must be specified when there are more than one struct needed to be generated",
                                    ));
                }
                let f = Field {
                  src_ty: field.ty.clone(),
                  src_vis: field.vis.clone(),
                  skip: skip.1,
                  vis: vis.1,
                  typ: typ.1,
                  rename: rename.1,
                  parent: parent.1,
                  getter: getter.1.unwrap_or_default(),
                  setter: setter.1.unwrap_or_default(),
                  from: from.1,
                  into: into.1,
                  attributes: attributes.1.unwrap_or_default(),
                  named,
                };

                let field_name = field
                  .ident
                  .as_ref()
                  .map(ToString::to_string)
                  .unwrap_or_else(|| idx.to_string());
                if map.len() > 1 {
                  let parent = f.parent.as_ref().unwrap().to_string();
                  match map.get_mut(&parent) {
                    Some(s) => match s.fields.entry(field_name) {
                      Entry::Occupied(mut old_f) => {
                        old_f.get_mut().merge(f);
                      }
                      Entry::Vacant(val) => {
                        val.insert(f);
                      }
                    },
                    None => {
                      return Err(darling::Error::custom(format!(
                        "Does not have parent {}",
                        parent
                      )))
                    }
                  }
                } else {
                  match map.iter_mut().next().unwrap().1.fields.entry(field_name) {
                    Entry::Occupied(mut old_f) => {
                      old_f.get_mut().merge(f);
                    }
                    Entry::Vacant(val) => {
                      val.insert(f);
                    }
                  }
                }
              }
              _ => continue,
            }
          }
          let key = field
            .ident
            .clone()
            .unwrap_or_else(|| format_ident!("{idx}"))
            .to_string();
          for (parent, s) in map.iter_mut() {
            if !s.fields.contains_key(&key) {
              s.fields.insert(
                key.clone(),
                Field {
                  src_ty: field.ty.clone(),
                  src_vis: field.vis.clone(),
                  skip: None,
                  vis: None,
                  typ: None,
                  rename: None,
                  parent: Some(format_ident!("{}", parent)),
                  getter: Default::default(),
                  setter: Default::default(),
                  from: None,
                  into: None,
                  attributes: Default::default(),
                  named,
                },
              );
            }
          }
        }

        Ok(FromIt {
          name: input.ident.clone(),
          struct_opts: map,
          bound: input.generics.clone(),
        })
      }
      syn::Data::Enum(_) => Err(darling::Error::unsupported_shape_with_expected("enum", &{
        let res = format!("struct with {}", struct_check);
        res
      })),
      syn::Data::Union(_) => Err(darling::Error::unsupported_shape_with_expected("union", &{
        let res = format!("struct with {}", struct_check);
        res
      })),
    }
  }
}

#[proc_macro_derive(FromIt, attributes(fromit))]
pub fn from_it(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  let fromit = match FromIt::from_derive_input(&input) {
    Ok(fromit) => fromit,
    Err(e) => return e.write_errors().into(),
  };

  let mut streams = Vec::new();
  let src_name = &fromit.name;
  let src_generics = &fromit.bound;
  for (name, opts) in fromit.struct_opts {
    let name = format_ident!("{}", name);
    let final_generics = match get_final_generics(opts.bound.as_ref(), src_generics) {
      Ok(g) => g,
      Err(e) => return e.to_compile_error().into(),
    };
    streams.push(match generate_struct(&name, &opts, &final_generics) {
      Ok(s) => s,
      Err(e) => return e.to_compile_error().into(),
    });

    streams.push(match generate_from(src_name, &opts, &final_generics) {
      Ok(s) => s,
      Err(e) => return e.to_compile_error().into(),
    });

    streams.push(match generate_into(src_name, &opts, &final_generics) {
      Ok(s) => s,
      Err(e) => return e.to_compile_error().into(),
    });

    streams.push(match generate_getters(&opts, &final_generics) {
      Ok(s) => s,
      Err(e) => return e.to_compile_error().into(),
    });

    streams.push(match generate_setters(&opts, &final_generics) {
      Ok(s) => s,
      Err(e) => return e.to_compile_error().into(),
    });
  }
  quote! {
      #(#streams)*
  }
  .into()
}
