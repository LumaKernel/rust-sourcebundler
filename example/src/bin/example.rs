// use example::example_core;
// use example::example_core::example_hello;
use example::example_core::{self, example_hello};
// use example::example_core::{self};

fn main() {
    println!("{}", example_core::example_hello());
    // println!("{}", example_hello());
}
