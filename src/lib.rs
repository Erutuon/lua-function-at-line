#![allow(dead_code, unused)]
use full_moon::{
    ast::{
        AstError, Block, Call, Expression, FunctionArgs, Index, Prefix, Stmt,
        Suffix, UnOp, Value, Var,
    },
    tokenizer::{TokenReference, TokenType},
};
use std::fmt::Write;

mod traits;
use traits::FirstToken;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct FunctionSpan {
    pub start: usize,
    pub end: usize,
    pub name: Option<String>,
}

enum FunctionName<'a> {
    Identifier(&'a TokenReference<'a>),
    Complex(&'a Var<'a>),
}

impl<'a> FunctionName<'a> {
    fn to_string(&self) -> Result<String, AstError<'a>> {
        match self {
            FunctionName::Identifier(token) => {
                if let TokenType::Identifier { identifier } = token.token_type()
                {
                    Ok(identifier.to_string())
                } else {
                    Err(AstError::UnexpectedToken {
                        token: token.token().to_owned(),
                        additional: Some("expected identifier".into()),
                    })
                }
            }
            FunctionName::Complex(var) => {
                match var {
                    Var::Expression(expr) => {
                        let prefix = match expr.prefix() {
                            Prefix::Name(name) => name,
                            Prefix::Expression(expr) => {
                                return Err(AstError::UnexpectedToken {
                                    token: expr
                                        .first_token()
                                        .token()
                                        .to_owned(),
                                    additional: Some(
                                        "expected identifier".into(),
                                    ),
                                })
                            }
                        };
                        let mut var = prefix.to_string();
                        for suffix in expr.iter_suffixes() {
                            if let Suffix::Index(index) = suffix {
                                match index {
                                    Index::Brackets { expression, .. } => {
                                        // Don't bother removing whitespace from expression.
                                        write!(var, "[{}]", expression);
                                    }
                                    Index::Dot { name, .. } => {
                                        if let TokenType::Identifier {
                                            identifier,
                                            ..
                                        } = name.token_type()
                                        {
                                            write!(var, ".{}", identifier);
                                        } else {
                                            return Err(
                                                AstError::UnexpectedToken {
                                                    token: suffix
                                                        .first_token()
                                                        .token()
                                                        .to_owned(),
                                                    additional: Some(
                                                        "expected identifier"
                                                            .into(),
                                                    ),
                                                },
                                            );
                                        }
                                    }
                                }
                            } else {
                                return Err(AstError::UnexpectedToken {
                                    token: suffix
                                        .first_token()
                                        .token()
                                        .to_owned(),
                                    additional: Some(
                                        "expected indexing brackets".into(),
                                    ),
                                });
                            }
                        }
                        Ok(var)
                    }
                    Var::Name(name) => {
                        if let TokenType::Identifier { identifier, .. } =
                            name.token_type()
                        {
                            Ok(identifier.to_string())
                        } else {
                            Err(AstError::UnexpectedToken {
                                token: name.token().to_owned(),
                                additional: Some("expected identifier".into()),
                            })
                        }
                    }
                }
            }
        }
    }
}

fn process_expression<'a, 'b>(
    var: Option<FunctionName<'a>>,
    mut expr: &'a Expression<'a>,
    functions: &'b mut Vec<FunctionSpan>,
) -> Result<(), AstError<'a>> {
    // Strip off layers of parentheses.
    while let Expression::Parentheses { expression, .. } = expr {
        // eprintln!("var: {}, expression: {}", var.as_ref().map(|v| v.to_string().unwrap()).unwrap_or("?".to_string()), expression);
        expr = expression.as_ref();
    }
    match expr {
        Expression::Value { value, binop } => {
            match value.as_ref() {
                Value::Function((keyword, body)) => {
                    let start = keyword.start_position().line();
                    let end = body.end_token().end_position().line();
                    let var = var
                        .filter(|_| binop.is_none())
                        .map(|var| var.to_string())
                        .transpose()?;
                    functions.push(FunctionSpan {
                        start,
                        end,
                        name: var,
                    });
                    gather_function_line_spans(body.block(), functions)?;
                }
                Value::ParseExpression(expr) => {
                    process_expression(var.filter(|_| binop.is_none()), expr, functions)?;
                }
                Value::FunctionCall(call) => {
                    for suffix in call.iter_suffixes() {
                        if let Suffix::Call(call) = suffix {
                            let args = match call {
                                Call::AnonymousCall(args) => args,
                                Call::MethodCall(call) => call.args(),
                            };
                            match args {
                                FunctionArgs::Parentheses {
                                    parentheses,
                                    arguments,
                                } => {
                                    for expr in arguments {
                                        process_expression(
                                            None, expr, functions,
                                        )?;
                                    }
                                }
                                FunctionArgs::TableConstructor(_) => todo!(
                                    "handle functions in table constructors"
                                ),
                                FunctionArgs::String(_) => {}
                            }
                        }
                    }
                }
                Value::TableConstructor(_) => {
                    todo!("handle functions in table constructors")
                }
                Value::Number(_) => {}
                Value::String(_) => {}
                Value::Symbol(_) => {}
                Value::Var(_) => todo!("handle var"),
            }
            if let Some(tail) = binop {
                process_expression(None, tail.rhs(), functions)?;
            }
        }
        Expression::UnaryOperator { expression, .. } => {
            process_expression(None, expression, functions)?;
        }
        Expression::Parentheses { .. } => unreachable!(),
    }
    Ok(())
}

pub fn gather_function_line_spans<'a, 'b>(
    block: &'a Block<'a>,
    functions: &'b mut Vec<FunctionSpan>,
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
                            name: Some(identifier.to_string()),
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
                    name: Some(formatted_name),
                    start,
                    end,
                });
                gather_function_line_spans(func.body().block(), functions)?;
            }
            // Todo: process expressions with no variable name.
            Stmt::Assignment(asgn) => {
                for (var, mut expr) in
                    asgn.var_list().iter().zip(asgn.expr_list())
                {
                    process_expression(
                        Some(FunctionName::Complex(var)),
                        expr,
                        functions,
                    )?;
                }
            }
            // Todo: process expressions with no variable name.
            Stmt::LocalAssignment(asgn) => {
                for (name, expr) in
                    asgn.name_list().iter().zip(asgn.expr_list())
                {
                    process_expression(
                        Some(FunctionName::Identifier(name)),
                        expr,
                        functions,
                    )?;
                }
            }
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
mod tests;
