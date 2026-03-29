mod handlers;

use std::{env, process};

fn main() {
    env_logger::init();

    let verb = parse_verb();

    let Some((_, handler)) = handlers::HANDLERS
        .iter()
        .find(|(name, _)| return *name == verb)
    else {
        eprintln!("Unknown verb: {verb}");
        eprintln!("Available verbs:");
        for (name, _) in handlers::HANDLERS {
            eprintln!("{}", name);
        }
        process::exit(2);
    };

    match handler() {
        Ok(output) => {
            println!("{output}");
            process::exit(0);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

fn parse_verb() -> String {
    // Skip the first item, that's the program name itself.
    let mut args = env::args().skip(1);

    let Some(verb) = args.next() else {
        eprintln!("Usage: sync_versions <verb>");
        process::exit(2);
    };

    if args.next().is_some() {
        eprintln!("Too many arguments provided. Only one verb is expected.");
        process::exit(2);
    }

    return verb;
}
