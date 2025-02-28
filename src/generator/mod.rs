use crate::transpiler::converter::{ClarityContract, ClarityExpression, ClarityFunction};
use anyhow::Result;

fn to_kebab_case(s: &str) -> String {
    if s.chars().all(|c| c.is_uppercase() || c.is_numeric() || c == '_') {
        return s.to_string();
    }

    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i != 0 { out.push('-'); }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

pub fn generate(contract: ClarityContract) -> Result<String> {
    let mut output = String::new();

    output.push_str(&format!(
        ";; Contract: {}\n",
        contract.name
    ));
    output.push_str(";; Auto-generated Clarity contract from Solidity source\n\n");

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

    for map in &contract.maps {
        output.push_str(&format!(
            ";; @desc Map storing {} values\n",
            map.name
        ));
        let map_name = to_kebab_case(&map.name);
        output.push_str(&format!(
            "(define-map {} {} {})\n",
            map_name, map.key_type, map.value_type
        ));

        output.push_str(&format!(
            ";; @desc Getter for map {}\n",
            map.name
        ));
        output.push_str(&format!(
            "(define-read-only (get-{} (key {}))\n",
            map_name, map.key_type
        ));
        output.push_str(&format!(
            "  (ok (map-get? {} key)))\n\n",
            map_name
        ));
    }

    for var in &contract.data_vars {
        if !var.is_constant {
            output.push_str(&format!(
                ";; @desc Stores the {} value\n",
                var.name
            ));
            if var.visibility.as_ref().map_or(false, |v| v == "public") {
                output.push_str(";; @access public\n");
            }
            let var_name = var.name.clone();
            output.push_str(&format!(
                "(define-data-var {} {} {})\n",
                var_name, var.var_type, var.initial_value
            ));

            if var.visibility.as_ref().map_or(false, |v| v == "public") {
                output.push_str(&format!(
                    ";; @desc Getter for public variable {}\n",
                    var.name
                ));
                output.push_str(&format!(
                    "(define-read-only (get-{})\n",
                    var_name
                ));
                output.push_str(&format!(
                    "  (ok (var-get {})))\n\n",
                    var_name
                ));
            }
        }
    }
    output.push_str("\n");

    for event in &contract.events {
        output.push_str(&format!(
            ";; @desc Event: {}\n",
            event.name
        ));
        output.push_str(";; @fields ");
        for field in &event.fields {
            output.push_str(&format!("{}{}: {}, ", 
                if field.indexed { "(indexed) " } else { "" },
                field.name, 
                field.field_type));
        }
        output.push_str("\n\n");
    }

    for func in &contract.functions {
        output.push_str(&generate_function(func));
        output.push_str("\n");
    }

    Ok(output)
}

fn generate_function(func: &ClarityFunction) -> String {
    let mut output = String::new();

    output.push_str(&format!(";; Function: {}\n", func.name));

    if func.read_only {
        output.push_str(";; @access read-only\n");
    }

    if func.public {
        output.push_str(&format!("(define-public ({}", func.name));
    } else {
        output.push_str(&format!("(define-private ({}", func.name));
    }

    for param in &func.params {
        output.push_str(&format!(" ({} {})", param.name, param.param_type));
    }
    output.push_str(")\n");

    output.push_str("  ");
    if func.body.len() > 1 {
        output.push_str("(begin\n    ");
        for expr in &func.body[..func.body.len() - 1] {
            output.push_str(&generate_expression(expr));
            output.push_str("\n    ");
        }
        output.push_str("(ok ");
        output.push_str(&generate_expression(func.body.last().unwrap()));
        output.push_str("))");
    } else if let Some(last_expr) = func.body.last() {
        output.push_str("(ok ");
        output.push_str(&generate_expression(last_expr));
        output.push_str(")");
    } else {
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
            format!("(map-get? {} {})", map_name, keys.iter()
                .map(|k| generate_expression(k))
                .collect::<Vec<_>>()
                .join(" "))
        }
        ClarityExpression::MapSet(map_name, keys, value) => {
            format!("(map-set {} {} {})",
                map_name,
                keys.iter()
                    .map(|k| generate_expression(k))
                    .collect::<Vec<_>>()
                    .join(" "),
                generate_expression(value))
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