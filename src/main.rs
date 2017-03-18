extern crate backforth;

static REPL_SOURCE: &'static str = r#"
loop {
    try {
        len capture

        if < 0 rot {
            strcat swap " ~> " flatten " " capture
        } {
            "> "
        }

        eval parse prompt
    } {
        echo
    }
}
"#;

fn main() {
    let mut program = backforth::parse(REPL_SOURCE).unwrap();

    if let Some(path) = std::env::args().nth(1) {
        use backforth::Word;

        program.clear();
        for word in &["eval", "parse", "load"] {
            program.push(Word::Atom(word.to_string()));
        }
        program.push(Word::from(path));
    }

    let mut shell = backforth::Shell::new();

    shell.load(program.into_iter());

    shell.run().unwrap_or_else(|err| {
        println!("{}", err);
    });
}
