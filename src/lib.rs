use full_moon::{
    ast::BinOp,
    ast::Field,
    ast::FunctionName,
    ast::TableConstructor,
    ast::UnOp,
    ast::VarExpression,
    ast::{
        AstError, Block, Call, Expression, FunctionArgs, Index, Prefix, Stmt,
        Suffix, Value, Var,
    },
    tokenizer::Token,
    tokenizer::{TokenReference, TokenType},
ast::FunctionCall};
use itertools::{EitherOrBoth, Itertools};
use std::{
    borrow::Cow, convert::TryFrom, convert::TryInto, fmt::Display,
};
// use trace::trace;

// trace::init_depth_var!();

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

#[derive(Debug, PartialEq)]
pub struct FunctionSpan<'a> {
    pub start: usize,
    pub end: usize,
    pub name: FunctionNameStack<'a>,
}

fn remove_trivia<'a>(token_ref: &'a TokenReference<'a>) -> TokenReference<'a> {
    TokenReference::new(vec![], token_ref.token().to_owned(), vec![])
}

#[derive(Clone, Debug, PartialEq)]
enum FunctionNameSegment<'a> {
    Anonymous,
    Name(&'a Cow<'a, str>),
    Expression(Cow<'a, Expression<'a>>),
}

// impl<'a> From<Var<'a>> for FunctionNameSegment<'a> {
//     fn from(_: Var<'a>) -> Self {
//         todo!()
//     }
// }

impl<'a> From<&'a Expression<'a>> for FunctionNameSegment<'a> {
    fn from(expr: &'a Expression<'a>) -> Self {
        FunctionNameSegment::Expression(Cow::Borrowed(expr))
    }
}

impl<'a> TryFrom<&'a TokenReference<'a>> for FunctionNameSegment<'a> {
    type Error = AstError<'a>;

    fn try_from(
        token_ref: &'a TokenReference<'a>,
    ) -> Result<Self, Self::Error> {
        if let TokenType::Identifier { identifier } = token_ref.token_type() {
            Ok(FunctionNameSegment::Name(identifier))
        } else {
            Err(unexpected_token(token_ref, "expected identifier"))
        }
    }
}

impl<'a> TryFrom<TableKey<'a>> for FunctionNameSegment<'a> {
    type Error = AstError<'a>;
    fn try_from(key: TableKey<'a>) -> Result<Self, Self::Error> {
        let value = match key {
            TableKey::Positional(index) => {
                FunctionNameSegment::Expression(Cow::Owned(Expression::Value {
                    value: Box::new(Value::Number(Cow::Owned(
                        TokenReference::new(
                            vec![],
                            Token::new(TokenType::Number {
                                text: index.to_string().into(),
                            }),
                            vec![],
                        ),
                    ))),
                    binop: None,
                }))
            }
            TableKey::Name(token) => token.as_ref().try_into()?,
            TableKey::Expression(expr) => expr.into(),
        };
        Ok(value)
    }
}

// Function name:
// identifier
// optional suffixes (dot index or bracket index)
// optional method name
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionNameStack<'a> {
    first: FunctionNameSegment<'a>,
    middle: Vec<FunctionNameSegment<'a>>,
    method: Option<FunctionNameSegment<'a>>,
}

impl<'a> FunctionNameStack<'a> {
    fn new(first: FunctionNameSegment<'a>) -> Self {
        FunctionNameStack {
            first,
            middle: vec![],
            method: None,
        }
    }

    fn anonymous() -> Self {
        Self::new(FunctionNameSegment::Anonymous)
    }
}

impl<'a> FunctionNameStack<'a> {
    fn push(&mut self, segment: FunctionNameSegment<'a>) {
        self.middle.push(segment);
    }

    #[allow(unused)]
    fn pop(&mut self) -> Option<FunctionNameSegment<'a>> {
        self.middle.pop()
    }
}

