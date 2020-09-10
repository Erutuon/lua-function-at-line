use full_moon::parse;

fn main() {
    dbg!(parse(r#"local _ = -function()

    end
    
    _ = #function()

    end"#).unwrap());
}
