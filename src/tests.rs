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
                name: Some("first_do".into()),
            },
            FunctionSpan {
                start: 2,
                end: 2,
                name: Some("then_do".into()),
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
                name: Some("add".into()),
            },
            FunctionSpan {
                start: 2,
                end: 3,
                name: Some("inner".into()),
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
                name: Some("x.y:z".into()),
            },
            FunctionSpan {
                start: 2,
                end: 4,
                name: Some("a.b.c".into()),
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
                name: Some("very.spread:out".into()),
            },
            FunctionSpan {
                start: 10,
                end: 16,
                name: Some("very.indented".into()),
            },
        ],
    );
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
    
    local parenthesized = (((function() end)))", &[
        FunctionSpan {
            name: Some("compact".into()),
            start: 1,
            end: 3,
        },
        FunctionSpan {
            start: 7,
            end: 11,
            name: Some("spread".into()),
        },
        FunctionSpan {
            start: 8,
            end: 10,
            name: Some("inner".into()),
        },
        FunctionSpan {
            start: 13,
            end: 13,
            name: Some("parenthesized".into()),
        },
    ])
}

#[test]
fn anonymous_function_in_variable() {
    check_result("global =
    
    function()
        function inner()
            hello_world()
        end
    end", &[
        FunctionSpan {
            name: Some("global".into()),
            start: 3,
            end: 7,
        },
        FunctionSpan {
            name: Some("inner".into()),
            start: 4,
            end: 6,
        }
    ])
}

#[test]
fn anonymous_function_in_field() {
    check_result("x.y = function()
        local field = true
    end", &[
        FunctionSpan {
            name: Some("x.y".into()),
            start: 1,
            end: 3,
        }
    ])
}

#[test]
fn anonymous_functions_binopped() {
    check_result("_ = (function()

    end) + (function()
    
    end)
    
    local _ = (function()

    end) - (function()
    
    end)", &[
        FunctionSpan {
            start: 1,
            end: 3,
            name: None,
        },
        FunctionSpan {
            start: 3,
            end: 5,
            name: None,
        },
        FunctionSpan {
            start: 7,
            end: 9,
            name: None,
        },
        FunctionSpan {
            start: 9,
            end: 11,
            name: None,
        },
    ]);
}

#[test]
fn anonymous_functions_unopped() {
    check_result("_ = -function()

    end
    
    _ = #function()

    end", &[
        FunctionSpan {
            start: 1,
            end: 3,
            name: None,
        },
        FunctionSpan {
            start: 5,
            end: 7,
            name: None,
        },
    ]);
}