impl<'a> Display for FunctionNameStack<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.first {
            FunctionNameSegment::Anonymous => write!(f, "?")?,
            FunctionNameSegment::Name(name) => write!(f, "{}", name)?,
            FunctionNameSegment::Expression(expr) => {
                if let Expression::Value { value, binop: None } = expr.as_ref()
                {
                    if let Value::String(token)
                    | Value::Number(token)
                    | Value::Symbol(token) = value.as_ref()
                    {
                        write!(f, "[{}]", remove_trivia(token))?;
                    } else {
                        write!(f, "[{}]", value)?;
                    }
                } else {
                    write!(f, "[{}]", expr)?;
                }
            }
        }
        for segment in self.middle.iter() {
            match &segment {
                FunctionNameSegment::Anonymous => {
                    // This should not happen.
                    write!(f, ".?")?
                }
                FunctionNameSegment::Name(name) => {
                    write!(f, ".{}", name)?;
                }
                FunctionNameSegment::Expression(expr) => {
                    if let Expression::Value { value, binop: None } =
                        expr.as_ref()
                    {
                        if let Value::String(token)
                        | Value::Number(token)
                        | Value::Symbol(token) = value.as_ref()
                        {
                            write!(f, "[{}]", remove_trivia(token))?;
                        } else {
                            write!(f, "[{}]", value)?;
                        }
                    } else {
                        write!(f, "[{}]", expr)?;
                    }
                }
            }
        }
        if let Some(method) = &self.method {
            // This should be infallible.
            if let FunctionNameSegment::Name(name) = method {
                write!(f, ":{}", name)?;
            }
        }
        Ok(())
    }
}

impl<'a> From<FunctionNameSegment<'a>> for FunctionNameStack<'a> {
    fn from(segment: FunctionNameSegment<'a>) -> Self {
        FunctionNameStack::new(segment)
    }
}

impl<'a> TryFrom<&'a FunctionName<'a>> for FunctionNameStack<'a> {
    type Error = AstError<'a>;

    fn try_from(name: &'a FunctionName<'a>) -> Result<Self, Self::Error> {
        let mut names = name.names().iter();
        let first = names
            .next()
            .expect("a function name must contain at least one identifier")
            .as_ref()
            .try_into()?;
        let middle = names
            .map(|id| id.as_ref().try_into())
            .collect::<Result<_, _>>()?;
        let method = name.method_name().map(TryInto::try_into).transpose()?;
        Ok(Self {
            first,
            middle,
            method,
        })
    }
}

impl<'a> TryFrom<&'a VarExpression<'a>> for FunctionNameStack<'a> {
    type Error = AstError<'a>;

    fn try_from(var_expr: &'a VarExpression<'a>) -> Result<Self, Self::Error> {
        let mut stack = match var_expr.prefix() {
            Prefix::Expression(expr) => {
                return Err(unexpected_token(
                    expr.first_token(),
                    "expected identifier",
                ))
            }
            Prefix::Name(name) => {
                if let TokenType::Identifier { identifier } = name.token_type()
                {
                    FunctionNameStack::new(FunctionNameSegment::Name(
                        identifier,
                    ))
                } else {
                    return Err(unexpected_token(name, "expected identifier"));
                }
            }
        };
        for suffix in var_expr.iter_suffixes() {
            let segment = match suffix {
                Suffix::Call(call) => {
                    return Err(unexpected_token(
                        call.first_token(),
                        "expected indexing syntax",
                    ))
                }
                Suffix::Index(index) => match index {
                    Index::Brackets { expression, .. } => {
                        FunctionNameSegment::Expression(Cow::Borrowed(
                            expression,
                        ))
                    }
                    Index::Dot { name, .. } => {
                        if let TokenType::Identifier { identifier } =
                            name.token_type()
                        {
                            FunctionNameSegment::Name(identifier)
                        } else {
                            return Err(unexpected_token(
                                name,
                                "expected identifier",
                            ));
                        }
                    }
                },
            };
            stack.push(segment);
        }
        Ok(stack)
    }
}

impl<'a> TryFrom<&'a TokenReference<'a>> for FunctionNameStack<'a> {
    type Error = AstError<'a>;

    fn try_from(
        token_ref: &'a TokenReference<'a>,
    ) -> Result<Self, Self::Error> {
        if let TokenType::Identifier { identifier } = token_ref.token_type() {
            Ok(FunctionNameStack::new(FunctionNameSegment::Name(
                identifier,
            )))
        } else {
            Err(unexpected_token(token_ref, "expected identifier"))
        }
    }
}

impl<'a> TryFrom<&'a Var<'a>> for FunctionNameStack<'a> {
    type Error = AstError<'a>;
    fn try_from(var: &'a Var<'a>) -> Result<Self, Self::Error> {
        match var {
            Var::Expression(var_expr) => var_expr.try_into(),
            Var::Name(name) => name.as_ref().try_into(),
        }
    }
}

