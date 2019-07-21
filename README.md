This crate provides a procedural attribute macro version of [proptest]'s `proptest!` macro.

So instead of having to write:

``` rust
use proptest::proptest;

proptest! {
    fn test_excluded_middle(x: u32, y: u32) {
        assert!(x == y || x != y);
    }
}
```

you can write:

``` rust
use proptest_attr_macro::proptest;

#[proptest]
fn test_excluded_middle(x: u32, y: u32) {
    assert!(x == y || x != y);
}
```

[proptest]: https://docs.rs/proptest/*/

## Limitations

Procedural attribute macros can only be used with valid Rust syntax, which means that you can't
use proptest's `in` operator (which allows you to draw values from a specific strategy
function):

``` rust
// This won't compile!
#[proptest]
fn test_even_numbers(x in even(any::<u32>())) {
    assert!((x % 2) == 0);
}
```

Instead you must provide an actual parameter list, just like you would with a real Rust
function definition.  That, in turn, means that your function parameters can only draw values
using the `any` strategy for their types.  If you want to use a custom strategy, you must
create a separately named type, and have the new type's `Arbitrary` impl use that strategy:

``` rust
struct Even { value: i32 }

impl Arbitrary for Even {
    type Parameters = ();
    type Strategy = BoxedStrategy<Even>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        (0..100).prop_map(|x| Even { value: x * 2 }).boxed()
    }
}

#[proptest]
fn test_even_numbers(even: Even) {
    assert!((even.value % 2) == 0);
}
```

## Benefits

The main one is purely aesthetic: since you're applying the `proptest` attribute macro to valid
Rust functions, `rustfmt` works on them!
