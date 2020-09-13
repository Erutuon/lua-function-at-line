use lua_function_at_line::{FunctionFinder, FunctionNameLine};
use full_moon::parse;

fn main() {
    let file = std::env::args_os().skip(1).next().expect("supply file name");
    let code = std::fs::read_to_string(&file).expect("failed to read file");
    let mut finder = FunctionFinder::new();
    let functions = finder.get_functions(code.clone()).unwrap_or_else(|| {
        // Show error message ignored by FunctionFinder.
        parse(&code).unwrap();
        std::process::exit(1);
    }).to_owned();
    let max_function_name = functions.iter().map(|function| function.name.len()).max();
    for (i, line) in code.lines().enumerate() {
        println!("{: <5}{: >width$}  {}", i, finder.function_from_line(&code, i).unwrap_or("<chunk>"), line, width = max_function_name.unwrap_or(0));
    }
    for FunctionNameLine { start, end, name } in functions {
        println!("{: >width$} {:<3}..{:<3}", name, start, end, width = max_function_name.unwrap_or(0))
    }
}
