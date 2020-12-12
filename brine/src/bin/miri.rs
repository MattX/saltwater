extern crate brine;

use serde_lexpr::{from_str, to_string};
use std::io::BufRead;
use brine::mir::MirExpr;
use brine::miri::run;

fn main() {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        match from_str::<MirExpr>(&line.unwrap()) {
            Ok(p) => {
                println!("=> {}", to_string(&p).unwrap());
                println!("=> {:?}", run(&p));
            }
            Err(e) => println!("!! {:?}", e),
        }
    }
}
