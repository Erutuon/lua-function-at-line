use full_moon::parse;
use lua_function_at_line::gather_function_line_spans;

fn main() {
    // dbg!(parse(r#"local _ = -function()

    // end
    
    // _ = #function()

    // end"#).unwrap());
    let ast = parse(r#"local _ = call {
        function()
            do_something(function()
                local _ = inside "function call"
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
    end}"#).unwrap();
    let mut functions = vec![];
    gather_function_line_spans(ast.nodes(), &mut functions).unwrap();
}
