use super::*;

pub(super) fn generate_from(
  src_name: &syn::Ident,
  opts: &StructOpts,
  final_generics: &FinalGenerics,
) -> syn::Result<proc_macro2::TokenStream> {
  if opts.converter.try_from.is_none() && opts.converter.from.is_none() {
    return Ok(quote!());
  }
  let (try_, style, error) = match (&opts.converter.try_from, &opts.converter.from) {
    (None, None) => return Ok(quote!()),
    (None, Some(from)) => (false, *from, None),
    (Some(try_from), None) => (true, try_from.style, try_from.error.clone()),
    (Some(_), Some(_)) => {
      return Err(syn::Error::new_spanned(
        &opts.name,
        "Cannot have both `try_from` and `from`",
      ))
    }
  };

  let mut try_from_fields = Vec::new();
  let mut ctr = 0;
  if let Some(extra) = &opts.extra {
    for field in extra.fields.values() {
      let name = field
        .name
        .clone()
        .unwrap_or_else(|| format_ident!("{}", ctr));
      ctr += usize::from(field.named);
      let default = field
        .default
        .as_ref()
        .map(|d| quote!(#d()))
        .unwrap_or(quote! { ::core::default::Default::default() });

      try_from_fields.push(quote! {
          #name: #default,
      });
    }
  }

  for (src_name, field) in &opts.fields {
    if field.skip.is_some() {
      continue;
    }
    let name = field.rename.clone().unwrap_or_else(|| {
      if field.named {
        format_ident!("{}", src_name)
      } else {
        format_ident!("{}", ctr)
      }
    });
    let src_name = format_ident!("{}", src_name);
    ctr += usize::from(field.named);

    match &field.from {
      Some(from) => {
        let final_style = from.style.unwrap_or(style);

        let converter = if try_ {
          from
            .func
            .as_ref()
            .map(|f| quote!(#f(#final_style s.#src_name)?))
            .unwrap_or_else(|| {
              quote! {
                  ::core::convert::TryInto::try_into(#final_style s.#src_name)?
              }
            })
        } else {
          from
            .func
            .as_ref()
            .map(|f| quote!(#f(#final_style s.#src_name)))
            .unwrap_or_else(|| {
              quote! {
                  ::core::convert::Into::into(#final_style s.#src_name)
              }
            })
        };
        try_from_fields.push(quote! {
            #name: #converter,
        });
      }
      None => {
        if try_ {
          try_from_fields.push(quote! {
              #name: ::core::convert::TryInto::try_into(#style s.#src_name)?,
          });
        } else {
          try_from_fields.push(quote! {
              #name: ::core::convert::Into::into(#style s.#src_name),
          });
        }
      }
    }
  }

  let final_impl_generics = &final_generics.final_impl_generics;
  let self_ty_generics = &final_generics.ty_generics;
  let final_where_clause = &final_generics.final_where_clause;
  let src_ty_generics = &final_generics.src_ty_generics;

  let name = &opts.name;
  if try_ {
    let error = error.map(|t| quote!(#t)).unwrap_or_else(|| {
      quote!(
        ::std::boxed::Box<
          dyn ::std::error::Error + ::core::marker::Send + ::core::marker::Sync + 'static,
        >
      )
    });
    Ok(quote! {
        impl #final_impl_generics ::core::convert::TryFrom<#style #src_name #src_ty_generics> for #name #self_ty_generics #final_where_clause {
            type Error = #error;

            fn try_from(s: #style #src_name #src_ty_generics) -> ::core::result::Result<Self, Self::Error> {
                ::core::result::Result::Ok(Self {
                    #(#try_from_fields)*
                })
            }
        }
    })
  } else {
    Ok(quote! {
        impl #final_impl_generics ::core::convert::From<#style #src_name #src_ty_generics> for #name #self_ty_generics #final_where_clause {
            fn from(s: #style #src_name #src_ty_generics) -> Self {
                Self {
                    #(#try_from_fields)*
                }
            }
        }
    })
  }
}
