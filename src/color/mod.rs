macro_rules! color_output {
    ($color:expr, $expression:block) => {
        use io::stdout;
        use std::io::{IsTerminal, Write};

        let mut stdout = stdout();
        if stdout.is_terminal() {
            write!(stdout, "{}", $color.prefix())?;
        }
        $expression
        if stdout.is_terminal() {
            write!(stdout, "{}", $color.suffix())?;
        }
    };
}

pub(crate) use color_output;

macro_rules! request_output {
    ($expression:block) => {
        use ansi_term::Color::Yellow;
        color_output!(Yellow, $expression)
    };
}

macro_rules! response_output {
    ($expression:block) => {
        use ansi_term::Color::Cyan;
        color_output!(Cyan, $expression)
    };
}

pub(crate) use request_output;
pub(crate) use response_output;
