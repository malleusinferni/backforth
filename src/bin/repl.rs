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

    backforth::Shell::new().run(program).unwrap_or_else(|err| {
        println!("{}", err);
    });
}
