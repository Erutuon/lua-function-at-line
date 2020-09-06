use lua_function_at_line::gather_function_line_spans;
use full_moon::{
    parse,
    Error,
};

fn main() {
    let block = parse(
        "local x = 10
    
    local function add(y)
        local function inner()
        end
        return x + y
    end
    
    function x.y:z() end
    
    function very
    .
    spread
    :
    out()
    end",
    )
    .unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });
    let mut functions = Vec::new();
    gather_function_line_spans(&block.nodes(), &mut functions)
        .map_err(Error::AstError)
        .unwrap_or_else(|e| {
            eprintln!("{}", e);
            std::process::exit(1);
        });
    dbg!(functions);
}
