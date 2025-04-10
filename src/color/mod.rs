macro_rules! request_output {
    ($expression:block) => {
        use ansi_term::Color::Yellow;
        $crate::color::color_output!(Yellow, $expression)
    };
}
pub(crate) use request_output;

macro_rules! response_output {
    ($expression:block) => {
        use ansi_term::Color::Cyan;
        $crate::color::color_output!(Cyan, $expression)
    };
}
pub(crate) use response_output;

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
