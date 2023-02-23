use super::*;

pub(crate) fn generate_into(
  src_name: &syn::Ident,
  opts: &StructOpts,
  final_generics: &FinalGenerics,
) -> syn::Result<proc_macro2::TokenStream> {
  if opts.converter.try_into.is_none() && opts.converter.into.is_none() {
    return Ok(quote!());
  }
  let (try_, style, error) = match (&opts.converter.try_into, &opts.converter.into) {
    (None, None) => return Ok(quote!()),
    (None, Some(into)) => (false, *into, None),
    (Some(try_into), None) => (true, try_into.style, try_into.error.clone()),
    (Some(_), Some(_)) => {
      return Err(syn::Error::new_spanned(
        &opts.name,
        "Cannot have both `try_from` and `from`",
      ))
    }
  };

  let mut try_into_fields = Vec::new();
  for (src_name, field) in &opts.fields {
    let src_name = format_ident!("{}", src_name);
    if let Some(skip) = &field.skip {
      if let Some(default) = &skip.default {
        try_into_fields.push(quote! {
            #src_name: #default(),
        });
      } else {
        try_into_fields.push(quote! {
            #src_name: ::core::default::Default::default(),
        });
      }

      continue;
    }

    let name = field
      .rename
      .clone()
      .unwrap_or_else(|| format_ident!("{}", src_name));

    let src_name = format_ident!("{}", src_name);
    match &field.into {
      Some(into) => {
        let final_style = into.style.unwrap_or(style);

        let converter = if try_ {
          into
            .func
            .as_ref()
            .map(|f| quote!(#f(#final_style s.#name)?))
            .unwrap_or_else(|| {
              quote! {
                  ::core::convert::TryInto::try_into(#final_style s.#name)?
              }
            })
        } else {
          into
            .func
            .as_ref()
            .map(|f| quote!(#f(#final_style s.#name)))
            .unwrap_or_else(|| {
              quote! {
                  ::core::convert::Into::into(#final_style s.#name)
              }
            })
        };
        try_into_fields.push(quote! {
            #src_name: #converter,
        });
      }
      None => match style {
        Style::Ref => {
          if try_ {
            try_into_fields.push(quote! {
                #src_name: ::core::convert::TryInto::try_into(&s.#name)?,
            });
          } else {
            try_into_fields.push(quote! {
                #src_name: ::core::convert::Into::into(&s.#name),
            });
          }
        }
        Style::Move => {
          if try_ {
            try_into_fields.push(quote! {
                #src_name: ::core::convert::TryInto::try_into(s.#name)?,
            });
          } else {
            try_into_fields.push(quote! {
                #src_name: ::core::convert::Into::into(s.#name),
            });
          }
        }
      },
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
        impl #final_impl_generics ::core::convert::TryFrom<#style #name #self_ty_generics> for #src_name #src_ty_generics #final_where_clause {
            type Error = #error;

            fn try_from(s: #style #name #self_ty_generics) -> ::core::result::Result<Self, Self::Error> {
                ::core::result::Result::Ok(Self {
                    #(#try_into_fields)*
                })
            }
        }
    })
  } else {
    Ok(quote! {
        impl #final_impl_generics ::core::convert::From<#style #name #self_ty_generics> for #src_name #src_ty_generics #final_where_clause {
            fn from(s: #style #name #self_ty_generics) -> Self {
                Self {
                    #(#try_into_fields)*
                }
            }
        }
    })
  }
}
