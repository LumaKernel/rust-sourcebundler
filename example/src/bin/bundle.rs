use std::path::Path;
extern crate rustsourcebundler;
use rustsourcebundler::Bundler;

fn main() {
    let mut bundler: Bundler = Bundler::new(
        Path::new("src/bin/example.rs"),
        Path::new("src/bin/bundled.rs"),
    );
    bundler.crate_name("example");
    bundler.header("// DO NOT EDIT: Generated file.");
    bundler.run();
}
