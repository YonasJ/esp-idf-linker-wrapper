mod linker_that_configures_cmake;

fn main() {
    linker_that_configures_cmake::linker_main().expect("Failed to configure cmake");
}
