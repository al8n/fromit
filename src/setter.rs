use super::*;

#[derive(Default, FromMeta)]
pub(crate) struct FieldLevelSetter {
  rename: Option<syn::Ident>,
  style: Option<SetterStyle>,
  #[darling(default, rename = "skip")]
  ignore: bool,
  vis: Option<syn::Visibility>,
  #[darling(default)]
  bound: FieldLevelBound,
}

#[derive(FromMeta)]
pub(crate) struct StructLevelSetter {
  pub(crate) prefix: Option<syn::Ident>,
  #[darling(default)]
  pub(crate) style: SetterStyle,
  #[darling(default, rename = "skip")]
  pub(crate) ignore: bool,
  pub(crate) vis_all: Option<syn::Visibility>,
}

impl Default for StructLevelSetter {
  fn default() -> Self {
    Self {
      prefix: None,
      style: SetterStyle::Move,
      ignore: false,
      vis_all: None,
    }
  }
}

#[derive(Default, FromMeta, Clone, Copy)]
pub(crate) enum SetterStyle {
  Ref,
  #[default]
  Move,
  Into,
  #[darling(rename = "try_into")]
  TryInto,
}

impl SetterStyle {
  pub(crate) fn to_setter(
    &self,
    fn_vis: &syn::Visibility,
    bound: Option<&syn::Generics>,
    field_name: &syn::Ident,
    field_ty: &syn::Type,
    fn_name: &syn::Ident,
  ) -> proc_macro2::TokenStream {
    match self {
      Self::Ref => quote! {
        #fn_vis fn #fn_name #bound (&mut self, val: #field_ty) {
          self.#field_name = val;
        }
      },
      Self::Move => quote! {
        #fn_vis fn #fn_name #bound (mut self, val: #field_ty) -> Self {
          self.#field_name = val;
          self
        }
      },
      Self::Into => quote! {
        #fn_vis fn #fn_name #bound (mut self, val: impl core::convert::Into<#field_ty>) -> Self {
          self.#field_name = ::core::convert::Into::into(val);
          self
        }
      },
      Self::TryInto => {
        let bound = bound.map(|tt| {
          let bound = format!(
            "{}, Error>",
            tt.to_token_stream().to_string().trim_end_matches('>')
          );
          syn::parse_str::<syn::Generics>(&bound).unwrap()
        });
        quote! {
          #fn_vis fn #fn_name #bound (mut self, val: impl ::core::convert::TryInto<#field_ty, Error = Error>) -> ::core::result::Result<Self, Error> {
            self.#field_name = ::core::convert::TryInto::try_into(val)?;
            ::core::result::Result::Ok(self)
          }
        }
      }
    }
  }
}

pub(crate) fn generate_setters(
  opts: &StructOpts,
  final_generics: &FinalGenerics,
) -> syn::Result<proc_macro2::TokenStream> {
  if opts.setters.ignore {
    return Ok(quote!());
  }
  let mut setters = Vec::new();
  let mut extra_setters = Vec::new();
  let setters_prefix = opts
    .setters
    .prefix
    .as_ref()
    .cloned()
    .unwrap_or_else(|| format_ident!("set"));

  let mut ctr = 0;
  if let Some(extra) = &opts.extra {
    for field in extra.fields.values() {
      if field.setter.ignore {
        continue;
      }

      let src_name = field
        .name
        .clone()
        .unwrap_or_else(|| format_ident!("{}", ctr));
      ctr += usize::from(field.named);

      let field_name = field.name.as_ref().unwrap_or(&src_name);
      let vis = field.setter.vis.as_ref().unwrap_or_else(|| {
        opts
          .setters
          .vis_all
          .as_ref()
          .unwrap_or_else(|| field.vis.as_ref().unwrap_or(&opts.vis))
      });
      let fn_name = field
        .setter
        .rename
        .clone()
        .unwrap_or_else(|| format_ident!("{}_{}", setters_prefix, field_name));

      let field_ty = &field.src_ty;
      extra_setters.push(field.setter.style.unwrap_or(opts.setters.style).to_setter(
        vis,
        field.setter.bound.bound.as_ref(),
        field_name,
        field_ty,
        &fn_name,
      ));
    }
  }

  for (src_name, field) in opts.fields.iter() {
    if field.skip.is_some() || field.setter.ignore {
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
    let fn_name = field
      .setter
      .rename
      .clone()
      .unwrap_or_else(|| format_ident!("{}_{}", setters_prefix, field_name));

    let field_ty = field.typ.as_ref().unwrap_or(&field.src_ty);
    setters.push(field.setter.style.unwrap_or(opts.setters.style).to_setter(
      vis,
      field.setter.bound.bound.as_ref(),
      field_name,
      field_ty,
      &fn_name,
    ));
  }

  let name = &opts.name;
  let impl_generics = &final_generics.impl_generics;
  let self_ty_generics = &final_generics.ty_generics;
  let where_clause = &final_generics.where_clause;
  Ok(quote! {
      impl #impl_generics #name #self_ty_generics #where_clause {

          #(#setters)*

          #(#extra_setters)*
      }
  })
}
