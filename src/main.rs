extern crate base64;
extern crate clap;
extern crate flate2;
extern crate tempfile;

use std::fs::File;
use std::io;
use std::io::{BufRead, ErrorKind, Read, Write};
use std::path::Path;
use std::process::{exit, Command};

use clap::{App, Arg, ArgMatches};
use flate2::read::ZlibDecoder;
use tempfile::NamedTempFile;

enum ReplayType {
    Unknown,
    /// The raw replay format created by the Forged Alliance binary
    ForgedAlliance,
    /// The legacy replay format from FAForever
    /// (A json followed by a linebreak and then including the Qt-zipped base64-ed replay stream)
    FafLegacy,
}

enum ReplayLocation<'a> {
    AtPath(&'a Path),
    AtTempFile(NamedTempFile),
}

fn build_cli() -> ArgMatches<'static> {
    App::new("faf-replay-cli")
        .about("A replay launcher for FAForever")
        .version("0.1")
        .author("Brutus5000 <Brutus5000@gmx.net>")
        .arg(
            Arg::with_name("executable")
                .long("executable")
                .short("e")
                .value_name("PATH TO ForgedAlliance.exe")
                .help("Path to the ForgedAlliance.exe")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("local-file")
                .long("local-file")
                .short("f")
                .value_name("FILE")
                .help("Path to the replay file you want to watch")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("wrapper")
                .long("wrapper")
                .short("w")
                .value_name("WRAPPER")
                .help("Path to the wrapper script (usually for Linux)")
                .takes_value(true)
                .required(false),
        )
        .get_matches()
}

fn get_executable_path<'a>(args: &'a ArgMatches) -> &'a Path {
    let executable_str = args.value_of("executable").unwrap();
    let executable_path = Path::new(executable_str);

    if !executable_path.exists() {
        println!("No executable found at {}", executable_str);
        exit(1)
    }

    executable_path
}

fn get_replay_path<'a>(args: &'a ArgMatches) -> &'a Path {
    let replay_str = args.value_of("local-file").unwrap();
    let replay_path = Path::new(replay_str);

    if !replay_path.exists() {
        println!("No replay file found at {}", replay_str);
        exit(1)
    }

    replay_path
}

fn get_wrapper_path<'a>(args: &'a ArgMatches) -> Option<&'a Path> {
    args.value_of("wrapper").map(|wrapper_str| {
        let wrapper_path = Path::new(wrapper_str);

        if !wrapper_path.exists() {
            println!("No wrapper file found at {}", wrapper_str);
            exit(1)
        }

        wrapper_path
    })
}

fn main() {
    let matches = build_cli();

    let executable = get_executable_path(&matches);
    let replay_path = get_replay_path(&matches);
    let wrapper = get_wrapper_path(&matches);

    let replay_preparation_result = prepare_replay_file(replay_path).expect("Replay file issues!");

    let raw_replay_path = match &replay_preparation_result {
        ReplayLocation::AtPath(path) => path,
        ReplayLocation::AtTempFile(f) => f.path(),
    }
        .to_str()
        .unwrap();

    launch_game(executable, &raw_replay_path, 12345, wrapper);
}

fn get_replay_type(file_name: &str) -> ReplayType {
    match file_name {
        _ if file_name.ends_with(".scfareplay") => ReplayType::ForgedAlliance,
        _ if file_name.ends_with(".fafreplay") => ReplayType::FafLegacy,
        _ => ReplayType::Unknown,
    }
}

fn prepare_replay_file(replay_path: &Path) -> io::Result<ReplayLocation> {
    let file_name = replay_path.to_str().unwrap();

    match get_replay_type(file_name) {
        ReplayType::Unknown => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unknown replay format!",
        )),
        ReplayType::ForgedAlliance => Ok(ReplayLocation::AtPath(replay_path)),
        ReplayType::FafLegacy => {
            extract_faf_legacy_replay(file_name).map(ReplayLocation::AtTempFile)
        }
    }
}

fn extract_faf_legacy_replay(file_name: &str) -> io::Result<NamedTempFile> {
    let file = File::open(file_name)?;

    let mut lines = io::BufReader::new(file).lines();

    let _json_metadata = lines.next().unwrap_or_else(|| {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Replay corrupt - replay metadata json is missing",
        ))
    })?;

    let base64_replay_stream = lines.next().unwrap_or_else(|| {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Replay corrupt - binary replay stream is missing",
        ))
    })?;

    let tempfile = convert_legacy_replay_stream_to_raw(&base64_replay_stream)?;

    Ok(tempfile)
}

fn convert_legacy_replay_stream_to_raw(base64_stream: &str) -> io::Result<NamedTempFile> {
    let zipped_qt_data = base64::decode_config(base64_stream, base64::STANDARD).map_err(|_| {
        io::Error::new(
            ErrorKind::InvalidData,
            "Replay corrupt - couldn't decode base64",
        )
    })?;

    let (_, zipped_data_slice) = zipped_qt_data.split_at(4);
    let zipped_data = Vec::from(zipped_data_slice);

    let mut temp_replay_file = tempfile::NamedTempFile::new()?;

    let mut decoder = ZlibDecoder::new(zipped_data.as_slice());
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    temp_replay_file.as_file_mut().write_all(&output)?;

    Ok(temp_replay_file)
}

fn launch_game(executable: &Path, file_name: &str, replay_id: u32, wrapper: Option<&Path>) {
    let executable_str = executable.to_str().unwrap();
    let executable_dir_str = executable.parent().unwrap().to_str().unwrap();

    let launch_arg = wrapper
        .map(|w| w.to_str().unwrap())
        .unwrap_or_else(|| executable_str);

    let mut launch_command = Command::new(launch_arg);

    if wrapper.is_some() {
        launch_command.arg(executable_str);
    }

    launch_command
        .args(&[
            "/init",
            "init.lua",
            "/nobugreport",
            "/replay",
            file_name,
            "/replayid",
            &replay_id.to_string(),
        ])
        .current_dir(executable_dir_str);

    // game_directory.map(|dir| launch_command.current_dir(Path::new(dir)));

    let result = launch_command.output().expect("Game failed to launch");

    io::stdout().write_all(&result.stdout).unwrap();
    io::stderr().write_all(&result.stderr).unwrap();

    println!("We launched the game. Check for errors!");
}
