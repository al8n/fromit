use super::*;

#[derive(Default, FromMeta, Clone, Copy)]
pub(crate) enum Style {
  Ref,
  #[default]
  Move,
}

impl ToTokens for Style {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    match self {
      Style::Ref => tokens.extend(quote! { & }),
      Style::Move => tokens.extend(quote! {}),
    }
  }
}

#[derive(Default, FromMeta)]
pub(crate) struct FieldConverter {
  pub(crate) style: Option<Style>,
  #[darling(rename = "fn")]
  pub(crate) func: Option<syn::Path>,
}

pub(crate) struct FieldLevelSkip {
  pub(crate) default: Option<syn::Path>,
}

impl darling::FromMeta for FieldLevelSkip {
  fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
    match item {
      syn::Meta::Path(_) => Ok(Self { default: None }),
      syn::Meta::List(l) => {
        let mut default: (bool, Option<syn::Path>) = (false, None);
        for item in &l.nested {
          match item {
            syn::NestedMeta::Meta(inner) => {
              if inner.path().is_ident("default") {
                crate::parser::Parser::parse("default", inner, &mut default)?;
              }
            }
            syn::NestedMeta::Lit(v) => {
              return Err(darling::Error::unsupported_format("literal").with_span(v));
            }
          }
        }
        Ok(Self { default: default.1 })
      }
      syn::Meta::NameValue(nv) => {
        Err(darling::Error::unsupported_format("namedValue").with_span(nv))
      }
    }
  }
}

pub(crate) struct ExtraField {
  pub(crate) name: Option<syn::Ident>,
  pub(crate) src_ty: syn::Type,
  pub(crate) src_vis: syn::Visibility,
  pub(crate) vis: Option<syn::Visibility>,
  pub(crate) getter: FieldLevelGetter,
  pub(crate) setter: FieldLevelSetter,
  pub(crate) default: Option<syn::Path>,
  pub(crate) attributes: Attributes,
  pub(crate) named: bool,
}

pub(crate) struct Field {
  pub(crate) src_ty: syn::Type,
  pub(crate) src_vis: syn::Visibility,
  pub(crate) skip: Option<FieldLevelSkip>,
  pub(crate) vis: Option<syn::Visibility>,
  pub(crate) typ: Option<syn::Type>,
  pub(crate) rename: Option<syn::Ident>,
  pub(crate) parent: Option<syn::Ident>,
  pub(crate) getter: FieldLevelGetter,
  pub(crate) setter: FieldLevelSetter,
  pub(crate) from: Option<FieldConverter>,
  pub(crate) into: Option<FieldConverter>,
  pub(crate) attributes: Attributes,
  pub(crate) named: bool,
}

impl Field {
  pub(crate) fn merge(&mut self, other: Self) {
    if other.skip.is_some() {
      self.skip = other.skip;
    }

    if other.vis.is_some() {
      self.vis = other.vis;
    }

    if other.typ.is_some() {
      self.typ = other.typ;
    }

    if other.rename.is_some() {
      self.rename = other.rename;
    }

    if other.parent.is_some() {
      self.parent = other.parent;
    }

    if other.from.is_some() {
      self.from = other.from;
    }

    if other.into.is_some() {
      self.into = other.into;
    }

    self.attributes.attrs.extend(other.attributes.attrs);
  }
}
