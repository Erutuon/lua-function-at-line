use full_moon::{
    ast::{AstError, Block, Expression, Stmt, Var, Value},
    tokenizer::TokenType,
};

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct FunctionSpan {
    pub start: usize,
    pub end: usize,
    pub name: String,
}

pub fn process_vars_and_exprs<'a>(
    iter: impl IntoIterator<Item = (&'a Var<'a>, &'a Expression<'a>)>,
    functions: &mut Vec<FunctionSpan>,
) {
    for (var, expr) in iter {
        match expr {
            Expression::Parentheses {
                contained,
                expression,
            } => {
                if let Expression::Value { value, binop: None } = &**expression {
                    if let Value::Function((_, body)) = &**value {
                        
                    }
                }
            }
            Expression::UnaryOperator { unop, expression } => {}
            Expression::Value { value, binop } => {}
        }
    }
}

pub fn gather_function_line_spans<'a>(
    block: &'a Block<'a>,
    functions: &mut Vec<FunctionSpan>,
) -> Result<(), AstError<'a>> {
    for statement in block.iter_stmts() {
        match statement {
            Stmt::LocalFunction(func) => {
                let name = func.name();
                match name.token_type() {
                    TokenType::Identifier { identifier } => {
                        let start = func.local_token().start_position().line();
                        let end =
                            func.func_body().end_token().end_position().line();
                        functions.push(FunctionSpan {
                            name: identifier.to_string(),
                            start,
                            end,
                        });
                    }
                    _ => (),
                };
                gather_function_line_spans(
                    func.func_body().block(),
                    functions,
                )?;
            }
            Stmt::FunctionDeclaration(func) => {
                let name = func.name();
                let mut formatted_name = name
                    .names()
                    .pairs()
                    .map(|pair| {
                        let token = match pair {
                            full_moon::ast::punctuated::Pair::End(token) => {
                                token
                            }
                            full_moon::ast::punctuated::Pair::Punctuated(
                                token,
                                // This can be ignored because it should always be a dot.
                                _sep,
                            ) => token,
                        };
                        if let TokenType::Identifier { identifier } =
                            token.token_type()
                        {
                            Ok(identifier.as_ref())
                        } else {
                            return Err(AstError::UnexpectedToken {
                                token: token.token().to_owned(),
                                additional: Some("expected identifier in dot-separated function name".into()),
                            });
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(".");
                if let Some(method) = name.method_name() {
                    if let TokenType::Identifier { identifier } =
                        method.token_type()
                    {
                        formatted_name.push(':');
                        formatted_name.push_str(identifier);
                    } else {
                        return Err(AstError::UnexpectedToken {
                            token: method.token().to_owned(),
                            additional: Some(
                                "expected identifier as method name".into(),
                            ),
                        });
                    }
                }
                let start = func.function_token().start_position().line();
                let end = func.body().end_token().end_position().line();
                functions.push(FunctionSpan {
                    name: formatted_name,
                    start,
                    end,
                });
                gather_function_line_spans(func.body().block(), functions)?;
            }
            Stmt::Assignment(asgn) => {
                let (exprs, vars) = (asgn.var_list(), asgn.expr_list());
                process_vars_and_exprs(exprs.iter().zip(vars), functions);
            }
            Stmt::LocalAssignment(_) => {}
            Stmt::Do(do_stmt) => {
                gather_function_line_spans(do_stmt.block(), functions)?;
            }
            Stmt::FunctionCall(_) => {}
            Stmt::GenericFor(for_stmt) => {
                gather_function_line_spans(for_stmt.block(), functions)?;
            }
            Stmt::If(if_stmt) => {
                gather_function_line_spans(if_stmt.block(), functions)?;
                if let Some(blocks) = if_stmt.else_if() {
                    for block in blocks {
                        gather_function_line_spans(block.block(), functions)?;
                    }
                }
                if let Some(block) = if_stmt.else_block() {
                    gather_function_line_spans(block, functions)?;
                }
            }
            Stmt::NumericFor(for_stmt) => {
                gather_function_line_spans(for_stmt.block(), functions)?;
            }
            Stmt::Repeat(repeat_stmt) => {
                gather_function_line_spans(repeat_stmt.block(), functions)?;
            }
            Stmt::While(while_stmt) => {
                gather_function_line_spans(while_stmt.block(), functions)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{gather_function_line_spans, FunctionSpan};
    use full_moon::parse;

    fn check_result(code: &str, expected: &[FunctionSpan]) {
        let mut functions = Vec::new();
        let code = parse(code).unwrap();
        gather_function_line_spans(&code.nodes(), &mut functions).unwrap();
        assert_eq!(&functions, &expected);
    }

    #[test]
    fn top_level_functions() {
        check_result(
            &"local function first_do() end
            function then_do() end",
            &[
                FunctionSpan {
                    start: 1,
                    end: 1,
                    name: "first_do".into(),
                },
                FunctionSpan {
                    start: 2,
                    end: 2,
                    name: "then_do".into(),
                },
            ],
        );
    }

    #[test]
    fn local_function_in_local_function() {
        check_result(
            &"local function add(y)
                local function inner()
                end
                return x + y
            end",
            &[
                FunctionSpan {
                    start: 1,
                    end: 5,
                    name: "add".into(),
                },
                FunctionSpan {
                    start: 2,
                    end: 3,
                    name: "inner".into(),
                },
            ],
        );
    }

    #[test]
    fn function_with_fields_in_function_with_fields() {
        check_result(
            &"function x.y:z()
                function a.b.c()
                    local var = const;
                end
            end",
            &[
                FunctionSpan {
                    start: 1,
                    end: 5,
                    name: "x.y:z".into(),
                },
                FunctionSpan {
                    start: 2,
                    end: 4,
                    name: "a.b.c".into(),
                },
            ],
        );
    }

    #[test]
    fn spread_out_method_or_function_calls_are_compacted() {
        check_result(
            &"function
            
            very
            .
            spread
            :
            out()
            end
            
            function
            
                very
                    .
                        indented
                            ()
                        end",
            &[
                FunctionSpan {
                    start: 1,
                    end: 8,
                    name: "very.spread:out".into(),
                },
                FunctionSpan {
                    start: 10,
                    end: 16,
                    name: "very.indented".into(),
                },
            ],
        );
    }
}
