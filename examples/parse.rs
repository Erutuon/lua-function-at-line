use full_moon::{
    parse,
};

fn main() {
    dbg!(parse("_ = (function()

    end) + (function()
    
    end)").unwrap());
}