// #[trace(disable(suffixes))]
fn process_suffixes<'a, 'b>(
    suffixes: impl Iterator<Item = &'a Suffix<'a>> + 'a,
    functions: &'b mut Vec<FunctionSpan<'a>>,
) -> Result<(), AstError<'a>> {
    for suffix in suffixes {
        if let Suffix::Call(call) = suffix {
            let args = match call {
                Call::AnonymousCall(args) => args,
                Call::MethodCall(call) => call.args(),
            };
            match args {
                FunctionArgs::Parentheses { arguments, .. } => {
                    for arg in arguments {
                        process_expression(
                            &mut FunctionNameStack::anonymous(),
                            arg,
                            functions,
                        )?;
                    }
                }
                FunctionArgs::TableConstructor(table) => {
                    process_table_constructor(&mut FunctionNameStack::anonymous(), table, functions)?;
                }
                FunctionArgs::String(_) => {}
            }
        }
    }
    Ok(())
}

// #[trace(disable(suffixes))]
fn process_function_call<'a, 'b>(
    call: &'a FunctionCall<'a>,
    functions: &'b mut Vec<FunctionSpan<'a>>,
) -> Result<(), AstError<'a>> {
    if let Prefix::Expression(expr) = call.prefix() {
        process_expression(&mut FunctionNameStack::anonymous(), expr, functions)?;
    }
    process_suffixes(call.iter_suffixes(), functions)?;
    Ok(())
}

#[derive(Clone, Debug)]
enum TableKey<'a> {
    Positional(u64),
    Name(&'a Cow<'a, TokenReference<'a>>),
    Expression(&'a Expression<'a>),
}

impl<'a> TableKey<'a> {
    fn with_value_from_field(
        field: &'a Field<'a>,
        index: &mut u64,
    ) -> (Self, &'a Expression<'a>) {
        match field {
            Field::ExpressionKey { key, value, .. } => {
                (TableKey::Expression(key), value)
            }
            Field::NameKey { key, value, .. } => (TableKey::Name(key), value),
            Field::NoKey(value) => {
                *index += 1;
                (TableKey::Positional(*index), value)
            }
        }
    }
}

// #[trace]
fn process_table_constructor<'a>(
    name: &mut FunctionNameStack<'a>,
    table: &'a TableConstructor<'a>,
    functions: &mut Vec<FunctionSpan<'a>>,
) -> Result<(), AstError<'a>> {
    let mut index = 0;
    for (key, value) in table
        .iter_fields()
        .map(|(field, _)| TableKey::with_value_from_field(field, &mut index))
    {
        if let TableKey::Expression(expr) = key {
            process_expression(&mut FunctionNameStack::anonymous(), expr, functions)?;
        }
        name.push(key.clone().try_into()?);
        process_expression(name, value, functions)?;
        name.pop();
    }
    Ok(())
}

enum UsefulExpression<'a> {
    Single(&'a Box<Value<'a>>),
    UnOp(&'a UnOp<'a>, &'a Expression<'a>),
    BinOp(&'a Box<Value<'a>>, &'a BinOp<'a>, &'a Expression<'a>),
}

fn strip_parentheses<'a>(mut expr: &'a Expression<'a>) -> UsefulExpression<'a> {
    while let Expression::Parentheses { expression, .. } = expr {
        expr = expression.as_ref();
    }
    match expr {
        Expression::Parentheses { .. } => {
            unreachable!("parentheses have been stripped")
        }
        Expression::UnaryOperator { unop, expression } => {
            UsefulExpression::UnOp(unop, expression)
        }
        Expression::Value { value, binop } => match binop {
            Some(op) => UsefulExpression::BinOp(value, op.bin_op(), op.rhs()),
            None => {
                if let Value::ParseExpression(expr) = value.as_ref() {
                    strip_parentheses(expr)
                } else {
                    UsefulExpression::Single(value)
                }
            },
        },
    }
}

