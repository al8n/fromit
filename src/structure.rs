use super::*;

#[derive(Default, FromMeta, Clone)]
pub(crate) struct Try {
  #[darling(default)]
  pub(crate) style: Style,
  pub(crate) error: Option<syn::Type>,
}

#[derive(FromMeta)]
pub(crate) struct Converter {
  pub(crate) try_from: Option<Try>,
  pub(crate) try_into: Option<Try>,
  pub(crate) from: Option<Style>,
  pub(crate) into: Option<Style>,
}

impl Default for Converter {
  fn default() -> Self {
    Self {
      try_from: None,
      try_into: None,
      from: Some(Style::default()),
      into: Some(Style::default()),
    }
  }
}

pub(crate) struct Bound {
  pub(crate) inherit: bool,
  pub(crate) extra: Option<String>,
}

impl FromMeta for Bound {
  fn from_list(items: &[syn::NestedMeta]) -> ::darling::Result<Self> {
    let mut inherit: (bool, Option<bool>) = (false, None);
    let mut extra: (bool, Option<String>) = (false, None);
    let mut errors = ::darling::Error::accumulator();
    for item in items {
      match *item {
        syn::NestedMeta::Meta(ref inner) => {
          let name = ::darling::util::path_to_string(inner.path());
          match name.as_str() {
            "inherit" => {
              if !inherit.0 {
                match inner {
                  syn::Meta::Path(_) => inherit = (true, Some(true)),
                  syn::Meta::List(_) => {
                    return Err(::darling::Error::unsupported_format("list").with_span(inner))
                  }
                  syn::Meta::NameValue(v) => {
                    inherit = (true, Some(<bool as darling::FromMeta>::from_value(&v.lit)?));
                  }
                }
              } else {
                errors.push(::darling::Error::duplicate_field("inherit").with_span(&inner));
              }
            }
            "extra" => {
              if !extra.0 {
                extra = (
                  true,
                  errors.handle(
                    ::darling::FromMeta::from_meta(inner)
                      .map_err(|e| e.with_span(&inner).at("extra")),
                  ),
                );
              } else {
                errors.push(::darling::Error::duplicate_field("extra").with_span(&inner));
              }
            }
            other => {
              errors.push(
                ::darling::Error::unknown_field_with_alts(other, &["inherit", "extra"])
                  .with_span(inner),
              );
            }
          }
        }
        syn::NestedMeta::Lit(ref inner) => {
          errors.push(::darling::Error::unsupported_format("literal").with_span(inner));
        }
      }
    }

    errors.finish()?;
    Ok(Self {
      inherit: inherit.1.unwrap_or(false),
      extra: extra.1,
    })
  }
}

pub(crate) struct Extra {
  pub(crate) attributes: Attributes,
  pub(crate) fields: HashMap<String, ExtraField>,
}

