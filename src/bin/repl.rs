extern crate backforth;

fn main() {
    let source = "loop { try { eval parse prompt \"> \" } { echo } }";
    let program = backforth::parse(source).unwrap();

    if let Err(err) = backforth::Env::new().run(program) {
        println!("{}", err);
    }
}
