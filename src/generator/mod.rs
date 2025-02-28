use crate::transpiler::converter::{ClarityContract, ClarityExpression, ClarityFunction};
use anyhow::Result;

pub fn generate(contract: ClarityContract) -> Result<String> {
    let mut output = String::new();

    // Generate contract header with documentation
    output.push_str(&format!(
        ";; Contract: {}\n",
        contract.name
    ));
    output.push_str(";; Auto-generated Clarity contract from Solidity source\n\n");

    // Generate constants first
    for var in &contract.data_vars {
        if var.is_constant {
            output.push_str(&format!(
                ";; @desc Constant value for {}\n",
                var.name
            ));
            output.push_str(&format!(
                "(define-constant {} {})\n",
                var.name, var.initial_value
            ));
        }
    }
    output.push_str("\n");

    // Generate map definitions
    for map in &contract.maps {
        output.push_str(&format!(
            ";; @desc Map storing {} values\n",
            map.name
        ));
        output.push_str(&format!(
            "(define-map {} {} {})\n",
            map.name, map.key_type, map.value_type
        ));
    }
    output.push_str("\n");

    // Generate data vars (non-constants)
    for var in &contract.data_vars {
        if !var.is_constant {
            output.push_str(&format!(
                ";; @desc Stores the {} value\n",
                var.name
            ));
            if var.visibility.as_deref() == Some("public") {
                output.push_str(";; @access public\n");
            }
            output.push_str(&format!(
                "(define-data-var {} {} {})\n",
                var.name, var.var_type, var.initial_value
            ));
        }
    }
    output.push_str("\n");

    // Generate public getters for public state variables
    for var in &contract.data_vars {
        if var.visibility.as_deref() == Some("public") && !var.is_constant {
            output.push_str(&format!(
                ";; @desc Getter for public variable {}\n",
                var.name
            ));
            output.push_str(&format!(
                "(define-read-only (get-{})\n",
                var.name
            ));
            output.push_str(&format!(
                "  (ok (var-get {})))\n\n",
                var.name
            ));
        }
    }

    // Generate event definitions as constants
    for event in &contract.events {
        output.push_str(&format!(
            ";; @desc Event: {}\n",
            event.name
        ));
        output.push_str(";; @fields ");
        for field in &event.fields {
            output.push_str(&format!("{}: {}, ", field.name, field.field_type));
        }
        output.push_str("\n\n");
    }

    // Generate functions
    for func in &contract.functions {
        output.push_str(&generate_function(func));
        output.push_str("\n");
    }

    Ok(output)
}

fn generate_function(func: &ClarityFunction) -> String {
    let mut output = String::new();

    // Add function documentation
    output.push_str(&format!(";; Function: {}\n", func.name));

    // Add read-only/public indicator
    if func.read_only {
        output.push_str(";; @access read-only\n");
    }

    // Generate function signature with appropriate visibility
    if func.public {
        output.push_str(&format!("(define-public ({}", func.name));
    } else {
        output.push_str(&format!("(define-private ({}", func.name));
    }

    // Generate parameters
    for param in &func.params {
        output.push_str(&format!(" ({} {})", param.name, param.param_type));
    }
    output.push_str(")\n");

    // Generate function body
    output.push_str("  ");
    if let Some(last_expr) = func.body.last() {
        // Generate all expressions except the last one
        for expr in &func.body[..func.body.len() - 1] {
            output.push_str(&generate_expression(expr));
            output.push_str("\n  ");
        }
        // Return the last expression wrapped in (ok ...)
        output.push_str("(ok ");
        output.push_str(&generate_expression(last_expr));
        output.push_str(")");
    } else {
        // Return the default value for empty functions
        output.push_str("(ok true)");
    }

    output.push_str(")\n");
    output
}

fn generate_expression(expr: &ClarityExpression) -> String {
    match expr {
        ClarityExpression::Literal(val) => val.clone(),
        ClarityExpression::Var(name) => name.clone(),
        ClarityExpression::FunctionCall(name, args) => {
            let mut output = format!("({}", name);
            for arg in args {
                output.push_str(" ");
                output.push_str(&generate_expression(arg));
            }
            output.push(')');
            output
        }
        ClarityExpression::MapGet(map_name, keys) => {
            let mut output = format!("(map-get? {}", map_name);
            for key in keys {
                output.push_str(" ");
                output.push_str(&generate_expression(key));
            }
            output.push(')');
            output
        }
        ClarityExpression::MapSet(map_name, keys, value) => {
            let mut output = format!("(map-set {}", map_name);
            for key in keys {
                output.push_str(" ");
                output.push_str(&generate_expression(key));
            }
            output.push_str(" ");
            output.push_str(&generate_expression(value));
            output.push(')');
            output
        }
        ClarityExpression::Print(args) => {
            let mut output = String::from("(print");
            for arg in args {
                output.push_str(" ");
                output.push_str(&generate_expression(arg));
            }
            output.push(')');
            output
        }
    }
}