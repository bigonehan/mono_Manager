use std::io::{self, Write};
use std::time::Duration;

use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAnswerKind {
    YesNo,
    Number,
    Text,
}

#[derive(Debug, Clone, Copy)]
pub struct InputQuestionOptions {
    pub auto: bool,
    pub time: u64,
}

impl Default for InputQuestionOptions {
    fn default() -> Self {
        Self {
            auto: false,
            time: 0,
        }
    }
}

#[allow(dead_code)]
pub fn input_ask_question(
    question: String,
    kind: InputAnswerKind,
    options: Option<InputQuestionOptions>,
) -> Result<String> {
    let opts = options.unwrap_or_default();
    input_validate_time(opts.time)?;
    if opts.time > 0 {
        std::thread::sleep(Duration::from_secs(opts.time));
    }

    if opts.auto {
        return Ok(auto_answer(kind));
    }

    loop {
        print!("{question} ");
        io::stdout().flush()?;

        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        let input = buffer.trim();

        match kind {
            InputAnswerKind::YesNo => {
                if let Some(normalized) = input_validate_yes_no(input) {
                    return Ok(normalized);
                }
                println!("invalid input: use y or n");
            }
            InputAnswerKind::Number => {
                if input_validate_number(input) {
                    return Ok(input.to_string());
                }
                println!("invalid input: use number");
            }
            InputAnswerKind::Text => {
                return Ok(input.to_string());
            }
        }
    }
}

fn auto_answer(kind: InputAnswerKind) -> String {
    match kind {
        InputAnswerKind::YesNo => "yes".to_string(),
        InputAnswerKind::Number => "1".to_string(),
        InputAnswerKind::Text => "나비".to_string(),
    }
}

fn input_validate_time(time: u64) -> Result<()> {
    if time == 0 || (1..=60).contains(&time) {
        return Ok(());
    }
    bail!("time must be 0 or in range 1..=60");
}

fn input_validate_yes_no(input: &str) -> Option<String> {
    let lower = input.to_ascii_lowercase();
    match lower.as_str() {
        "y" | "yes" => Some("yes".to_string()),
        "n" | "no" => Some("no".to_string()),
        _ => None,
    }
}

fn input_validate_number(input: &str) -> bool {
    input.parse::<i64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::{
        InputAnswerKind, InputQuestionOptions, auto_answer, input_validate_number,
        input_validate_time, input_validate_yes_no,
    };

    #[test]
    fn auto_answer_follows_required_values() {
        assert_eq!(auto_answer(InputAnswerKind::YesNo), "yes");
        assert_eq!(auto_answer(InputAnswerKind::Number), "1");
        assert_eq!(auto_answer(InputAnswerKind::Text), "나비");
    }

    #[test]
    fn yes_no_validation_works() {
        assert_eq!(input_validate_yes_no("y"), Some("yes".to_string()));
        assert_eq!(input_validate_yes_no("N"), Some("no".to_string()));
        assert_eq!(input_validate_yes_no("ok"), None);
    }

    #[test]
    fn number_validation_works() {
        assert!(input_validate_number("12"));
        assert!(input_validate_number("-9"));
        assert!(!input_validate_number("1.2"));
        assert!(!input_validate_number("abc"));
    }

    #[test]
    fn time_range_validation_works() {
        assert!(input_validate_time(0).is_ok());
        assert!(input_validate_time(1).is_ok());
        assert!(input_validate_time(60).is_ok());
        assert!(input_validate_time(61).is_err());
    }

    #[test]
    fn options_default_is_false_and_zero() {
        let opts = InputQuestionOptions::default();
        assert!(!opts.auto);
        assert_eq!(opts.time, 0);
    }
}
