use std::{env, fs, process};

use rt_common::SourceFile;
use rt_driver::{compile_source, CompileOptions, EmitStage, OutputFormat};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_args(&args) {
        Ok(Command::Help) => {
            print_help();
        }
        Ok(Command::Compile {
            source,
            options,
            output_path,
        }) => {
            let text = match fs::read_to_string(&source) {
                Ok(text) => text,
                Err(err) => {
                    eprintln!("rtc: failed to read `{}`: {}", source, err);
                    process::exit(1);
                }
            };
            let output = compile_source(SourceFile::new(source, text), options);
            if let Some(output_path) = output_path {
                if let Err(err) = fs::write(&output_path, output) {
                    eprintln!("rtc: failed to write `{}`: {}", output_path, err);
                    process::exit(1);
                }
            } else {
                print!("{}", output);
            }
        }
        Err(message) => {
            eprintln!("rtc: {}", message);
            eprintln!("try `rtc --help`");
            process::exit(2);
        }
    }
}

enum Command {
    Help,
    Compile {
        source: String,
        options: CompileOptions,
        output_path: Option<String>,
    },
}

fn parse_args(args: &[String]) -> Result<Command, String> {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(Command::Help);
    }

    let mut options = CompileOptions::default();
    let mut source = None;
    let mut output_path = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--emit" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--emit requires a stage".to_string())?;
                options.emit = parse_emit(value)?;
            }
            "--format" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--format requires text or json".to_string())?;
                options.format = match value.as_str() {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    other => return Err(format!("unknown format `{}`", other)),
                };
            }
            "--check" => {
                options.emit = EmitStage::Semantic;
            }
            "-S" => {
                options.emit = EmitStage::LlvmIr;
            }
            "-o" => {
                index += 1;
                let path = args
                    .get(index)
                    .ok_or_else(|| "-o requires an output path".to_string())?;
                output_path = Some(path.to_string());
            }
            arg if arg.starts_with('-') => {
                return Err(format!("unknown option `{}`", arg));
            }
            path => {
                if source.replace(path.to_string()).is_some() {
                    return Err("multiple input files are not supported yet".to_string());
                }
            }
        }
        index += 1;
    }

    let source = source.ok_or_else(|| "missing input file".to_string())?;
    Ok(Command::Compile {
        source,
        options,
        output_path,
    })
}

fn parse_emit(value: &str) -> Result<EmitStage, String> {
    match value {
        "tokens" => Ok(EmitStage::Tokens),
        "ast" => Ok(EmitStage::Ast),
        "semantic" => Ok(EmitStage::Semantic),
        "llvm-ir" | "ir" => Ok(EmitStage::LlvmIr),
        "teaching-ir" => Ok(EmitStage::TeachingIr),
        "all" => Ok(EmitStage::All),
        other => Err(format!("unknown emit stage `{}`", other)),
    }
}

fn print_help() {
    println!(
        "Rust Teaching Compiler\n\nUSAGE:\n    rtc [OPTIONS] <input.rs>\n\nOPTIONS:\n    --emit <stage>       tokens | ast | semantic | llvm-ir | teaching-ir | ir | all\n    --format <format>    text | json\n    --check              run semantic checks\n    -S                   emit LLVM IR text\n    -o <file>            write output to a file\n    -h, --help           print help\n"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_help() {
        let args = vec!["--help".to_string()];
        assert!(matches!(parse_args(&args), Ok(Command::Help)));
    }

    #[test]
    fn parses_emit_tokens_json() {
        let args = vec![
            "--emit".to_string(),
            "tokens".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "examples/basic.rs".to_string(),
        ];
        let command = parse_args(&args).expect("valid args");
        match command {
            Command::Compile { options, .. } => {
                assert_eq!(options.emit, EmitStage::Tokens);
                assert_eq!(options.format, OutputFormat::Json);
            }
            Command::Help => panic!("expected compile command"),
        }
    }
}
