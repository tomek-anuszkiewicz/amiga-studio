#![allow(clippy::unwrap_used, clippy::panic)]

use gui::host_video::{FrameInfo, VideoExchange, VideoExchangeError, VideoRead};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn info(publication: u64) -> FrameInfo {
    FrameInfo {
        width: 4,
        height: 2,
        row_stride: 6,
        field_number: publication / 2,
        publication,
        cck: publication * 100,
        generation: 1,
        interlaced: true,
        field_odd: publication % 2 != 0,
        complete: publication % 3 != 0,
    }
}

#[test]
fn full_and_reading_storage_cannot_be_overwritten() {
    let exchange = VideoExchange::new(vec![0; 12]);
    assert_eq!(
        exchange.try_consume(1, |_, _| ()).unwrap(),
        VideoRead::Pending
    );
    assert!(exchange
        .try_publish(|pixels| {
            assert_eq!(
                exchange.try_consume(1, |_, _| ()).unwrap(),
                VideoRead::Pending
            );
            pixels.fill(0xff12_3456);
            Some(info(1))
        })
        .unwrap());
    let prepared = AtomicBool::new(false);
    let prepare_again = |_: &mut Vec<u32>| {
        prepared.store(true, Ordering::Relaxed);
        Some(info(2))
    };
    assert!(!exchange.try_publish(prepare_again).unwrap());
    assert!(!prepared.load(Ordering::Relaxed));
    let VideoRead::Frame(converted) = exchange
        .try_consume(1, |metadata, pixels| {
            assert_eq!(metadata, info(1));
            assert_eq!(pixels, &[0xff12_3456; 12]);
            assert!(!exchange.try_publish(prepare_again).unwrap());
            assert!(!prepared.load(Ordering::Relaxed));
            pixels.to_vec()
        })
        .unwrap()
    else {
        panic!("Published frame was not available");
    };
    assert!(exchange.try_publish(prepare_again).unwrap());
    assert!(prepared.load(Ordering::Relaxed));
    assert_eq!(converted, vec![0xff12_3456; 12]);
}

#[test]
fn invalid_bounds_do_not_publish_and_stale_images_release_the_buffer() {
    let exchange = VideoExchange::new(vec![0; 12]);
    let mut invalid = info(1);
    for (width, height, stride) in [
        (0, 2, 6),
        (4, 0, 6),
        (7, 2, 6),
        (4, 3, 6),
        (1, usize::MAX, 2),
    ] {
        invalid.width = width;
        invalid.height = height;
        invalid.row_stride = stride;
        assert_eq!(
            exchange.try_publish(|_| Some(invalid)),
            Err(VideoExchangeError::InvalidFrame)
        );
        assert_eq!(
            exchange.try_consume(1, |_, _| ()).unwrap(),
            VideoRead::Pending
        );
    }
    assert_eq!(
        exchange.try_publish(|_| None),
        Err(VideoExchangeError::InvalidFrame)
    );
    assert!(exchange.try_publish(|_| Some(info(1))).unwrap());
    assert_eq!(
        exchange
            .try_consume(2, |_, _| -> () { panic!("Stale frame reached the reader") })
            .unwrap(),
        VideoRead::Discarded
    );
    let mut current = info(7);
    current.generation = 2;
    assert!(exchange.try_publish(|_| Some(current)).unwrap());
    assert_eq!(
        exchange.try_consume(2, |metadata, _| metadata).unwrap(),
        VideoRead::Frame(current)
    );
}

#[test]
fn concurrent_exchange_keeps_metadata_pixels_and_publication_order_coherent() {
    let exchange = Arc::new(VideoExchange::new(vec![0; 12]));
    let producer_exchange = exchange.clone();
    let deadline = Instant::now() + Duration::from_secs(10);
    let producer = std::thread::spawn(move || {
        for publication in 1..=1000 {
            loop {
                assert!(Instant::now() < deadline, "Producer did not make progress");
                if producer_exchange
                    .try_publish(|pixels| {
                        pixels.fill(publication as u32);
                        Some(info(publication))
                    })
                    .unwrap()
                {
                    break;
                }
                std::thread::yield_now();
            }
        }
    });
    for publication in 1..=1000 {
        loop {
            assert!(Instant::now() < deadline, "Consumer did not make progress");
            if matches!(
                exchange
                    .try_consume(1, |metadata, pixels| {
                        assert_eq!(metadata, info(publication));
                        assert_eq!(pixels, &[publication as u32; 12]);
                        std::thread::yield_now();
                        assert_eq!(pixels, &[publication as u32; 12]);
                    })
                    .unwrap(),
                VideoRead::Frame(())
            ) {
                break;
            }
            std::thread::yield_now();
        }
    }
    producer.join().unwrap();
}

#[test]
fn real_frame_builder_handoff_leaves_published_pixels_and_drawing_independent() {
    for (standard, height) in [
        (config::VideoStandard::Pal, 568),
        (config::VideoStandard::Ntsc, 480),
    ] {
        let mut session = gui::DebuggerSession::new();
        let frame = &mut session.machine.denise.frame_builder;
        frame.set_video_standard(standard);
        let exchange = VideoExchange::new(vec![0; frame.storage_len()]);
        frame.set_field_mode(true, false);
        frame.write_pixel_pair(0, 0, [0xffff_0000; 2]);
        frame.write_pixel_pair(718, height - 2, [0xff11_2233; 2]);
        frame.finish_field(height / 2);
        let metadata = FrameInfo {
            width: frame.width as usize,
            height: frame.height as usize,
            row_stride: frame.row_stride(),
            field_number: frame.frame_number,
            ..info(1)
        };
        assert!(exchange
            .try_publish(|pixels| frame.exchange_buffer(pixels).then_some(metadata))
            .unwrap());
        frame.set_field_mode(true, true);
        frame.write_pixel_pair(0, 1, [0xff00_00ff; 2]);
        assert_eq!(
            exchange
                .try_consume(1, |received, pixels| {
                    assert_eq!(received, metadata);
                    assert_eq!(
                        [received.width, received.height, received.row_stride],
                        [720, height, 720]
                    );
                    assert_eq!(pixels[720 * height - 721], 0xff11_2233);
                    assert_eq!(pixels.len(), 720 * height);
                    assert_eq!(pixels[0], 0xffff_0000);
                    assert_eq!(pixels[720], 0xff00_0000);
                })
                .unwrap(),
            VideoRead::Frame(())
        );
        assert_eq!(frame.get_pixel(0, 0), 0xffff_0000);
        assert_eq!(frame.get_pixel(0, 1), 0xff00_00ff);
    }
}
