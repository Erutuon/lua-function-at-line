use std::borrow::Cow;
use full_moon::{ast::{Value, Expression, UnOp, FunctionCall, TableConstructor, span::ContainedSpan, Var, Prefix, VarExpression, Suffix, Call, FunctionArgs, Index}, tokenizer::TokenReference};

pub(crate) trait FirstToken {
    fn first_token(&self) -> &TokenReference;
}

impl<'a> FirstToken for Expression<'a> {
    fn first_token(&self) -> &TokenReference {
        match self {
            Expression::Parentheses { contained, .. } => {
                contained.first_token()
            }
            Expression::UnaryOperator { unop, .. } => {
                match unop {
                    UnOp::Minus(op) => op,
                    UnOp::Not(op) => op,
                    UnOp::Hash(op) => op,
                }
            }
            Expression::Value { value, .. } => {
                match &**value {
                    Value::Function((keyword, _)) => keyword,
                    Value::FunctionCall(call) => call.first_token(),
                    Value::TableConstructor(constructor) => {
                        constructor.first_token()
                    }
                    Value::Number(number) => number,
                    Value::ParseExpression(expr) => expr.first_token(),
                    Value::String(string) => string,
                    Value::Symbol(symbol) => symbol,
                    Value::Var(var) => var.first_token()
                }
            }
        }
    }
}

impl<'a> FirstToken for FunctionCall<'a> {
    fn first_token(&self) -> &TokenReference {
        self.prefix().first_token()
    }
}

impl<'a> FirstToken for TableConstructor<'a> {
    fn first_token(&self) -> &TokenReference {
        self.braces().first_token()
    }
}

impl<'a> FirstToken for Suffix<'a> {
    fn first_token(&self) -> &TokenReference {
        match self {
            Suffix::Call(Call::AnonymousCall(args)) => {
                match args {
                    FunctionArgs::Parentheses { parentheses, .. } => parentheses.first_token(),
                    FunctionArgs::String(string) => string,
                    FunctionArgs::TableConstructor(constructor) => constructor.first_token(),
                }
            }
            Suffix::Call(Call::MethodCall(method)) => method.colon_token(),
            Suffix::Index(index) => index.first_token(),
        }
    }
}

impl<'a> FirstToken for ContainedSpan<'a> {
    fn first_token(&self) -> &TokenReference {
        self.tokens().0
    }
}

impl<'a> FirstToken for Var<'a> {
    fn first_token(&self) -> &TokenReference {
        match self {
            Var::Expression(expr) => expr.first_token(),
            Var::Name(name) => name,
        }
    }
}

impl<'a> FirstToken for Prefix<'a> {
    fn first_token(&self) -> &TokenReference {
        match self {
            Prefix::Expression(expr) => expr.first_token(),
            Prefix::Name(name) => name,
        }
    }
}

impl<'a> FirstToken for VarExpression<'a> {
    fn first_token(&self) -> &TokenReference {
        self.prefix().first_token()
    }
}

impl<'a> FirstToken for Index<'a> {
    fn first_token(&self) -> &TokenReference {
        match self {
            Index::Brackets { brackets, .. } => brackets.first_token(),
            Index::Dot { dot, .. } => dot,
        }
    }
}