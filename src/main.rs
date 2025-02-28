use clap::{App, Arg};
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

mod parser;
mod transpiler;
mod generator;

#[cfg(test)]
mod tests;

fn main() -> Result<()> {
    let matches = App::new("sol2clarity")
        .version("0.1.0")
        .author("Solidity to Clarity Transpiler")
        .about("Converts Solidity smart contracts to Clarity")
        .arg(
            Arg::with_name("INPUT")
                .help("Input Solidity file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("Output Clarity file")
                .takes_value(true),
        )
        .get_matches();

    let input_file = matches.value_of("INPUT").unwrap();
    let output_file = matches
        .value_of("output")
        .map(String::from)
        .unwrap_or_else(|| {
            Path::new(input_file)
                .with_extension("clar")
                .to_string_lossy()
                .into_owned()
        });

    // Read input file
    let source = fs::read_to_string(input_file)
        .with_context(|| format!("Failed to read input file: {}", input_file))?;

    // Parse Solidity code
    let ast = parser::parse(&source)
        .with_context(|| "Failed to parse Solidity code")?;

    // Convert to Clarity AST
    let clarity_ast = transpiler::convert(ast)
        .with_context(|| "Failed to convert to Clarity")?;

    // Generate Clarity code
    let clarity_code = generator::generate(clarity_ast)
        .with_context(|| "Failed to generate Clarity code")?;

    // Write output file
    fs::write(&output_file, clarity_code)
        .with_context(|| format!("Failed to write output file: {}", output_file))?;

    println!("Successfully converted {} to {}", input_file, output_file);
    Ok(())
}