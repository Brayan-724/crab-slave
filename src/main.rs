use std::{
    path::PathBuf,
    process::{self, Output}, fs,
};

mod ffmpeg;
// mod mp4;

fn main() {
    let mut args = std::env::args();
    let input_video_path = args.nth(1).expect("Needs input video path");
    let output_video_path = args.next().expect("Needs output vide path");

    println!("{input_video_path:?} {output_video_path:?}");

    let duration = video_duration(&input_video_path);
    let speed_multiplier = duration / 60f32;

    let fast_video = speed_up(&input_video_path, speed_multiplier);

    ffmpeg::process(fast_video.into(), output_video_path.into());

    // mp4::speed_up(input_video_path, output_video_path).unwrap();
}

fn speed_up(input_video_path: &String, speed_multiplier: f32) -> String {
    // ffmpeg -i $INPUT_FILE -filter:v "setpts=PTS/$SPEEDUP" $OUTPUT_FILE
    let output = PathBuf::from(input_video_path);
    let output = output
        .with_file_name("temp-speed-video-crab-slave.mp4")
        .display()
        .to_string();

    let _ = fs::remove_file(&output);

    let mut cmd = process::Command::new("ffmpeg")
        .args([
            "-i",
            input_video_path,
            "-filter:v",
            &format!("setpts=PTS/{speed_multiplier}"),
            &output,
            "-an"
        ])
        .stderr(process::Stdio::inherit())
        .stdin(process::Stdio::null())
        .stdout(process::Stdio::inherit())
        .spawn()
        .unwrap();

    cmd.wait().unwrap();

    output
}

fn video_duration(input_video_path: &String) -> f32 {
    let Output {
        status,
        stdout,
        stderr: _,
    } = process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-i",
            input_video_path,
        ])
        .stdin(process::Stdio::null())
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::inherit())
        .output()
        .unwrap();

    if !status.success() {
        panic!("{status:?}");
    }

    const DURATION_LEN: usize = "duration=".len();

    let stdout = String::from_utf8(stdout).unwrap();
    let stdout = stdout.split("\n").find_map(|l| {
        if !l.starts_with("duration=") {
            return None;
        }

        let duration = &l[DURATION_LEN..];
        Some(duration.parse::<f32>().unwrap())
    });

    stdout.unwrap()
}
