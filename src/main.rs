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
                .help("Output directory for Clarity files")
                .takes_value(true),
        )
        .get_matches();

    let input_file = matches.value_of("INPUT").unwrap();
    let output_dir = matches
        .value_of("output")
        .map(String::from)
        .unwrap_or_else(|| String::from("."));

    // Read input file
    let source = fs::read_to_string(input_file)
        .with_context(|| format!("Failed to read input file: {}", input_file))?;

    // Parse Solidity code - now returns a Vec<Contract>
    let contracts = parser::parse_all(&source)
        .with_context(|| "Failed to parse Solidity code")?;

    // Process each contract
    for contract in contracts {
        let contract_name = contract.name.clone();

        // Convert to Clarity AST
        let clarity_ast = transpiler::convert(contract)
            .with_context(|| format!("Failed to convert {} to Clarity", contract_name))?;

        // Generate Clarity code
        let clarity_code = generator::generate(clarity_ast)
            .with_context(|| format!("Failed to generate Clarity code for {}", contract_name))?;

        // Create output file path
        let output_file = Path::new(&output_dir)
            .join(format!("{}.clar", contract_name.to_lowercase()));

        // Write output file
        fs::write(&output_file, clarity_code)
            .with_context(|| format!("Failed to write output file: {}", output_file.display()))?;

        println!("Successfully converted {} to {}", contract_name, output_file.display());
    }

    Ok(())
}