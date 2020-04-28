use clap;
use std::{error, env};
use std::process::exit;
use toml::Value;
use clap::{App, Arg};
use ex::fs;
use std::fmt::Write;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;

pub fn linker_main() -> Result<(), Box<dyn error::Error>> {
    if env::var("CARGO_MANIFEST_DIR").is_err() {
        env::set_var("CARGO_MANIFEST_DIR", env::current_dir().unwrap().to_str().unwrap());
    }
    let base_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let matches = App::new("esp-idf-n-hal build support - Linker")
        .version("0.1.0")
        .author("Yonas Jongkind <yonas.jongkind@gmail.com>")
        .about("If you set this to be the linker in .cargo/config, then it will configure the ESP-IDF make system")
        .arg(Arg::with_name("libdir")
            .short("L")
            .number_of_values(1)
            .multiple(true)
            .takes_value(true)
            .help("A folder that contains library files."))
        .arg(Arg::with_name("output")
            .short("o")
            .number_of_values(1)
            .takes_value(true)
            .help("A folder that contains library files."))
        .arg(Arg::with_name("n")
            .short("n")
            .number_of_values(1)
            .multiple(true)
            .takes_value(true)
            .help("A folder that contains library files."))
        .arg(Arg::with_name("W")
            .short("W")
            .takes_value(true)
            .number_of_values(1)
            .multiple(true)
            .help("A folder that contains library files."))
        .arg(Arg::with_name("libs")
            .multiple(true)
            // .required(true)
            .help("lib file"))
        .get_matches();

    // if matches.is_present("libdir") {
    //     for x in matches.values_of("libdir").unwrap() {
    //         let xp = Path::new(x);
    //         if xp.exists() && xp.is_file() {
    //             println!("Lib    : {}", x)
    //         } else {
    //             println!("Lib dir: {}", x)
    //         }
    //     }
    // }
    // if matches.is_present("libs") {
    //     for x in matches.values_of("libs").unwrap() {
    //         println!("Lib!: {}", x)
    //     }
    // }
    println!("BEGIN linker_main: {:?}", std::env::current_exe().unwrap());

    let main_dir = format!("{}/main", base_dir);
    let main_path = Path::new(main_dir.as_str());
    if !main_path.exists() {
        fs::create_dir_all(main_path)?;
    }

    let cmakelists_in = format!("{}/main/CMakeLists.txt.in", base_dir);
    let cmakelists_in_path = Path::new(cmakelists_in.as_str());
    if !cmakelists_in_path.exists() {
        println!("Generating: {}", cmakelists_in_path.to_str().unwrap());
        fs::write(
            cmakelists_in_path,
            "idf_component_register(SRCS \"esp_app_main.c\" INCLUDE_DIRS \"\")\n",
        )?;

        let esp_app_main_path = Path::new("main/esp_app_main.c");
        if !esp_app_main_path.exists() {
            println!("Generating: {}", esp_app_main_path.to_str().unwrap());
            fs::write(esp_app_main_path, "void app_main_is_in_rust() {}\n")?;
        }
    }

    //    let CMakeFiles = "idf_component_register(SRCS "esp_app_main.c" INCLUDE_DIRS "")";
    let cmakelists_str = format!("{}/main/CMakeLists.txt", base_dir);
    let cmakelists_path = Path::new(cmakelists_str.as_str());

    let mut cmakelists = String::new();
    write!(cmakelists, "{}", fs::read_to_string(cmakelists_in_path)?)?;

    if matches.is_present("libs") {
        let mut libs_list_str = String::new();
        const LIBS_FOR_IDF_PATH:&str = "target/for_idf";
        fs::create_dir_all(LIBS_FOR_IDF_PATH)?;
        let mut name_counter = HashMap::<String,u32>::new();

        for x in matches.values_of("libs").unwrap() {
            let lib_is_included_by_isp_idf_so_should_be_skipped =
                x.contains("libcompiler_builtins");
            let xp = Path::new(x);
            let lib_name = xp.file_stem().unwrap().to_str().unwrap();
            // let lib_base_name =lib_name;
            let lib_base_name = format!("{}", regex::Regex::new("-.*").unwrap().replace_all(lib_name, ""));

            let name_count = match name_counter.get(&lib_base_name) {
                Some(&number) => {
                    name_counter.remove(&lib_base_name);
                    name_counter.insert(lib_base_name.clone(), number+1);
                    format!("-{}", (number+1))
                },
                _ => {
                    name_counter.insert(lib_base_name.clone(), 0);
                    String::new()
                },
            };

            let new_lib_name = format!(
                "{}/{}{}.{}",
                LIBS_FOR_IDF_PATH,
                lib_base_name,
                name_count,
                xp.extension().unwrap().to_str().unwrap());

            if !lib_is_included_by_isp_idf_so_should_be_skipped {
                fs::copy(x, &new_lib_name)?;

                if libs_list_str.len() > 0 {
                    writeln!(libs_list_str, "")?;
                }
                write!(
                    libs_list_str,
                    "    \"${{CMAKE_CURRENT_SOURCE_DIR}}/../{}\"",
                    new_lib_name
                )?;
            }
        }

        let mut input = String::new();
        File::open(format!("{}/Cargo.toml", base_dir))
            .and_then(|mut f| f.read_to_string(&mut input))
            .unwrap();

        let value = input.parse::<Value>().unwrap();

        // Add any local dependencies to the CMakeFiles so it can know better when to build.
        write!(cmakelists, "file(GLOB_RECURSE RUST_SRCS\n    \"${{CMAKE_CURRENT_SOURCE_DIR}}/../src/*.rs\"")?;
        for x in value["dependencies"].as_table().unwrap() {
            if x.1.is_table() {
                let t = x.1.as_table().unwrap();
                if let Some(v) = t.get("path") {
                    write!(cmakelists, "\n    \"${{CMAKE_CURRENT_SOURCE_DIR}}/../{}/src/*.rs\"",v.as_str().unwrap())?;
                }
            }
        }
        writeln!(cmakelists, ")")?;

        writeln!(cmakelists, "set(LIBS_FROM_RUST \n{})", libs_list_str)?;
        writeln!(cmakelists)?;
        let libs_list = "${LIBS_FROM_RUST}";

        writeln!(
            cmakelists,
            "target_link_libraries(${{COMPONENT_LIB}} INTERFACE {})\n\n",
            libs_list
        )?;
        writeln!(cmakelists, "set_property(DIRECTORY \"${{COMPONENT_DIR}}\" APPEND PROPERTY ADDITIONAL_MAKE_CLEAN_FILES {})", libs_list)?;
        writeln!(cmakelists)?;
        writeln!(
            cmakelists,
            "add_custom_command(COMMENT \"Building the rust portion of the project.\""
        )?;
        writeln!(cmakelists, "  OUTPUT {}", libs_list)?;
        writeln!(cmakelists, "  COMMAND cargo xbuild --release")?;
        writeln!(
            cmakelists,
            "  WORKING_DIRECTORY \"${{CMAKE_CURRENT_SOURCE_DIR}}/..\""
        )?;
        writeln!(cmakelists, "  DEPENDS ${{RUST_SRCS}}")?;
        writeln!(cmakelists, "  VERBATIM USES_TERMINAL)")?;
        writeln!(cmakelists, "")?;
        writeln!(
            cmakelists,
            "add_custom_target(rustbits DEPENDS {})",
            libs_list
        )?;
        writeln!(cmakelists, "add_dependencies(${{COMPONENT_LIB}} rustbits)")?;
        writeln!(cmakelists, "")?;
    }

    fs::write(cmakelists_path, cmakelists)?;

    println!("Generated main/CMakeLists.txt");
    exit(0);
}
