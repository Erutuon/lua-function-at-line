use full_moon::parse;

fn main() {
    dbg!(parse(r#"x
    [
        "y"
    ]
    =
    function()
    end"#).unwrap());
}