impl darling::FromMeta for Extra {
  fn from_list(items: &[syn::NestedMeta]) -> darling::Result<Self> {
    let mut attributes = (false, None);
    let mut extra_fields = (false, None);
    for item in items {
      match item {
        syn::NestedMeta::Meta(ref inner) => {
          let name = ::darling::util::path_to_string(inner.path());
          match name.as_str() {
            "field_attributes" => crate::parser::Parser::parse(&name, inner, &mut attributes)?,
            "fields" => {
              if !extra_fields.0 {
                match inner {
                  syn::Meta::Path(_) => {
                    return Err(darling::Error::unsupported_format("path").with_span(inner))?;
                  }
                  syn::Meta::List(l) => {
                    extra_fields = (
                      true,
                      Some(crate::parser::Parser::parse_fields(
                        &inner,
                        &l.nested.to_token_stream().to_string(),
                      )?),
                    );
                  }
                  syn::Meta::NameValue(val) => {
                    if let syn::Lit::Str(val) = &val.lit {
                      extra_fields = (
                        true,
                        Some(crate::parser::Parser::parse_fields(&inner, &val.value())?),
                      );
                    } else {
                      return Err(
                        darling::Error::custom("expected string literal").with_span(&inner),
                      )?;
                    }
                  }
                }
              } else {
                return Err(darling::Error::duplicate_field("fields").with_span(&inner))?;
              }
            }
            other => {
              return Err(
                ::darling::Error::unknown_field_with_alts(other, &["field_attributes", "fields"])
                  .with_span(inner),
              );
            }
          }
        }
        syn::NestedMeta::Lit(inner) => {
          return Err(darling::Error::unsupported_format("literal").with_span(inner));
        }
      }
    }

    Ok(Self {
      attributes: attributes.1.unwrap_or_default(),
      fields: if !extra_fields.0 {
        HashMap::new()
      } else {
        let mut fields = HashMap::new();
        let (named, extra_fields) = extra_fields.1.unwrap();
        for (idx, field) in extra_fields.into_iter().enumerate() {
          let key = field
            .ident
            .clone()
            .unwrap_or_else(|| format_ident!("{idx}"))
            .to_string();
          for attr in &field.attrs {
            match ToString::to_string(&attr.path.clone().into_token_stream()).as_str() {
              "fromit" => {
                let mut vis: (bool, Option<syn::Visibility>) = (false, None);
                let mut default: (bool, Option<syn::Path>) = (false, None);
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
                            "default" => crate::parser::Parser::parse(&name, inner, &mut default)?,
                            "getter" => crate::parser::Parser::parse(&name, inner, &mut getter)?,
                            "setter" => crate::parser::Parser::parse(&name, inner, &mut setter)?,
                            "vis" => crate::parser::Parser::parse(&name, inner, &mut vis)?,
                            "attributes" => {
                              crate::parser::Parser::parse(&name, inner, &mut attributes)?
                            }
                            "skip" => {
                              return Err(
                                ::darling::Error::custom("skip is not supported for extra field")
                                  .with_span(inner),
                              )
                            }
                            "type" => {
                              return Err(
                                ::darling::Error::custom("type is not supported for extra field")
                                  .with_span(inner),
                              )
                            }
                            "rename" => {
                              return Err(
                                ::darling::Error::custom("rename is not supported for extra field")
                                  .with_span(inner),
                              )
                            }
                            "parent" => {
                              return Err(
                                ::darling::Error::custom("parent is not supported for extra field")
                                  .with_span(inner),
                              )
                            }
                            "from" => {
                              return Err(
                                ::darling::Error::custom("from is not supported for extra field")
                                  .with_span(inner),
                              )
                            }
                            "into" => {
                              return Err(
                                ::darling::Error::custom("into is not supported for extra field")
                                  .with_span(inner),
                              )
                            }
                            other => {
                              return Err(
                                ::darling::Error::unknown_field_with_alts(
                                  other,
                                  &["default", "getter", "setter", "vis", "attributes"],
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

                let f = ExtraField {
                  src_ty: field.ty.clone(),
                  src_vis: field.vis.clone(),
                  vis: vis.1,
                  getter: getter.1.unwrap_or_default(),
                  setter: setter.1.unwrap_or_default(),
                  attributes: attributes.1.unwrap_or_default(),
                  default: default.1,
                  named,
                  name: field.ident.clone(),
                };
                fields.insert(key.clone(), f);
              }
              _ => continue,
            }
          }

          if let std::collections::hash_map::Entry::Vacant(e) = fields.entry(key) {
            e.insert(ExtraField {
              name: field.ident.clone(),
              src_ty: field.ty.clone(),
              src_vis: field.vis.clone(),
              vis: None,
              getter: FieldLevelGetter::default(),
              setter: FieldLevelSetter::default(),
              default: None,
              attributes: Attributes::default(),
              named,
            });
          }
        }
        fields
      },
    })
  }
}

pub(crate) struct StructOpts {
  pub(crate) name: syn::Ident,
  pub(crate) vis: syn::Visibility,
  pub(crate) bound: Option<Bound>,
  pub(crate) getters: StructLevelGetter,
  pub(crate) setters: StructLevelSetter,
  pub(crate) converter: Converter,
  pub(crate) attributes: Attributes,
  pub(crate) fields: HashMap<String, Field>,
  pub(crate) extra: Option<Extra>,
}

pub(crate) fn generate_struct(
  name: &syn::Ident,
  opts: &StructOpts,
  final_generics: &FinalGenerics,
) -> syn::Result<proc_macro2::TokenStream> {
  let mut fields = Vec::new();
  let mut extra_fields = Vec::new();
  let mut extra_attributes = Vec::new();
  let mut ctr = 0;
  if let Some(extra) = &opts.extra {
    extra_attributes = extra.attributes.attrs.clone();
    for field in extra.fields.values() {
      let name = field
        .name
        .clone()
        .unwrap_or_else(|| format_ident!("{}", ctr));
      ctr += usize::from(field.named);
      let ty = &field.src_ty;
      let attributes = field.attributes.attrs.iter().chain(extra_attributes.iter());
      let vis = field.vis.as_ref().unwrap_or(&field.src_vis);

      extra_fields.push(quote! {
          #(#attributes)*
          #vis #name: #ty,
      });
    }
  }

  opts.fields.iter().for_each(|(src_name, field)| {
    let name = field.rename.clone().unwrap_or_else(|| {
      if field.named {
        format_ident!("{}", src_name)
      } else {
        format_ident!("{}", ctr)
      }
    });
    ctr += usize::from(field.named);
    let ty = field.typ.as_ref().unwrap_or(&field.src_ty);
    let skip = &field.skip;
    let attributes = field.attributes.attrs.iter().chain(extra_attributes.iter());
    let vis = field.vis.as_ref().unwrap_or(&field.src_vis);
    if skip.is_none() {
      fields.push(quote! {
          #(#attributes)*
          #vis #name: #ty,
      });
    }
  });

  let struct_attrs = &opts.attributes.attrs;
  let vis = &opts.vis;
  let final_struct_generics = &final_generics.final_struct_generics;
  let self_where_clause = &final_generics.where_clause;

  Ok(quote! {
     #(#struct_attrs)*
      #vis struct #name #final_struct_generics #self_where_clause {
          #(#extra_fields)*
          #(#fields)*
      }
  })
}
