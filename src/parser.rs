pub(crate) struct Parser;

impl Parser {
  pub(crate) fn parse_fields<T: syn::spanned::Spanned>(
    span: &T,
    raw: &str,
  ) -> syn::Result<(bool, syn::Fields)> {
    let Some(start) = raw.find('{') else {
            return Err(
                darling::Error::custom("expected left curly brace")
                    .with_span(span),
            )?;
        };
    let Some(end) = raw.rfind('}') else {
            return Err(
                darling::Error::custom("expected right curly brace")
                    .with_span(span),
            )?;
        };
    syn::parse_str::<syn::FieldsNamed>(&raw[start..end + 1])
      .map(|v| (true, syn::Fields::Named(v)))
      .or_else(|_| {
        syn::parse_str::<syn::FieldsUnnamed>(&raw[start..end + 1])
          .map(|v| (false, syn::Fields::Unnamed(v)))
          .map_err(|_| syn::Error::new(span.span(), "fail to parse extra fields"))
      })
  }

  pub(crate) fn parse<T>(
    name: &str,
    inner: &syn::Meta,
    target: &mut (bool, Option<T>),
  ) -> darling::Result<()>
  where
    T: darling::FromMeta,
  {
    if !target.0 {
      *target = (
        true,
        Some(darling::FromMeta::from_meta(inner).map_err(|e| e.with_span(&inner).at(name))?),
      );
      Ok(())
    } else {
      Err(darling::Error::duplicate_field(name).with_span(&inner))
    }
  }
}
