extern crate backforth;

fn main() {
    use std::io::{Write, stdin, stdout};

    use backforth::Flattenable;

    let mut env = backforth::Env::new();
    let mut inbuf = String::new();

    let bye = backforth::parse("bye").unwrap();

    loop {
        inbuf.clear();

        if env.view().is_empty() {
            print!("> ");
        } else {
            print!("{} ~> ", env.view().flatten(" "));
        }

        stdout().flush().unwrap();

        stdin().read_line(&mut inbuf).unwrap();

        let program = match backforth::parse(&inbuf) {
            Ok(program) => program,
            Err(err) => {
                println!("{}", err);
                continue;
            },
        };

        if program == bye { return; }

        if let Err(err) = env.run(program) {
            println!("{}", err);
        }
    }
}
