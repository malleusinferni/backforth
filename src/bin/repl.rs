extern crate backforth;

fn main() {
    use std::io::{Write, stdin, stdout};

    use backforth::Flattenable;

    let mut env = backforth::Env::new();
    let mut inbuf = String::new();

    let bye = backforth::parse("bye").unwrap();

    loop {
        print!("> ");
        stdout().flush().unwrap();

        stdin().read_line(&mut inbuf).unwrap();

        let program = match backforth::parse(&inbuf) {
            Ok(program) => program,
            Err(err) => {
                println!("parse error");
                continue;
            },
        };

        if program == bye { return; }

        match env.run(program) {
            Ok(result) => println!("\t-> {}", result.flatten(" ")),
            Err(err) => println!("error"),
        }

        inbuf.clear();
    }
}
