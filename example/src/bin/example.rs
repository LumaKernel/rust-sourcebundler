#[macro_use]
extern crate example;
// use example::example_core;
// use example::example_core::example_hello;
// use example::example_core::{self, example_hello};
// use example::example_core::{self};

fn main() {
    // println!("{}", example_core::example_hello());
    foo!(1);
    // println!("{}", example_hello());
}
