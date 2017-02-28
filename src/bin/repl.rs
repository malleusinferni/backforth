extern crate backforth;

fn main() {
    let source = "loop { try { eval parse prompt \"> \" } { echo } }";
    backforth::Env::new().run(backforth::parse(source).unwrap()).unwrap();
}
