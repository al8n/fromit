use fromit::FromIt;

fn conv(x: &String) -> Result<Vec<u8>, std::convert::Infallible> {
  Ok(x.as_bytes().to_vec())
}

#[derive(FromIt)]
#[fromit(
  name = "FooDb",
  converter(try_from(style = "ref")),
  attributes(
    derive(Clone, Debug, serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
  )
)]
#[fromit(
  name = "FooGraphql",
  getters(prefix = "x", style = "ref"),
  bound(
    inherit,
    extra = "T: Clone + core::fmt::Debug + Default, C: Copy + Default"
  ),
  extra(fields(
    r#"{
      panda: T,
      tiger: C,
    }"#
  ))
)]
struct Foo<H: core::hash::Hash, O>
where
  O: Eq,
{
  #[fromit(
    parent = "FooDb",
    rename = "foo1",
    type = "Vec<u8>",
    from(fn = "conv"),
    getter(style = "ref", rename = "foo"),
    setter(style = "ref", bound = "C"),
    attributes(serde(rename = "foo2"))
  )]
  #[fromit(
    parent = "FooGraphql",
    rename = "foo1",
    skip,
    type = "Vec<u8>",
  )]
  foo: String,
  #[fromit(parent = "FooDb", from(style = "move"))]
  bar: i32,
  #[fromit(parent = "FooDb", from(style = "move"))]
  baz: u64,
  #[fromit(parent = "FooDb", skip)]
  h: H,
  #[fromit(parent = "FooDb", skip)]
  o: O,
}

fn main() {}