// #[trace]
fn process_value<'a, 'b>(
    var: &mut FunctionNameStack<'a>,
    value: &'a Value<'a>,
    functions: &'b mut Vec<FunctionSpan<'a>>,
) -> Result<(), AstError<'a>> {
    // println!("{} = {}; {:?}", var, value, functions);
    match value {
        Value::Function((keyword, body)) => {
            let start = keyword.start_position().line();
            let end = body.end_token().end_position().line();
            functions.push(FunctionSpan {
                start,
                end,
                name: var.clone(),
            });
            gather_function_line_spans(body.block(), functions)?;
        }
        Value::ParseExpression(expr) => {
            process_expression(var, expr, functions)?;
        }
        Value::FunctionCall(call) => {
            process_function_call(call, functions)?;
        }
        Value::TableConstructor(table) => {
            process_table_constructor(var, table, functions)?;
        }
        Value::Var(var) => {
            if let Var::Expression(expr) = var {
                process_suffixes(expr.iter_suffixes(), functions)?;
            }
        }
        Value::Number(_) | Value::String(_) | Value::Symbol(_) => {}
    }
    Ok(())
}

// #[trace]
fn process_expression<'a, 'b>(
    var: &mut FunctionNameStack<'a>,
    expr: &'a Expression<'a>,
    functions: &'b mut Vec<FunctionSpan<'a>>,
) -> Result<(), AstError<'a>> {
    // println!("{} = {}", var, expr);
    let expr = strip_parentheses(expr);
    match expr {
        UsefulExpression::Single(value) => {
            process_value(var, value, functions)?;
        }
        UsefulExpression::UnOp(_, value) => {
            process_expression(
                &mut FunctionNameStack::anonymous(),
                value,
                functions,
            )?;
        }
        UsefulExpression::BinOp(left, _, right) => {
            process_value(&mut FunctionNameStack::anonymous(), left, functions)?;
            process_expression(
                &mut FunctionNameStack::anonymous(),
                right,
                functions,
            )?;
        }
    }
    Ok(())
}

// #[trace(disable(name_list, expr_list))]
fn process_assignment<
    'a,
    'b,
    N: Iterator<Item = T>,
    E: Iterator<Item = &'a Expression<'a>>,
    T: TryInto<FunctionNameStack<'a>, Error = AstError<'a>> + 'a,
>(
    name_list: N,
    expr_list: E,
    functions: &'b mut Vec<FunctionSpan<'a>>,
) -> Result<(), AstError<'a>> {
    for item in name_list.into_iter().zip_longest(expr_list.into_iter()) {
        let (mut name, expr) = match item {
            EitherOrBoth::Both(var, expr) => (var.try_into()?, expr),
            EitherOrBoth::Right(expr) => (FunctionNameStack::anonymous(), expr),
            EitherOrBoth::Left(_) => continue,
        };
        process_expression(&mut name, expr, functions)?;
    }
    Ok(())
}

pub fn gather_function_line_spans<'a, 'b>(
    block: &'a Block<'a>,
    functions: &'b mut Vec<FunctionSpan<'a>>,
) -> Result<(), AstError<'a>> {
    for statement in block.iter_stmts() {
        match statement {
            Stmt::LocalFunction(func) => {
                let start = func.local_token().start_position().line();
                let end = func.func_body().end_token().end_position().line();
                let name = func.name().try_into()?;
                functions.push(FunctionSpan { name, start, end });
                gather_function_line_spans(
                    func.func_body().block(),
                    functions,
                )?;
            }
            Stmt::FunctionDeclaration(func) => {
                let start = func.function_token().start_position().line();
                let end = func.body().end_token().end_position().line();
                functions.push(FunctionSpan {
                    name: func.name().try_into()?,
                    start,
                    end,
                });
                gather_function_line_spans(func.body().block(), functions)?;
            }
            Stmt::Assignment(asgn) => {
                process_assignment(
                    asgn.var_list().iter(),
                    asgn.expr_list().iter(),
                    functions,
                )?;
            }
            Stmt::LocalAssignment(asgn) => {
                process_assignment(
                    asgn.name_list().iter().map(|name| name.as_ref()),
                    asgn.expr_list().iter(),
                    functions,
                )?;
            }
            Stmt::FunctionCall(call) => {
                process_function_call(call, functions)?;
            }
            Stmt::GenericFor(for_stmt) => {
                gather_function_line_spans(for_stmt.block(), functions)?;
            }
            Stmt::Do(do_stmt) => {
                gather_function_line_spans(do_stmt.block(), functions)?;
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
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
