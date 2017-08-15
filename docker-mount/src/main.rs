extern crate curl;

use std::io::{stdout, Write};
use curl::easy::Easy;

fn main(){
    let mut easy = Easy::new();
    easy.url("https://www.rust-lang.org/").unwrap();
    easy.write_function(|data| {
        Ok(stdout().write(data).unwrap())
    }).unwrap();
    easy.perform().unwrap();
}
