use pest::Parser;
use pest_derive::Parser;
use anyhow::{Result, Context, anyhow};
use crate::transpiler::ast::*;

#[derive(Parser)]
#[grammar = "parser/solidity.pest"]
pub struct SolidityParser;

pub fn parse(source: &str) -> Result<Contract> {
    let pairs = SolidityParser::parse(Rule::contract, source)
        .with_context(|| "Failed to parse Solidity contract, syntax error")?;

    let mut contract = Contract {
        name: String::new(),
        functions: Vec::new(),
        state_variables: Vec::new(),
        events: Vec::new(),
        constructor: None,
    };

    if let Some(contract_pair) = pairs.into_iter().next() {
        for pair in contract_pair.into_inner() {
            match pair.as_rule() {
                Rule::contract_declaration => {
                    let mut inner_pairs = pair.into_inner();
                    if let Some(name_pair) = inner_pairs.find(|p| p.as_rule() == Rule::identifier) {
                        contract.name = name_pair.as_str().to_string();
                    } else {
                        return Err(anyhow!("Contract name not found"));
                    }

                    if let Some(body_pair) = inner_pairs.find(|p| p.as_rule() == Rule::contract_body) {
                        parse_contract_body(&mut contract, body_pair)?;
                    }
                }
                _ => continue,
            }
        }
    } else {
        return Err(anyhow!("Invalid contract structure"));
    }

    Ok(contract)
}

