use std::{
    fs::File,
    io::{BufReader, BufWriter},
};

use mp4::*;

pub fn speed_up(input_video_path: String, output_video_path: String) -> Result<()> {
    let src_file = File::open(input_video_path)?;
    let size = src_file.metadata()?.len();
    let reader = BufReader::new(src_file);

    let dst_file = File::create(output_video_path)?;
    let writer = BufWriter::new(dst_file);

    let mut mp4_reader = mp4::Mp4Reader::read_header(reader, size)?;
    if mp4_reader.duration().as_secs() < 60 {
        panic!("Video should be more than 60 seconds");
    }

    let speed_up_multiplier = mp4_reader.duration().as_secs() as f32 / 60f32;

    println!("Speed Up: {speed_up_multiplier}x");

    let mut mp4_writer = mp4::Mp4Writer::write_start(
        writer,
        &Mp4Config {
            major_brand: *mp4_reader.major_brand(),
            minor_version: mp4_reader.minor_version(),
            compatible_brands: mp4_reader.compatible_brands().to_vec(),
            timescale: (mp4_reader.timescale() as f32 * speed_up_multiplier) as u32,
        },
    )?;

    // TODO interleaving
    for track in mp4_reader.tracks().values() {
        let media_conf = match track.media_type()? {
            MediaType::H264 => MediaConfig::AvcConfig(AvcConfig {
                width: track.width(),
                height: track.height(),
                seq_param_set: track.sequence_parameter_set()?.to_vec(),
                pic_param_set: track.picture_parameter_set()?.to_vec(),
            }),
            MediaType::H265 => MediaConfig::HevcConfig(HevcConfig {
                width: track.width(),
                height: track.height(),
            }),
            MediaType::VP9 => MediaConfig::Vp9Config(Vp9Config {
                width: track.width(),
                height: track.height(),
            }),
            MediaType::AAC => MediaConfig::AacConfig(AacConfig {
                bitrate: track.bitrate(),
                profile: track.audio_profile()?,
                freq_index: track.sample_freq_index()?,
                chan_conf: track.channel_config()?,
            }),
            MediaType::TTXT => MediaConfig::TtxtConfig(TtxtConfig {}),
        };

        let track_conf = TrackConfig {
            track_type: track.track_type()?,
            timescale: (track.timescale() as f32 * speed_up_multiplier) as u32,
            language: track.language().to_string(),
            media_conf,
        };

        mp4_writer.add_track(&track_conf)?;
    }

    for track_id in mp4_reader.tracks().keys().copied().collect::<Vec<u32>>() {
        let sample_count = mp4_reader.sample_count(track_id)?;
        for sample_idx in 0..sample_count {
            let sample_id = sample_idx + 1;
            let sample = mp4_reader.read_sample(track_id, sample_id)?.unwrap();
            mp4_writer.write_sample(track_id, &sample)?;
            // println!("copy {}:({})", sample_id, sample);
        }
    }

    mp4_writer.write_end()?;

    Ok(())
}
