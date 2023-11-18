use std::path::PathBuf;

use video_rs::{self, ffmpeg::Rescale, Decoder, Encoder, EncoderSettings, Locator, Time};

pub fn process(input_video_path: PathBuf, output_video_path: PathBuf) {
    video_rs::init().unwrap();
    let source: Locator = input_video_path.into();
    let mut decoder = Decoder::new(&source).expect("failed to create decoder");

    let destination: Locator = output_video_path.into();
    let settings = decoder.size_out();
    let settings =
        EncoderSettings::for_h264_yuv420p(settings.0 as usize, settings.1 as usize, false);
    let mut encoder = Encoder::new(&destination, settings).expect("failed to create encoder");

    let mut before_frame: Vec<u8> = Vec::new();
    let mut before_logged_frame: Option<usize> = None;

    const MAX_DIFF: usize = 1366 * 720; // My screen resolution

    let duration: Time = Time::from_nth_of_a_second(25);
    let mut position = Time::zero();
    let time_base = encoder.time_base();

    for mut frame in decoder
        .decode_raw_iter()
        .take_while(Result::is_ok)
        .map(Result::unwrap)
    {
        let ts = frame.timestamp().unwrap() as usize / 10000;
        let mut pixel_buff = 0u16;
        let mut frame_diff = 0usize;

        for (i, p) in frame.data(0).iter().enumerate() {
            pixel_buff += *p as u16;

            if i % 3 == 2 {
                let now = (pixel_buff / 3) as u8;
                pixel_buff = 0;

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
            }
        }

        let frame_diff = frame_diff as usize / MAX_DIFF;

        let must_skip = frame_diff == 0;

        if before_logged_frame != Some(ts) {
            println!("{}: {must_skip} {frame_diff}", ts);
            before_logged_frame = Some(ts);
        }

        if must_skip {
            continue;
        }

        let position_aligned = position.clone().into_parts();
        let position_aligned = Time::new(
            position_aligned
                .0
                .map(|f| f.rescale(position_aligned.1, time_base)),
            position_aligned.1,
        );

        frame.set_pts(position_aligned.into_value());

        encoder.encode_raw(frame).unwrap();

        // encoder
        //     .encode(&frame, &position)
        //     .expect("failed to encode frame");

        position = position.aligned_with(&duration).add();
    }

    encoder.finish().expect("failed to finish encoder");
}
