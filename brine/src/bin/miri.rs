extern crate brine;

use brine::mir::{lexpr_to_mir, mir_to_lexpr, MirExpr};
use brine::miri::run;
use serde_lexpr::{from_str, to_string};
use std::io::BufRead;

fn main() {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        match lexpr::from_str(&line.unwrap())
            .map_err(|e| e.to_string())
            .and_then(|e| lexpr_to_mir(e))
        {
            Ok(p) => {
                println!("=> {}", mir_to_lexpr(&p).to_string());
                println!("=> {:?}", run(&p));
            }
            Err(e) => println!("!! {:?}", e),
        }
    }
}
