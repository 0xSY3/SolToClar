use pest::Parser;
use pest_derive::Parser;
use anyhow::{Result, Context, anyhow};
use crate::transpiler::ast::*;

#[derive(Parser)]
#[grammar = "parser/solidity.pest"]
pub struct SolidityParser;

fn debug_log(msg: &str) {
    println!("[DEBUG] {}", msg);
}

pub fn parse_all(source: &str) -> Result<Vec<Contract>> {
    let mut file_pairs = SolidityParser::parse(Rule::file, source)
        .with_context(|| "Failed to parse Solidity contract, syntax error")?;

    // Get the file pair (should be the first and only one)
    let file_pair = file_pairs.next()
        .ok_or_else(|| anyhow!("Empty source file"))?;

    debug_log(&format!("Processing file: {}", file_pair.as_str()));

    // Parse all contracts
    let mut contracts = Vec::new();
    for pair in file_pair.into_inner() {
        match pair.as_rule() {
            Rule::contract_declaration => {
                debug_log(&format!("Found contract declaration: {}", pair.as_str()));
                let mut contract = Contract {
                    name: String::new(),
                    functions: Vec::new(),
                    state_variables: Vec::new(),
                    events: Vec::new(),
                    constructor: None,
                };

                for item in pair.into_inner() {
                    match item.as_rule() {
                        Rule::identifier => {
                            contract.name = item.as_str().to_string();
                            debug_log(&format!("Found contract name: {}", contract.name));
                        }
                        Rule::contract_body => {
                            parse_contract_body(&mut contract, item)?;
                        }
                        _ => {}
                    }
                }
                contracts.push(contract);
            }
            Rule::EOI => {}
            _ => debug_log(&format!("Skipping rule: {:?}", pair.as_rule())),
        }
    }

    if contracts.is_empty() {
        return Err(anyhow!("No contract found in source"));
    }

    Ok(contracts)
}


