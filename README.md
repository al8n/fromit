# fromit

A super powerful macro for generating new structs with getters, setters, and `From` or `TryFrom` implementation based on the given struct.

## Example

```rust
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
```

The `FromIt` will help you to write the below of code.

```rust

use fromit::FromIt;

fn conv(x: &String) -> Result<Vec<u8>, std::convert::Infallible> {
    Ok(x.as_bytes().to_vec())
}

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
    #[fromit(parent = "FooGraphql", rename = "foo1", skip, type = "Vec<u8>")]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FooDb {
    #[serde(rename = "foo2")]
    foo1: Vec<u8>,
    bar: i32,
    baz: u64,
}

impl<H: core::hash::Hash, O> ::core::convert::TryFrom<&Foo<H, O>> for FooDb
where
    O: Eq,
{
    type Error = ::std::boxed::Box<
        dyn ::std::error::Error + ::core::marker::Send + ::core::marker::Sync + 'static,
    >;
    fn try_from(s: &Foo<H, O>) -> ::core::result::Result<Self, Self::Error> {
        ::core::result::Result::Ok(Self {
            foo1: conv(&s.foo)?,
            bar: ::core::convert::TryInto::try_into(s.bar)?,
            baz: ::core::convert::TryInto::try_into(s.baz)?,
        })
    }
}
impl FooDb {
    #[inline]
    fn foo(&self) -> &Vec<u8> {
        &self.foo1
    }
    #[inline]
    fn bar(&self) -> &i32 {
        &self.bar
    }
    #[inline]
    fn baz(&self) -> &u64 {
        &self.baz
    }
}
impl FooDb {
    fn set_foo1<C>(&mut self, val: Vec<u8>) {
        self.foo1 = val;
    }
    fn set_bar(mut self, val: i32) -> Self {
        self.bar = val;
        self
    }
    fn set_baz(mut self, val: u64) -> Self {
        self.baz = val;
        self
    }
}
struct FooGraphql<H, O, T, C>
where
    O: Eq,
{
    panda: T,
    tiger: C,
    h: H,
    baz: u64,
    o: O,
    bar: i32,
}

impl<H: core::hash::Hash, O, T: Clone + core::fmt::Debug + Default, C: Copy + Default>
    FooGraphql<H, O, T, C>
where
    O: Eq,
{
    #[inline]
    fn x_h(&self) -> &H {
        &self.h
    }
    #[inline]
    fn x_baz(&self) -> &u64 {
        &self.baz
    }
    #[inline]
    fn x_o(&self) -> &O {
        &self.o
    }
    #[inline]
    fn x_bar(&self) -> &i32 {
        &self.bar
    }
    #[inline]
    fn x_panda(&self) -> &T {
        &self.panda
    }
    #[inline]
    fn x_tiger(&self) -> &C {
        &self.tiger
    }
}
impl<H: core::hash::Hash, O, T: Clone + core::fmt::Debug + Default, C: Copy + Default>
    FooGraphql<H, O, T, C>
where
    O: Eq,
{
    fn set_h(mut self, val: H) -> Self {
        self.h = val;
        self
    }
    fn set_baz(mut self, val: u64) -> Self {
        self.baz = val;
        self
    }
    fn set_o(mut self, val: O) -> Self {
        self.o = val;
        self
    }
    fn set_bar(mut self, val: i32) -> Self {
        self.bar = val;
        self
    }
    fn set_panda(mut self, val: T) -> Self {
        self.panda = val;
        self
    }
    fn set_tiger(mut self, val: C) -> Self {
        self.tiger = val;
        self
    }
}
```
