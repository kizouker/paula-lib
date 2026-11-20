use std::path::PathBuf;

pub const USAGE: &str =
    "usage: paula-gui <file.mod>\n       paula-gui --score <song.paula.toml> --base <kit.mod> [--strict]\n       paula-gui [--menu [dir]] [--base <kit.mod>] [--strict]";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartMode {
    Play { path: PathBuf },
    Score { score: PathBuf, base: PathBuf, strict: bool },
    Menu { dirs: Vec<PathBuf>, base: Option<PathBuf>, strict: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliError {
    pub message: String,
    pub code: u8,
}

impl CliError {
    fn usage(message: &str) -> Self {
        Self { message: format!("{message}\n{USAGE}"), code: 2 }
    }
}

pub fn parse_args(args: &[String]) -> Result<StartMode, CliError> {
    parse(args.iter().map(|a| a.as_str()))
}

pub fn parse<I, S>(args: I) -> Result<StartMode, CliError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();
    let mut file = None;
    let mut score = None;
    let mut base = None;
    let mut strict = false;
    let mut menu = false;
    let mut menu_dir = None;
    let mut it = args.iter().peekable();

    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--menu" => {
                menu = true;
                if let Some(dir) = it.peek().and_then(|next| (!next.starts_with('-')).then(|| next.to_string())) {
                    it.next();
                    menu_dir = Some(PathBuf::from(dir));
                }
            }
            "--score" => score = Some(PathBuf::from(it.next().ok_or_else(|| CliError::usage("--score needs a file name"))?.clone())),
            "--base" => base = Some(PathBuf::from(it.next().ok_or_else(|| CliError::usage("--base needs a file name"))?.clone())),
            "--strict" => strict = true,
            value if value.starts_with('-') => return Err(CliError::usage(&format!("unknown option {value}"))),
            value => file = Some(PathBuf::from(value)),
        }
    }

    if menu || (file.is_none() && score.is_none()) {
        if file.is_some() || score.is_some() {
            return Err(CliError::usage("--menu does not combine with a file or --score"));
        }
        let dirs = menu_dir.map(|d| vec![d]).unwrap_or_else(|| vec![PathBuf::from("examples"), PathBuf::from(".")]);
        return Ok(StartMode::Menu { dirs, base, strict });
    }

    match (file, score, base) {
        (Some(path), None, None) => Ok(StartMode::Play { path }),
        (None, Some(score), Some(base)) => Ok(StartMode::Score { score, base, strict }),
        (None, Some(_), None) => Err(CliError::usage("--score needs --base <kit.mod>")),
        _ => Err(CliError::usage("give either <file.mod> or --score <song.paula.toml> --base <kit.mod>")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_examples_and_dot() {
        assert_eq!(
            parse(Vec::<String>::new()).unwrap(),
            StartMode::Menu { dirs: vec![PathBuf::from("examples"), PathBuf::from(".")], base: None, strict: false }
        );
    }

    #[test]
    fn menu_accepts_directory_base_and_strict() {
        assert_eq!(
            parse(["--menu", "scores", "--base", "kit.mod", "--strict"]).unwrap(),
            StartMode::Menu { dirs: vec![PathBuf::from("scores")], base: Some(PathBuf::from("kit.mod")), strict: true }
        );
    }

    #[test]
    fn parses_file_and_score_modes() {
        assert_eq!(parse(["song.mod"]).unwrap(), StartMode::Play { path: PathBuf::from("song.mod") });
        assert_eq!(
            parse(["--score", "song.paula.toml", "--base", "kit.mod", "--strict"]).unwrap(),
            StartMode::Score { score: PathBuf::from("song.paula.toml"), base: PathBuf::from("kit.mod"), strict: true }
        );
    }

    #[test]
    fn invalid_mode_has_usage() {
        let error = parse(["--score", "song.paula.toml"]).unwrap_err();
        assert_eq!(error.code, 2);
        assert!(error.message.contains("usage:"));
    }
}