fn parse_contract_body(contract: &mut Contract, pair: pest::iterators::Pair<Rule>) -> Result<()> {
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::state_variable_declaration => {
                debug_log(&format!("Parsing state variable declaration: {}", item.as_str()));
                if let Some(var) = parse_state_variable(item)? {
                    contract.state_variables.push(var);
                }
            }
            Rule::function_definition => {
                debug_log(&format!("Parsing function definition: {}", item.as_str()));
                let mut constructor = None;
                let mut function = None;

                for inner in item.into_inner() {
                    match inner.as_rule() {
                        Rule::constructor_definition => {
                            constructor = parse_constructor(inner)?;
                        }
                        Rule::regular_function_definition => {
                            function = parse_function(inner)?;
                        }
                        _ => {}
                    }
                }

                if let Some(ctor) = constructor {
                    contract.constructor = Some(ctor);
                } else if let Some(func) = function {
                    contract.functions.push(func);
                }
            }
            Rule::event_definition => {
                debug_log(&format!("Parsing event definition: {}", item.as_str()));
                if let Some(event) = parse_event(item)? {
                    contract.events.push(event);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_state_variable(pair: pest::iterators::Pair<Rule>) -> Result<Option<StateVariable>> {
    let mut var = StateVariable {
        name: String::new(),
        var_type: String::new(),
        visibility: None,
        is_mapping: false,
        mapping_key_type: None,
        mapping_value_type: None,
        is_constant: false,
        initial_value: None,
        nested_mapping: None,
    };

    let decl = pair.into_inner().next().unwrap();
    match decl.as_rule() {
        Rule::basic_state_variable_declaration | Rule::mapping_state_variable_declaration => {
            for token in decl.into_inner() {
                match token.as_rule() {
                    Rule::basic_type => {
                        var.var_type = token.as_str().to_string();
                        debug_log(&format!("Found basic type: {}", var.var_type));
                    }
                    Rule::mapping_type => {
                        var.is_mapping = true;
                        let (key_type, value_type, nested) = parse_mapping_type(token)?;
                        var.mapping_key_type = Some(key_type.clone());
                        var.mapping_value_type = Some(value_type.clone());
                        var.nested_mapping = nested;
                        var.var_type = format!("mapping({} => {})", key_type, value_type);
                        debug_log(&format!("Found mapping type: {}", var.var_type));
                    }
                    Rule::visibility_modifier => {
                        var.visibility = Some(token.as_str().to_string());
                    }
                    Rule::constant_modifier => {
                        var.is_constant = true;
                    }
                    Rule::identifier => {
                        var.name = token.as_str().to_string();
                    }
                    Rule::expression => {
                        var.initial_value = Some(parse_expression(token)?);
                    }
                    _ => {}
                }
            }
        }
        _ => return Ok(None),
    }

    if var.name.is_empty() {
        return Err(anyhow!("State variable must have a name"));
    }

    Ok(Some(var))
}

fn parse_mapping_type(pair: pest::iterators::Pair<Rule>) -> Result<(String, String, Option<Box<MappingType>>)> {
    let mut tokens = pair.into_inner();

    let key_type = tokens.next()
        .ok_or_else(|| anyhow!("Mapping key type not found"))?
        .as_str().to_string();

    let value_type_token = tokens.next()
        .ok_or_else(|| anyhow!("Mapping value type not found"))?;

    let type_token = value_type_token.into_inner().next().unwrap();
    match type_token.as_rule() {
        Rule::mapping_type => {
            let (nested_key, nested_value, more_nested) = parse_mapping_type(type_token)?;
            debug_log(&format!("Found nested mapping: {} => mapping({} => {})", 
                key_type, nested_key, nested_value));
            Ok((
                key_type,
                format!("mapping({} => {})", nested_key, nested_value),
                Some(Box::new(MappingType {
                    key_type: nested_key,
                    value_type: nested_value,
                    nested: more_nested,
                }))
            ))
        }
        Rule::basic_type => {
            debug_log(&format!("Found basic mapping: {} => {}", key_type, type_token.as_str()));
            Ok((
                key_type,
                type_token.as_str().to_string(),
                None
            ))
        }
        _ => Err(anyhow!("Invalid mapping value type")),
    }
}

fn parse_function(pair: pest::iterators::Pair<Rule>) -> Result<Option<Function>> {
    let mut function = Function {
        name: String::new(),
        params: Vec::new(),
        return_type: None,
        visibility: None,
        mutability: None,
        body: Vec::new(),
    };

    for token in pair.into_inner() {
        match token.as_rule() {
            Rule::identifier => {
                function.name = token.as_str().to_string();
                debug_log(&format!("Found function name: {}", function.name));
            }
            Rule::parameter_list => {
                function.params = parse_parameters(token)?;
            }
            Rule::visibility_modifier => {
                function.visibility = Some(token.as_str().to_string());
            }
            Rule::state_mutability_modifier => {
                function.mutability = Some(token.as_str().to_string());
            }
            Rule::type_name => {
                let inner = token.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::basic_type => {
                        function.return_type = Some(inner.as_str().to_string());
                    }
                    Rule::mapping_type => {
                        let (key_type, value_type, _) = parse_mapping_type(inner)?;
                        function.return_type = Some(format!("mapping({} => {})", key_type, value_type));
                    }
                    _ => {}
                }
            }
            Rule::function_body => {
                debug_log("Parsing function body");
                function.body = parse_statements(token)?;
                debug_log(&format!("Found {} statements in function body", function.body.len()));
            }
            _ => {}
        }
    }

    if function.name.is_empty() {
        return Err(anyhow!("Function must have a name"));
    }

    Ok(Some(function))
}

fn parse_constructor(pair: pest::iterators::Pair<Rule>) -> Result<Option<Constructor>> {
    let mut constructor = Constructor {
        params: Vec::new(),
        visibility: None,
        body: Vec::new(),
    };

    for token in pair.into_inner() {
        match token.as_rule() {
            Rule::parameter_list => {
                constructor.params = parse_parameters(token)?;
            }
            Rule::visibility_modifier => {
                constructor.visibility = Some(token.as_str().to_string());
            }
            Rule::function_body => {
                constructor.body = parse_statements(token)?;
            }
            _ => {}
        }
    }

    Ok(Some(constructor))
}

fn parse_event(pair: pest::iterators::Pair<Rule>) -> Result<Option<Event>> {
    let mut event = Event {
        name: String::new(),
        params: Vec::new(),
    };

    for token in pair.into_inner() {
        match token.as_rule() {
            Rule::identifier => {
                event.name = token.as_str().to_string();
            }
            Rule::event_parameter_list => {
                for param in token.into_inner() {
                    if let Rule::event_parameter = param.as_rule() {
                        let mut param_type = String::new();
                        let mut param_name = String::new();
                        let mut indexed = false;

                        for param_token in param.into_inner() {
                            match param_token.as_rule() {
                                Rule::type_name => {
                                    let type_token = param_token.into_inner().next().unwrap();
                                    param_type = type_token.as_str().to_string();
                                }
                                Rule::indexed_modifier => {
                                    indexed = true;
                                }
                                Rule::identifier => {
                                    param_name = param_token.as_str().to_string();
                                }
                                _ => {}
                            }
                        }

                        event.params.push(EventParameter {
                            name: param_name,
                            param_type,
                            indexed,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    Ok(Some(event))
}

fn parse_parameters(pair: pest::iterators::Pair<Rule>) -> Result<Vec<Parameter>> {
    let mut params = Vec::new();

    for param in pair.into_inner() {
        if let Rule::parameter = param.as_rule() {
            let mut param_type = String::new();
            let mut param_name = String::new();

            for token in param.into_inner() {
                match token.as_rule() {
                    Rule::type_name => {
                        let type_token = token.into_inner().next().unwrap();
                        match type_token.as_rule() {
                            Rule::basic_type => {
                                param_type = type_token.as_str().to_string();
                            }
                            Rule::mapping_type => {
                                let (key_type, value_type, _) = parse_mapping_type(type_token)?;
                                param_type = format!("mapping({} => {})", key_type, value_type);
                            }
                            _ => {}
                        }
                    }
                    Rule::identifier => {
                        param_name = token.as_str().to_string();
                    }
                    _ => {}
                }
            }

            params.push(Parameter {
                name: param_name,
                param_type,
            });
        }
    }

    Ok(params)
}

fn parse_statements(pair: pest::iterators::Pair<Rule>) -> Result<Vec<Statement>> {
    let mut statements = Vec::new();

    for stmt in pair.into_inner() {
        match stmt.as_rule() {
            Rule::assignment_statement => {
                let mut tokens = stmt.into_inner();

                let index_access = tokens.next()
                    .ok_or_else(|| anyhow!("Assignment target not found"))?;
                let expr = tokens.next()
                    .ok_or_else(|| anyhow!("Assignment value not found"))?;

                debug_log(&format!("Parsing assignment: {} = <expr>", index_access.as_str()));

                let target = parse_index_access(index_access)?;
                match target {
                    Expression::Identifier(id) => {
                        statements.push(Statement::Assignment(
                            id,
                            parse_expression(expr)?
                        ));
                    }
                    Expression::MapAccess(map, key) => {
                        statements.push(Statement::MapAccessAssignment(
                            map,
                            key,
                            parse_expression(expr)?
                        ));
                    }
                    _ => return Err(anyhow!("Invalid assignment target")),
                }
            }
            Rule::emit_statement => {
                let mut tokens = stmt.into_inner();
                let event_name = tokens.next()
                    .ok_or_else(|| anyhow!("Event name not found"))?;

                let mut args = Vec::new();
                if let Some(arg_list) = tokens.next() {
                    for arg in arg_list.into_inner() {
                        args.push(parse_expression(arg)?);
                    }
                }

                debug_log(&format!("Parsing emit: {} with {} args", event_name.as_str(), args.len()));
                statements.push(Statement::Emit(
                    event_name.as_str().to_string(),
                    args
                ));
            }
            Rule::return_statement => {
                if let Some(expr) = stmt.into_inner().next() {
                    debug_log("Parsing return statement with expression");
                    statements.push(Statement::Return(parse_expression(expr)?));
                }
            }
            Rule::expression_statement => {
                if let Some(expr) = stmt.into_inner().next() {
                    debug_log("Parsing expression statement");
                    statements.push(Statement::Expression(parse_expression(expr)?));
                }
            }
            _ => {}
        }
    }

    debug_log(&format!("Parsed {} statements", statements.len()));
    Ok(statements)
}

fn parse_expression(pair: pest::iterators::Pair<Rule>) -> Result<Expression> {
    debug_log(&format!("Parsing expression: {}", pair.as_str()));
    match pair.as_rule() {
        Rule::expression => {
            let mut tokens = pair.into_inner().peekable();

            let first = tokens.next()
                .ok_or_else(|| anyhow!("Expression must have at least one term"))?;
            let mut expr = parse_term(first)?;

            while let Some(op) = tokens.next() {
                if let Some(term) = tokens.next() {
                    debug_log(&format!("Found binary operator: {}", op.as_str()));
                    expr = Expression::BinaryOp(
                        Box::new(expr),
                        op.as_str().to_string(),
                        Box::new(parse_term(term)?)
                    );
                }
            }
            Ok(expr)
        }
        Rule::term | Rule::primary => parse_term(pair),
        _ => parse_term(pair),
    }
}

fn parse_term(pair: pest::iterators::Pair<Rule>) -> Result<Expression> {
    debug_log(&format!("Parsing term: {}", pair.as_str()));
    match pair.as_rule() {
        Rule::index_access => parse_index_access(pair),
        Rule::literal => Ok(Expression::Literal(pair.as_str().to_string())),
        Rule::primary => {
            let inner = pair.into_inner().next()
                .ok_or_else(|| anyhow!("Invalid primary expression"))?;
            parse_term(inner)
        }
        Rule::term => {
            let inner = pair.into_inner().next()
                .ok_or_else(|| anyhow!("Invalid term"))?;
            parse_term(inner)
        }
        _ => Err(anyhow!("Unexpected term type: {:?}", pair.as_rule())),
    }
}

fn parse_index_access(pair: pest::iterators::Pair<Rule>) -> Result<Expression> {
    let mut tokens = pair.into_inner();

    let member_access = tokens.next()
        .ok_or_else(|| anyhow!("Expected member access in index access"))?;
    let mut expr = parse_member_access(member_access)?;

    // Process any array/map access expressions that follow
    for access in tokens {
        if let Some(index_expr) = access.into_inner().next() {
            let index = parse_expression(index_expr)?;
            match expr {
                Expression::Identifier(map_name) => {
                    expr = Expression::MapAccess(
                        map_name,
                        Box::new(index)
                    );
                }
                Expression::MapAccess(map_name, prev_key) => {
                    // For nested access, create a new MapAccess with a composite key
                    expr = Expression::MapAccess(
                        map_name,
                        Box::new(Expression::BinaryOp(
                            prev_key,
                            ",".to_string(),
                            Box::new(index)
                        ))
                    );
                }
                _ => return Err(anyhow!("Invalid nested map access")),
            }
        }
    }

    Ok(expr)
}

fn parse_member_access(pair: pest::iterators::Pair<Rule>) -> Result<Expression> {
    let mut tokens = pair.into_inner();

    let first = tokens.next()
        .ok_or_else(|| anyhow!("Expected identifier in member access"))?;
    let mut expr = Expression::Identifier(first.as_str().to_string());

    // Process any subsequent member accesses
    for member in tokens {
        if member.as_rule() == Rule::identifier {
            expr = Expression::MemberAccess(
                Box::new(expr),
                member.as_str().to_string()
            );
        }
    }

    Ok(expr)
}