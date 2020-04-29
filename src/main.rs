git use crate::lib::linker_main;

mod lib;

fn main() {
    linker_main().expect("Failed to configure cmake");
}
