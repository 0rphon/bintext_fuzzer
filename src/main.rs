mod fuzz;
mod matching;

use std::env::args;
use std::fs::create_dir_all;

///just handles the file system and does an argument match
fn main() {
    create_dir_all("corpus").unwrap();
    create_dir_all("crashes").unwrap();
    create_dir_all("tmp").unwrap();
    match args().nth(1) {
        Some(a) if a == "--match"  => matching::get_results().unwrap(),
        Some(a) if a == "--isolate"=> matching::isolate().unwrap(),
        Some(a) => println!("Unrecognized argument: {}",a),
        None    => fuzz::fuzz().unwrap()
    }
}