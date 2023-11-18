use std::path::PathBuf;

use video_rs::{self, ffmpeg::Rescale, Decoder, Encoder, EncoderSettings, Locator, Time};

pub fn process(input_video_path: PathBuf, output_video_path: PathBuf) {
    video_rs::init().unwrap();
    let source: Locator = input_video_path.into();
    let mut decoder = Decoder::new(&source).expect("failed to create decoder");
    let input_size = decoder.size_out();

    let destination: Locator = output_video_path.into();
    let settings =
        EncoderSettings::for_h264_yuv420p(input_size.0 as usize, input_size.1 as usize, false);
    let mut encoder = Encoder::new(&destination, settings).expect("failed to create encoder");

    let mut before_frame: Vec<u8> = Vec::new();
    let mut before_logged_frame: Option<usize> = None;

    const MAX_DIFF: usize = 1360 * 720; // My screen resolution

    let duration: Time = Time::from_nth_of_a_second(25);
    let (duration_time, duration_time_base) = duration.into_parts();

    let position = Time::zero();
    let (mut position_time, position_time_base) = position.into_parts();
    let time_base = encoder.time_base();

    let frame_rate = decoder.frame_rate() as usize;

    for (frame_idx, mut frame) in decoder
        .decode_raw_iter()
        .take_while(Result::is_ok)
        .map(Result::unwrap)
        .enumerate()
    {
        let ts = frame_idx / frame_rate;
        let mut frame_diff = 0usize;

        frame.data(0).chunks(3).enumerate().for_each(|(i, pixel)| {
            let pixel_buff = pixel[0] as u16 + pixel[1] as u16 + pixel[2] as u16;
            let now = (pixel_buff / 3) as u8;

            let before = before_frame.get_mut(i);

            if let Some(before) = before {
                let diff = before.abs_diff(now);
                if diff > 50 {
                    frame_diff += diff as usize;
                }

                *before = now;
            } else {
                before_frame.push(now);
            }
        });

        let frame_diff = frame_diff / MAX_DIFF;

        let must_skip = frame_diff == 0;

        if before_logged_frame != Some(ts) {
            println!("{}: {must_skip} {frame_diff}", ts);
            before_logged_frame = Some(ts);
        }

        if must_skip {
            continue;
        }

        let position_aligned = position_time.map(|f| f.rescale(position_time_base, time_base));

        frame.set_pts(position_aligned);
        encoder.encode_raw(frame).unwrap();

        position_time = Some(
            position_time.unwrap()
                + duration_time
                    .unwrap()
                    .rescale(duration_time_base, position_time_base),
        );
    }

    encoder.finish().expect("failed to finish encoder");
}
