use proptest::prelude::prop_compose;
use proptest_attr_macro::proptest;

#[proptest(33u8..100u8)]
fn inline_strategy(x: u8) {
    assert!(x >= 33 && x < 100);
}

#[proptest(range(1, 5))]
fn predefined_strategy(x: u8) {
    assert!(x >= 1 && x < 5);
}

#[proptest(range(5, 7), range(10, 20))]
fn multiple_strategies(x: u8, y: u8) {
    assert!(x >= 5 && x < 7);
    assert!(y >= 10 && y < 20);
}

#[proptest(1u8..5u8, 6u8..10u8)]
fn multiple_inline_strategies(x: u8, y: u8) {
    assert!(x >= 1 && x < 5);
    assert!(y >= 6 && y < 10);
}

prop_compose! {
  fn range(from: u8, to: u8)
                       (integer in from..to)
                       -> u8 {
    integer
  }
}
