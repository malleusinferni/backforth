extern crate backforth;

static SOURCE: &'static str = r#"
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
    let program = backforth::parse(SOURCE).unwrap();

    let mut shell = backforth::Shell::new();

    shell.load(program.into_iter());

    shell.run().unwrap_or_else(|err| {
        println!("{}", err);
    });
}