fn parse_contract_body(contract: &mut Contract, pair: pest::iterators::Pair<Rule>) -> Result<()> {
    println!("Parsing contract body");
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::function_definition => {
                println!("Found function definition");
                let first_token = item.clone().into_inner().next()
                    .ok_or_else(|| anyhow!("Empty function definition"))?;

                if first_token.as_rule() == Rule::constructor_definition {
                    if let Some(constructor) = parse_constructor(first_token)? {
                        contract.constructor = Some(constructor);
                    }
                } else if let Some(function) = parse_function(item)? {
                    contract.functions.push(function);
                }
            }
            Rule::state_variable_declaration => {
                println!("Found state variable declaration");
                if let Some(var) = parse_state_variable(item)? {
                    contract.state_variables.push(var);
                }
            }
            Rule::event_definition => {
                println!("Found event definition");
                if let Some(event) = parse_event(item)? {
                    contract.events.push(event);
                }
            }
            _ => {}
        }
    }
    Ok(())
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

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::regular_function_definition => {
                // Process the regular function definition
                for token in item.into_inner() {
                    match token.as_rule() {
                        Rule::identifier => {
                            function.name = token.as_str().to_string();
                            println!("Found function name: {}", function.name);
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
                        Rule::return_type => {
                            let type_pair = token.into_inner().next()
                                .ok_or_else(|| anyhow!("Invalid return type"))?;
                            function.return_type = Some(type_pair.as_str().to_string());
                        }
                        Rule::function_body => {
                            println!("Parsing function body");
                            function.body = parse_statements(token)?;
                            println!("Function body statements: {:?}", function.body);
                        }
                        _ => {}
                    }
                }
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

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::parameter_list => {
                constructor.params = parse_parameters(item)?;
            }
            Rule::visibility_modifier => {
                constructor.visibility = Some(item.as_str().to_string());
            }
            Rule::function_body => {
                constructor.body = parse_statements(item)?;
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

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::identifier => {
                event.name = item.as_str().to_string();
            }
            Rule::event_parameter_list => {
                for param in item.into_inner() {
                    if let Rule::event_parameter = param.as_rule() {
                        let tokens: Vec<_> = param.into_inner().collect();

                        // First token is always the type
                        let param_type = tokens[0].as_str().to_string();

                        // Check if we have an indexed modifier
                        let (indexed, name) = match tokens.len() {
                            3 => (true, tokens[2].as_str().to_string()),
                            2 => (false, tokens[1].as_str().to_string()),
                            _ => return Err(anyhow!("Invalid event parameter format")),
                        };

                        event.params.push(EventParameter {
                            name,
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
        if param.as_rule() == Rule::parameter {
            let mut param_iter = param.into_inner();
            let param_type = param_iter.next()
                .ok_or_else(|| anyhow!("Parameter type not found"))?
                .as_str().to_string();
            let param_name = param_iter.next()
                .ok_or_else(|| anyhow!("Parameter name not found"))?
                .as_str().to_string();
            params.push(Parameter {
                name: param_name,
                param_type,
            });
        }
    }

    Ok(params)
}

fn parse_statements(pair: pest::iterators::Pair<Rule>) -> Result<Vec<Statement>> {
    println!("Starting parse_statements");
    let mut statements = Vec::new();

    for stmt in pair.into_inner() {
        println!("Processing statement of type: {:?}", stmt.as_rule());
        // Get the actual statement type from inside the statement node
        let inner_stmt = match stmt.into_inner().next() {
            Some(inner) => inner,
            None => continue,
        };

        match inner_stmt.as_rule() {
            Rule::assignment_statement => {
                println!("Found assignment statement");
                let mut inner = inner_stmt.into_inner();
                let identifier = inner.next()
                    .ok_or_else(|| anyhow!("Assignment target not found"))?;
                let expr = inner.next()
                    .ok_or_else(|| anyhow!("Assignment value not found"))?;

                println!("Assignment: {} = <expr>", identifier.as_str());
                statements.push(Statement::Assignment(
                    identifier.as_str().to_string(),
                    parse_expression(expr)?
                ));
            }
            Rule::emit_statement => {
                println!("Found emit statement");
                let mut inner = inner_stmt.into_inner();
                let event_name = inner.next()
                    .ok_or_else(|| anyhow!("Event name not found"))?;

                let mut args = Vec::new();
                if let Some(arg_list) = inner.next() {
                    for arg in arg_list.into_inner() {
                        args.push(parse_expression(arg)?);
                    }
                }

                statements.push(Statement::Emit(
                    event_name.as_str().to_string(),
                    args
                ));
            }
            Rule::return_statement => {
                println!("Found return statement");
                if let Some(expr) = inner_stmt.into_inner().next() {
                    statements.push(Statement::Return(parse_expression(expr)?));
                }
            }
            Rule::expression_statement => {
                println!("Found expression statement");
                if let Some(expr) = inner_stmt.into_inner().next() {
                    statements.push(Statement::Expression(parse_expression(expr)?));
                }
            }
            _ => {
                println!("Found unknown statement type: {:?}", inner_stmt.as_rule());
            }
        }
    }

    println!("Finished parsing statements, count: {}", statements.len());
    Ok(statements)
}

fn parse_expression(pair: pest::iterators::Pair<Rule>) -> Result<Expression> {
    println!("Parsing expression: {:?}", pair.as_rule());
    match pair.as_rule() {
        Rule::expression => {
            let mut pairs = pair.into_inner().peekable();

            // Parse first term
            let first = pairs.next()
                .ok_or_else(|| anyhow!("Expression must have at least one term"))?;
            let mut expr = parse_term(first)?;

            // Process operators and additional terms
            while let Some(op_pair) = pairs.next() {
                if let Some(term_pair) = pairs.next() {
                    expr = Expression::BinaryOp(
                        Box::new(expr),
                        op_pair.as_str().to_string(),
                        Box::new(parse_term(term_pair)?)
                    );
                }
            }
            Ok(expr)
        }
        Rule::term | Rule::primary => parse_term(pair),
        _ => {
            println!("Attempting to parse unknown expression type: {:?}", pair.as_rule());
            parse_term(pair)
        }
    }
}

fn parse_term(pair: pest::iterators::Pair<Rule>) -> Result<Expression> {
    println!("Parsing term: {:?}", pair.as_rule());
    match pair.as_rule() {
        Rule::identifier => {
            println!("Found identifier: {}", pair.as_str());
            Ok(Expression::Identifier(pair.as_str().to_string()))
        }
        Rule::literal => {
            println!("Found literal: {}", pair.as_str());
            Ok(Expression::Literal(pair.as_str().to_string()))
        }
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
        _ => {
            println!("Unexpected term type: {:?}", pair.as_rule());
            Err(anyhow!("Unexpected term type: {:?}", pair.as_rule()))
        }
    }
}

fn parse_state_variable(pair: pest::iterators::Pair<Rule>) -> Result<Option<StateVariable>> {
    let mut inner = pair.into_inner();

    let mut visibility = None;
    let mut var_type = String::new();
    let mut var_name = String::new();
    let mut is_mapping = false;
    let mut mapping_key_type = None;
    let mut mapping_value_type = None;
    let mut is_constant = false;
    let mut initial_value = None;

    while let Some(token) = inner.next() {
        match token.as_rule() {
            Rule::visibility_modifier => {
                visibility = Some(token.as_str().to_string());
            }
            Rule::constant_modifier => {
                is_constant = true;
            }
            Rule::mapping_declaration => {
                is_mapping = true;
                let mut mapping_iter = token.into_inner();
                if let (Some(key_type), Some(value_type)) = (mapping_iter.next(), mapping_iter.next()) {
                    mapping_key_type = Some(key_type.as_str().to_string());
                    mapping_value_type = Some(value_type.as_str().to_string());
                }
            }
            Rule::type_name => {
                if !is_mapping {
                    var_type = token.as_str().to_string();
                }
            }
            Rule::identifier => {
                if var_name.is_empty() {
                    var_name = token.as_str().to_string();
                }
            }
            Rule::expression => {
                initial_value = Some(parse_expression(token)?);
            }
            _ => {}
        }
    }

    Ok(Some(StateVariable {
        name: var_name,
        var_type,
        visibility,
        is_mapping,
        mapping_key_type,
        mapping_value_type,
        is_constant,
        initial_value: initial_value.map(|e| e.to_string()),
    }))
}