use std::{
    fs,
    path::PathBuf,
    process::{self, Output},
};

mod ffmpeg;

const TARGET_VIDEO_LENGTH: f32 = 60f32 * 2f32;
const DURATION_LEN: usize = "duration=".len();

fn main() {
    let mut args = std::env::args();
    let input_video_path = args.nth(1).expect("Needs input video path");
    let output_video_path = args.next().expect("Needs output video path");

    println!("{input_video_path:?} {output_video_path:?}");

    let duration = get_video_duration(&input_video_path);
    let speed_multiplier = duration / TARGET_VIDEO_LENGTH;

    let fast_video = speed_up_video(&input_video_path, speed_multiplier);
    println!("{speed_multiplier}x -> {fast_video}");

    ffmpeg::process(fast_video.into(), output_video_path.into());
}

fn speed_up_video(input_video_path: &String, speed_multiplier: f32) -> String {
    let output = PathBuf::from(input_video_path);
    let output = output.with_file_name("temp-speed-video-crab-slave.mp4");

    let _ = fs::remove_file(&output);

    let output = output.display().to_string();

    let mut cmd = process::Command::new("ffmpeg")
        .args([
            "-an",
            "-i",
            input_video_path,
            "-filter:v",
            &format!("setpts=PTS/{speed_multiplier}"),
            &output,
        ])
        .stdin(process::Stdio::null())
        .stderr(process::Stdio::inherit())
        .stdout(process::Stdio::inherit())
        .spawn()
        .unwrap();

    cmd.wait().unwrap();

    output
}

fn get_video_duration(input_video_path: &String) -> f32 {
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
