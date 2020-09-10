use crate::{gather_function_line_spans, FunctionSpan, FunctionNameSegment};
use full_moon::parse;

#[derive(Debug, Eq, PartialEq)]
struct Function {
    start: usize,
    end: usize,
    name: Option<String>,
}

fn check_result(code: &str, expected: &[Function]) {
    let mut function_spans = Vec::new();
    let code = parse(code).unwrap();
    gather_function_line_spans(&code.nodes(), &mut function_spans).unwrap();
    let functions = function_spans.into_iter().map(|FunctionSpan { start, end, name }: FunctionSpan| {
        let name = if name.first == FunctionNameSegment::Anonymous && name.middle.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
        Function {
            start, end, name,
        }
    }).collect::<Vec<_>>();
    assert_eq!(&functions, &expected);
}

macro_rules! function_spans {
    (
        @ $name:literal [$start:literal - $end:literal]
    ) => {
        Function {
            start: $start,
            end: $end,
            name: Some($name.into()),
        }
    };
    (
        @ [$start:literal - $end:literal]
    ) => {
        Function {
            start: $start,
            end: $end,
            name: None,
        }
    };
    (
        $(
            $($name:literal)? [$start:literal - $end:literal]
        ),+ $(,)?
    ) => {
        [
            $(
                function_spans!(@ $($name)?[$start - $end])
            ),+
        ]
    };
}

#[test]
fn top_level_functions() {
    check_result(
        &"local function first_do() end
        function then_do() end", &function_spans! [
        "first_do"[1-1], "then_do"[2-2],
    ]);
}

#[test]
fn local_function_in_local_function() {
    check_result(
        &"local function add(y)
            local function inner()
            end
            return x + y
        end", &function_spans! [
        "add"[1-5], "inner"[2-3],
    ]);
}

#[test]
fn function_with_fields_in_function_with_fields() {
    check_result(
        &"function x.y:z()
            function a.b.c()
                local var = const;
            end
        end", &function_spans! [
        "x.y:z"[1-5], "a.b.c"[2-4],
    ]);
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
                    end", &function_spans! [
        "very.spread:out"[1-8], "very.indented"[10-16],
    ]);
}

#[test]
fn anonymous_function_in_local_variable() {
    check_result("local compact = function()
        local body = false
    end
    local
    spread
    =
    function()
        function inner()
            hello_world()
        end
    end
    
    local parenthesized = (((function() end)))", &function_spans! [
            "compact"[1-3], "spread"[7-11], "inner"[8-10], "parenthesized"[13-13],
        ],
    );
}

#[test]
fn anonymous_function_in_variable() {
    check_result("global =
    
    function()
        function inner()
            hello_world()
        end
    end", &function_spans! [
        "global"[3-7], "inner"[4-6],
    ]);
}

#[test]
fn anonymous_function_in_field() {
    check_result(r#"x.y = function()
        local field = true
    end
    
    x
        [
            "y"
        ]
        =
        function()
        end
    
    t[1] = function() end
    t[true] = function() end"#, &function_spans! [
        "x.y"[1-3], r#"x["y"]"#[10-11], "t[1]"[13-13], "t[true]"[14-14],
    ]);
}

#[test]
fn anonymous_functions_binopped() {
    check_result("_ = (function()

    end) + (function()
    
    end)
    
    local _ = (function()

    end) - (function()
    
    end)", &function_spans![
        [1-3], [3-5], [7-9], [9-11],
    ]);
}

#[test]
fn anonymous_functions_unopped() {
    check_result("local _ = -function()

    end
    
    _ = #function()

    end", &function_spans! [
        [1-3], [5-7],
    ]);
}

#[test]
fn anonymous_function_in_assignment_without_variable() {
    check_result("local x, y = 1, 2,
    function()
    end
    
    x, y = 1, 2,
    function()
    end", &function_spans![
        [2-3], [6-7]
    ]);
}

#[test]
fn function_in_table_literal() {
    check_result(r#"t = {
        function()
        end,
        get = function()
        end
    }
    
    local mt = {
        function()
        end,
        __newindex = function(self, k, v)
            rawset(self, k, v)
        end,
        __index = {
            get = function(self, k)
            end,
            ["set"] = function(self, k, v)
            end,
        },
    }
    
    ({ "value", get = function(self, k) return rawget(self, k) end }):get(1)"#, &function_spans![
        [2-3], "t.get"[4-5],
        "mt[1]"[9-10], "mt.__newindex"[11-13], "mt.__index.get"[14-15], r#"mt.__index["set"]"#[16-17],
        "?.get"[21-21],
    ]);
}

#[test]
fn function_in_function_arguments() {
    check_result("local _ = call(
        function()
            do_something(function()
            end)
        end
    )
    
    result = use_function(function()
    end)", &function_spans! [
        [2-5], [3-4], [8-9],
    ]);
}

#[test]
fn function_in_table_constructor_as_function_argument() {
    check_result(r#"local _ = call {
        function()
            do_something(function()
            end)
        end,
        identifier = function()
        end,
        ["string"] = function()
        end,
    }
    
    result = use_function{function()
    end}
    
    use_function{function()
    end}"#, &function_spans! [
        [2-5], [3-4], [6-7], [8-9], [12-13], [15-16],
    ]);
}