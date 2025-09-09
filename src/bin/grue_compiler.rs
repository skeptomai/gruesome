// grue-compiler - Z-Machine Compiler for Interactive Fiction
// Compiles Grue language source files to Z-Machine story files

use std::env;
use std::fs;
use std::path::Path;
use std::process;

use gruesome::grue_compiler::{GrueCompiler, ZMachineVersion};

fn main() {
    // Initialize logging
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    let mut input_file = "";
    let mut output_file = String::new();
    let mut version = ZMachineVersion::V3;
    let mut verbose = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: -o requires a filename");
                    process::exit(1);
                }
                output_file = args[i + 1].clone();
                i += 2;
            }
            "--version" => {
                if i + 1 >= args.len() {
                    #[cfg(debug_assertions)]
                    eprintln!("Error: --version requires v3, v4, or v5 (v4/v5 are experimental)");
                    #[cfg(not(debug_assertions))]
                    eprintln!("Error: --version requires v3 (v4/v5 disabled in release builds)");
                    process::exit(1);
                }
                version = match args[i + 1].as_str() {
                    "v3" | "V3" => ZMachineVersion::V3,
                    "v4" | "V4" => {
                        #[cfg(not(debug_assertions))]
                        {
                            eprintln!("Error: V4 compilation is experimental and disabled in release builds.");
                            eprintln!(
                                "V4 support has known string alignment and IR mapping issues."
                            );
                            eprintln!(
                                "Use debug build (cargo run) to compile V4 files for testing."
                            );
                            process::exit(1);
                        }
                        #[cfg(debug_assertions)]
                        {
                            eprintln!("Warning: V4 compilation is experimental and may fail.");
                            ZMachineVersion::V4
                        }
                    }
                    "v5" | "V5" => {
                        #[cfg(not(debug_assertions))]
                        {
                            eprintln!("Error: V5 compilation is experimental and disabled in release builds.");
                            eprintln!(
                                "V5 support has known string alignment and IR mapping issues."
                            );
                            eprintln!(
                                "Use debug build (cargo run) to compile V5 files for testing."
                            );
                            process::exit(1);
                        }
                        #[cfg(debug_assertions)]
                        {
                            eprintln!("Warning: V5 compilation is experimental and may fail.");
                            ZMachineVersion::V5
                        }
                    }
                    _ => {
                        eprintln!(
                            "Error: Unsupported version '{}'. Use v3, v4, or v5.",
                            args[i + 1]
                        );
                        process::exit(1);
                    }
                };
                i += 2;
            }
            "-v" | "--verbose" => {
                verbose = true;
                i += 1;
            }
            "-h" | "--help" => {
                print_usage(&args[0]);
                process::exit(0);
            }
            arg if arg.starts_with('-') => {
                eprintln!("Error: Unknown option '{}'", arg);
                print_usage(&args[0]);
                process::exit(1);
            }
            _ => {
                if input_file.is_empty() {
                    input_file = &args[i];
                } else {
                    eprintln!("Error: Multiple input files specified");
                    process::exit(1);
                }
                i += 1;
            }
        }
    }

    if input_file.is_empty() {
        eprintln!("Error: No input file specified");
        print_usage(&args[0]);
        process::exit(1);
    }

    if output_file.is_empty() {
        // Generate output filename from input
        let input_path = Path::new(input_file);
        let base_name = input_path.file_stem().unwrap_or_else(|| {
            eprintln!("Error: Invalid input filename");
            process::exit(1);
        });

        let extension = match version {
            ZMachineVersion::V3 => "z3",
            ZMachineVersion::V4 => "z4",
            ZMachineVersion::V5 => "z5",
        };

        output_file = format!("{}.{}", base_name.to_string_lossy(), extension);
    }

    if verbose {
        println!(
            "Compiling {} -> {} (Z-Machine {})",
            input_file, output_file, version
        );
    }

    // Read source file
    let source = match fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading '{}': {}", input_file, err);
            process::exit(1);
        }
    };

    // Compile
    let compiler = GrueCompiler::new();
    match compiler.compile(&source, version) {
        Ok(story_data) => {
            let data_size = story_data.len();

            // Write output file
            if let Err(err) = fs::write(&output_file, story_data) {
                eprintln!("Error writing '{}': {}", output_file, err);
                process::exit(1);
            }

            if verbose {
                println!(
                    "Successfully compiled {} bytes to {}",
                    data_size, output_file
                );
            }
        }
        Err(err) => {
            eprintln!("Compilation error: {}", err);
            process::exit(1);
        }
    }
}

fn print_usage(program_name: &str) {
    println!("Usage: {} [options] <input.grue>", program_name);
    println!();
    println!("Options:");
    println!("  -o, --output <file>    Output filename (default: input.z3)");
    println!("  --version <v3|v4|v5>   Z-Machine version (default: v3)");
    println!("  -v, --verbose          Verbose output");
    println!("  -h, --help             Show this help message");
    println!();
    println!("Z-Machine Version Support:");
    println!("  v3                     Production ready (recommended)");
    #[cfg(debug_assertions)]
    {
        println!("  v4, v5                 Experimental (debug builds only)");
    }
    #[cfg(not(debug_assertions))]
    {
        println!("  v4, v5                 Experimental (disabled in release)");
    }
    println!();
    println!("Examples:");
    println!(
        "  {} game.grue                    # Compile to game.z3 (production)",
        program_name
    );
    #[cfg(debug_assertions)]
    {
        println!(
            "  {} --version v4 game.grue       # Compile to game.z4 (experimental)",
            program_name
        );
        println!(
            "  {} --version v5 game.grue       # Compile to game.z5 (experimental)",
            program_name
        );
    }
    println!(
        "  {} -o mygame.z3 source.grue     # Custom output name",
        program_name
    );
}
