use proptest::prelude::prop_compose;
use proptest_attr_macro::proptest;

#[proptest(33u8..100u8)]
fn inline_strategy(x: u8) {
    assert!(x >= 33 && x < 100)
}

#[proptest(up_to(43))]
fn predefined_strategy(x: u8) {
    assert!(x <= 42)
}

#[proptest(up_to(43), up_to(100))]
fn multiple_strategies(x: u8, y: u8) {
    assert!(x <= 42 && y < 100)
}

#[proptest(1u8..5u8, 6u8..10u8)]
fn multiple_inline_strategies(x: u8, y: u8) {
    assert!(x >= 1 && x < 5);
    assert!(y >= 6 && y < 10);
}

prop_compose! {
  fn up_to(max_integer: u8)
                       (integer in 0..max_integer)
                       -> u8 {
    integer
  }
}
