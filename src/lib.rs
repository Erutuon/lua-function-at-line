use full_moon::{
    ast::{
        AstError, Block, Call, Expression, FunctionArgs, Index, Prefix, Stmt,
        Suffix, Value, Var,
    },
    tokenizer::{TokenReference, TokenType},
};
use itertools::{EitherOrBoth, Itertools};
use std::{borrow::Cow, fmt::Write};

mod traits;
use traits::FirstToken;

fn unexpected_token<'a>(
    token_ref: &'a TokenReference<'a>,
    msg: &'_ str,
) -> AstError<'a> {
    AstError::UnexpectedToken {
        token: token_ref.token().to_owned(),
        additional: Some(msg.to_owned().into()),
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct FunctionSpan {
    pub start: usize,
    pub end: usize,
    pub name: Option<String>,
}

fn remove_trivia<'a>(token_ref: &'a TokenReference<'a>) -> TokenReference<'a> {
    TokenReference::new(vec![], token_ref.token().to_owned(), vec![])
}

enum FunctionName<'a> {
    Identifier(&'a TokenReference<'a>),
    Complex(&'a Var<'a>),
}

impl<'a> From<&'a Cow<'a, TokenReference<'a>>> for FunctionName<'a> {
    fn from(token: &'a Cow<'a, TokenReference<'a>>) -> Self {
        FunctionName::Identifier(token.as_ref())
    }
}

impl<'a> From<&'a Var<'a>> for FunctionName<'a> {
    fn from(var: &'a Var<'a>) -> Self {
        FunctionName::Complex(var)
    }
}

fn var_to_string<'a>(var: &'a Var<'a>) -> Result<String, AstError<'a>> {
    match var {
        Var::Expression(expr) => {
            let prefix = match expr.prefix() {
                Prefix::Name(name) => name.token(),
                Prefix::Expression(expr) => {
                    return Err(unexpected_token(
                        expr.first_token(),
                        "expected identifier",
                    ));
                }
            };
            let mut var = prefix.to_string();
            for suffix in expr.iter_suffixes() {
                if let Suffix::Index(index) = suffix {
                    match index {
                        Index::Brackets { expression, .. } => {
                            // Remove some whitespace and wrapping parentheses from some types of expression.
                            let mut expr = expression;
                            while let Expression::Parentheses {
                                expression,
                                ..
                            } = expression
                            {
                                expr = expression;
                            }
                            while let Expression::Value { value, binop: None } =
                                expr
                            {
                                if let Value::ParseExpression(expression) =
                                    value.as_ref()
                                {
                                    expr = expression;
                                } else {
                                    break;
                                }
                            }
                            if let Expression::Value { value, binop: None } =
                                expr
                            {
                                if let Value::String(token)
                                | Value::Number(token)
                                | Value::Symbol(token) = value.as_ref()
                                {
                                    write!(var, "[{}]", remove_trivia(token))
                                        .unwrap();
                                } else {
                                    write!(var, "[{}]", value).unwrap();
                                }
                            } else {
                                write!(var, "[{}]", expression).unwrap();
                            }
                        }
                        Index::Dot { name, .. } => {
                            if let TokenType::Identifier {
                                identifier, ..
                            } = name.token_type()
                            {
                                write!(var, ".{}", identifier).unwrap();
                            } else {
                                return Err(unexpected_token(
                                    suffix.first_token(),
                                    "expected identifier",
                                ));
                            }
                        }
                    }
                } else {
                    return Err(unexpected_token(
                        suffix.first_token(),
                        "expected indexing brackets",
                    ));
                }
            }
            Ok(var)
        }
        Var::Name(name) => {
            if let TokenType::Identifier { identifier, .. } = name.token_type()
            {
                Ok(identifier.to_string())
            } else {
                Err(unexpected_token(name, "expected identifier"))
            }
        }
    }
}

impl<'a> FunctionName<'a> {
    fn to_string(&self) -> Result<String, AstError<'a>> {
        match self {
            FunctionName::Identifier(token) => {
                if let TokenType::Identifier { identifier } = token.token_type()
                {
                    Ok(identifier.to_string())
                } else {
                    Err(unexpected_token(token, "expected identifier"))
                }
            }
            FunctionName::Complex(var) => var_to_string(var),
        }
    }
}

fn process_suffixes<'a>(
    suffixes: impl Iterator<Item = &'a Suffix<'a>> + 'a,
    functions: &mut Vec<FunctionSpan>,
) -> Result<(), AstError<'a>> {
    for suffix in suffixes {
        if let Suffix::Call(call) = suffix {
            let args = match call {
                Call::AnonymousCall(args) => args,
                Call::MethodCall(call) => call.args(),
            };
            match args {
                FunctionArgs::Parentheses { arguments, .. } => {
                    for expr in arguments {
                        process_expression(None, expr, functions)?;
                    }
                }
                FunctionArgs::TableConstructor(_) => {
                    todo!("handle functions in table constructors")
                }
                FunctionArgs::String(_) => {}
            }
        }
    }
    Ok(())
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
                    process_expression(
                        var.filter(|_| binop.is_none()),
                        expr,
                        functions,
                    )?;
                }
                Value::FunctionCall(call) => {
                    process_suffixes(call.iter_suffixes(), functions)?;
                }
                Value::TableConstructor(_) => {
                    todo!("handle functions in table constructors")
                }
                Value::Var(var) => {
                    if let Var::Expression(expr) = var {
                        process_suffixes(expr.iter_suffixes(), functions)?;
                    }
                }
                Value::Number(_) => {}
                Value::String(_) => {}
                Value::Symbol(_) => {}
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

fn process_assignment<
    'a,
    N: Iterator<Item = T>,
    E: Iterator<Item = &'a Expression<'a>>,
    T: Into<FunctionName<'a>> + 'a,
>(
    name_list: N,
    expr_list: E,
    functions: &mut Vec<FunctionSpan>,
) -> Result<(), AstError<'a>> {
    for item in name_list.into_iter().zip_longest(expr_list.into_iter()) {
        let (name, expr) = match item {
            EitherOrBoth::Both(var, expr) => (Some(var.into()), expr),
            EitherOrBoth::Right(expr) => (None, expr),
            EitherOrBoth::Left(_) => continue,
        };
        process_expression(name, expr, functions)?;
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
                            return Err(unexpected_token(token, "expected identifier in dot-separated function name"));
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
                        return Err(unexpected_token(
                            method,
                            "expected identifier as method name",
                        ));
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
                process_assignment(
                    asgn.var_list().iter(),
                    asgn.expr_list().iter(),
                    functions,
                )?;
            }
            // Todo: process expressions with no variable name.
            Stmt::LocalAssignment(asgn) => {
                process_assignment(
                    asgn.name_list().iter(),
                    asgn.expr_list().iter(),
                    functions,
                )?;
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
