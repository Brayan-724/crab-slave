
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

    let speed_up_multiplier = mp4_reader.duration().as_secs() as i32 / 60i32;

    println!("Speed Up: {speed_up_multiplier}x");

    let mut mp4_writer = mp4::Mp4Writer::write_start(
        writer,
        &Mp4Config {
            major_brand: *mp4_reader.major_brand(),
            minor_version: mp4_reader.minor_version(),
            compatible_brands: mp4_reader.compatible_brands().to_vec(),
            timescale: (mp4_reader.timescale() as i32 * speed_up_multiplier) as u32,
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
            timescale: (track.timescale() as i32 * speed_up_multiplier) as u32,
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
            if sample.bytes.len() < 1028 {
                println!("Skipped! {}", sample.start_time);
                continue;
            }
            println!("{sample}");
            mp4_writer.write_sample(track_id, &sample)?;
            // println!("copy {}:({})", sample_id, sample);
        }
    }

    mp4_writer.write_end()?;

    Ok(())
}

fn dump<P: AsRef<Path>>(filename: &P) -> Result<()> {
    let f = File::open(filename)?;
    let boxes = get_boxes(f)?;

    // print out boxes
    for b in boxes.iter() {
        println!("[{}] size={} {}", b.name, b.size, b.summary);
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Box {
    name: String,
    size: u64,
    summary: String,
    indent: u32,
}

fn get_boxes(file: File) -> Result<Vec<Box>> {
    let size = file.metadata()?.len();
    let reader = BufReader::new(file);
    let mp4 = mp4::Mp4Reader::read_header(reader, size)?;

    // collect known boxes
    let mut boxes = vec![
        build_box(&mp4.ftyp),
        build_box(&mp4.moov),
        build_box(&mp4.moov.mvhd),
    ];

    if let Some(ref mvex) = &mp4.moov.mvex {
        boxes.push(build_box(mvex));
        if let Some(mehd) = &mvex.mehd {
            boxes.push(build_box(mehd));
        }
        boxes.push(build_box(&mvex.trex));
    }

    // trak.
    for track in mp4.tracks().values() {
        boxes.push(build_box(&track.trak));
        boxes.push(build_box(&track.trak.tkhd));
        if let Some(ref edts) = track.trak.edts {
            boxes.push(build_box(edts));
            if let Some(ref elst) = edts.elst {
                boxes.push(build_box(elst));
            }
        }

        // trak.mdia
        let mdia = &track.trak.mdia;
        boxes.push(build_box(mdia));
        boxes.push(build_box(&mdia.mdhd));
        boxes.push(build_box(&mdia.hdlr));
        boxes.push(build_box(&track.trak.mdia.minf));

        // trak.mdia.minf
        let minf = &track.trak.mdia.minf;
        if let Some(ref vmhd) = &minf.vmhd {
            boxes.push(build_box(vmhd));
        }
        if let Some(ref smhd) = &minf.smhd {
            boxes.push(build_box(smhd));
        }

        // trak.mdia.minf.stbl
        let stbl = &track.trak.mdia.minf.stbl;
        boxes.push(build_box(stbl));
        boxes.push(build_box(&stbl.stsd));
        if let Some(ref avc1) = &stbl.stsd.avc1 {
            boxes.push(build_box(avc1));
        }
        if let Some(ref hev1) = &stbl.stsd.hev1 {
            boxes.push(build_box(hev1));
        }
        if let Some(ref mp4a) = &stbl.stsd.mp4a {
            boxes.push(build_box(mp4a));
        }
        boxes.push(build_box(&stbl.stts));
        if let Some(ref ctts) = &stbl.ctts {
            boxes.push(build_box(ctts));
        }
        if let Some(ref stss) = &stbl.stss {
            boxes.push(build_box(stss));
        }
        boxes.push(build_box(&stbl.stsc));
        boxes.push(build_box(&stbl.stsz));
        if let Some(ref stco) = &stbl.stco {
            boxes.push(build_box(stco));
        }
        if let Some(ref co64) = &stbl.co64 {
            boxes.push(build_box(co64));
        }
    }

    // If fragmented, add moof boxes.
    for moof in mp4.moofs.iter() {
        boxes.push(build_box(moof));
        boxes.push(build_box(&moof.mfhd));
        for traf in moof.trafs.iter() {
            boxes.push(build_box(traf));
            boxes.push(build_box(&traf.tfhd));
            if let Some(ref trun) = &traf.trun {
                boxes.push(build_box(trun));
            }
        }
    }

    Ok(boxes)
}

fn build_box<M: Mp4Box + std::fmt::Debug>(m: &M) -> Box {
    Box {
        name: m.box_type().to_string(),
        size: m.box_size(),
        summary: m.summary().unwrap(),
        indent: 0,
    }
}
