extern crate backforth;

fn main() {
    use backforth::Word;

    let mut program = vec![Word::Atom("repl".to_owned())];

    if let Some(path) = std::env::args().nth(1) {
        program.clear();
        program.push(Word::Atom("interpret".to_owned()));
        program.push(Word::from(path));
    }

    let mut shell = backforth::Shell::new();

    shell.load(program.into_iter());

    shell.run().unwrap_or_else(|err| {
        println!("{}", err);
    });
}
