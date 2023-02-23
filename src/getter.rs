use super::*;

#[derive(Default, FromMeta)]
pub(crate) struct FieldLevelGetter {
  pub(crate) rename: Option<syn::Ident>,
  pub(crate) style: Option<Style>,
  #[darling(default, rename = "skip")]
  pub(crate) ignore: bool,
  pub(crate) vis: Option<syn::Visibility>,
  pub(crate) result: Option<AccessorConverter>,
}

#[derive(FromMeta)]
pub(crate) struct StructLevelGetter {
  pub(crate) prefix: Option<syn::Ident>,
  #[darling(default)]
  pub(crate) style: Style,
  #[darling(default, rename = "skip")]
  pub(crate) ignore: bool,
  pub(crate) vis_all: Option<syn::Visibility>,
}

impl Default for StructLevelGetter {
  fn default() -> Self {
    Self {
      prefix: None,
      style: Style::Ref,
      ignore: false,
      vis_all: None,
    }
  }
}

#[derive(Default)]
pub(crate) struct FieldLevelBound {
  pub(crate) bound: Option<syn::Generics>,
}

impl darling::FromMeta for FieldLevelBound {
  fn from_value(value: &syn::Lit) -> darling::Result<Self> {
    if let syn::Lit::Str(ref s) = value {
      if s.value().is_empty() {
        Ok(Self { bound: None })
      } else {
        let tt = format!("<{}>", s.value());
        let bound = syn::parse_str::<syn::Generics>(&tt)?;
        Ok(Self { bound: Some(bound) })
      }
    } else {
      Err(darling::Error::custom("expected str literal").with_span(value))
    }
  }
}

#[derive(FromMeta)]
pub(crate) struct AccessorConverter {
  #[darling(rename = "type")]
  pub(crate) ty: Option<syn::Type>,
  pub(crate) converter: FieldConverter,
  #[darling(default)]
  pub(crate) bound: FieldLevelBound,
}

impl AccessorConverter {
  pub(crate) fn to_getter(
    &self,
    field_name: &syn::Ident,
    field_ty: &syn::Type,
    style: Style,
    vis: &syn::Visibility,
    fn_name: &syn::Ident,
  ) -> proc_macro2::TokenStream {
    let field_ty = self.ty.as_ref().unwrap_or(field_ty);
    let bound = self.bound.bound.as_ref();
    let result = match self.converter.style.unwrap_or(style) {
      Style::Ref => match &self.converter.func {
        Some(conv) => quote! {
          #conv(&self.#field_name)
        },
        None => quote! {
          &self.#field_name
        },
      },
      Style::Move => match &self.converter.func {
        Some(conv) => quote! {
          #conv(self.#field_name)
        },
        None => quote! {
          self.#field_name
        },
      },
    };
    match style {
      Style::Ref => quote! {
        #[inline]
        #vis fn #fn_name #bound (&self) -> #field_ty {
          #result
        }
      },
      Style::Move => quote! {
        #[inline]
        #vis fn #fn_name #bound (self) ->  {
          #result
        }
      },
    }
  }
}

pub(crate) fn generate_getters(
  opts: &StructOpts,
  final_generics: &FinalGenerics,
) -> syn::Result<proc_macro2::TokenStream> {
  if opts.getters.ignore {
    return Ok(quote!());
  }
  let mut ctr = 0;
  let mut getters = Vec::new();
  let mut extra_getters = Vec::new();
  if let Some(extra) = &opts.extra {
    for field in extra.fields.values() {
      if field.getter.ignore {
        continue;
      }

      let src_name = field
        .name
        .clone()
        .unwrap_or_else(|| format_ident!("{}", ctr));
      ctr += usize::from(field.named);

      let field_name = field.name.as_ref().unwrap_or(&src_name);
      let vis = field.getter.vis.as_ref().unwrap_or_else(|| {
        opts
          .getters
          .vis_all
          .as_ref()
          .unwrap_or_else(|| field.vis.as_ref().unwrap_or(&opts.vis))
      });
      let fn_name = field.getter.rename.clone().unwrap_or_else(|| {
        if let Some(p) = &opts.getters.prefix {
          format_ident!("{}_{}", p, field_name)
        } else {
          field_name.clone()
        }
      });

      let style = field.getter.style.unwrap_or(opts.getters.style);
      let field_ty = &field.src_ty;
      match &field.getter.result {
        Some(ac) => {
          extra_getters.push(ac.to_getter(field_name, field_ty, style, vis, &fn_name));
        }
        None => {
          extra_getters.push(quote! {
            #[inline]
            #vis fn #fn_name(&self) -> #style #field_ty {
              #style self.#field_name
            }
          });
        }
      }
    }
  }

  for (src_name, field) in opts.fields.iter() {
    if field.skip.is_some() || field.getter.ignore {
      continue;
    }

    let src_name = field.rename.clone().unwrap_or_else(|| {
      if field.named {
        format_ident!("{}", src_name)
      } else {
        format_ident!("{}", ctr)
      }
    });
    ctr += usize::from(field.named);
    let field_name = field.rename.as_ref().unwrap_or(&src_name);
    let vis = field.getter.vis.as_ref().unwrap_or_else(|| {
      opts
        .getters
        .vis_all
        .as_ref()
        .unwrap_or_else(|| field.vis.as_ref().unwrap_or(&opts.vis))
    });
    let fn_name = field.getter.rename.clone().unwrap_or_else(|| {
      if let Some(p) = &opts.getters.prefix {
        format_ident!("{}_{}", p, field_name)
      } else {
        field_name.clone()
      }
    });

    let style = field.getter.style.unwrap_or(opts.getters.style);
    let field_ty = field.typ.as_ref().unwrap_or(&field.src_ty);
    match &field.getter.result {
      Some(ac) => {
        getters.push(ac.to_getter(field_name, field_ty, style, vis, &fn_name));
      }
      None => {
        getters.push(quote! {
          #[inline]
          #vis fn #fn_name(&self) -> #style #field_ty {
            #style self.#field_name
          }
        });
      }
    }
  }

  let name = &opts.name;
  let impl_generics = &final_generics.impl_generics;
  let self_ty_generics = &final_generics.ty_generics;
  let where_clause = &final_generics.where_clause;
  Ok(quote! {
      impl #impl_generics #name #self_ty_generics #where_clause {

          #(#getters)*

          #(#extra_getters)*
      }
  })
}
