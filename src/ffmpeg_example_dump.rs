
// mod info;
//
// use mp4::{
//     AacConfig, AvcConfig, HevcConfig, MediaConfig, MediaType, Mp4Box, Mp4Config, Result,
//     TrackConfig, TtxtConfig, Vp9Config,
// };
// use std::fs::File;
// use std::io::{BufReader, BufWriter};
// use std::ops::Deref;
// use std::path::Path;
// use std::time::Duration;
//
// use info::info;
//
// fn main() -> Result<()> {
//     let mut args = std::env::args();
//     let input_video_path = args.nth(1).expect("Needs input video path");
//     let output_video_path = args.next().expect("Needs output vide path");
//     println!("{input_video_path:?} {output_video_path:?}");
//
//     info(&input_video_path)
// }
//
extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use std::env;
use std::fs::File;
use std::io::{prelude::*, Cursor};

fn main() -> Result<(), ffmpeg::Error> {
    ffmpeg::init().unwrap();

    if let Ok(mut ictx) = input(&env::args().nth(1).expect("Cannot open file.")) {
        let input = ictx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        let video_stream_index = input.index();

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
        let mut decoder = context_decoder.decoder().video()?;

        let mut scaler = Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            Flags::BILINEAR,
        )?;

        let mut frame_index = 0;

        let mut receive_and_process_decoded_frames =
            |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
                let mut decoded = Video::empty();
                while decoder.receive_frame(&mut decoded).is_ok() {
                    let mut rgb_frame = Video::empty();
                    scaler.run(&decoded, &mut rgb_frame)?;
                    save_file(&rgb_frame, frame_index).unwrap();
                    frame_index += 1;
                }
                Ok(())
            };

        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream_index {
                decoder.send_packet(&packet)?;
                receive_and_process_decoded_frames(&mut decoder)?;
            }
        }
        decoder.send_eof()?;
        receive_and_process_decoded_frames(&mut decoder)?;
    }

    Ok(())
}

fn save_file(frame: &Video, index: usize) -> std::result::Result<(), std::io::Error> {
    // let mut file = File::create(format!("frame{}.ppm", index))?;
    // let mut file = BufferWriter::new(Vec::new());
    // file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
    // file.write_all(frame.data(0))?;
    // println!("P6\n{} {}\n255\n", frame.width(), frame.height());
    // println!("{:?}", frame.data(0));
    // println!("{:?}", file.into_inner());
    println!("Frame {index}");
    Ok(())
}
