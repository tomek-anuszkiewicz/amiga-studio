#[path = "common/frame.rs"]
mod receiver;

#[path = "frame/fields.rs"]
mod fields;
#[path = "frame/monitor_image.rs"]
mod monitor_image;
#[path = "frame/raster_origin.rs"]
mod raster_origin;
#[path = "frame/visible_storage.rs"]
mod visible_storage;

#[test]
fn output_coordinates_follow_visible_lines_across_the_hardware_counter_wrap() {
    use denise::{Denise, DeniseModel};

    for ccks in [227, 228] {
        for (lace, odd) in [(false, false), (true, false), (true, true)] {
            let mut denise = Denise::new(DeniseModel::Ocs8362);
            denise.bplcon0 = (if lace { 4 } else { 0 }).into();
            denise.sync.field_odd = odd;
            denise.strobe_strhor();
            let row = usize::from(lace && odd);
            for hpos in 0..ccks {
                denise.step_cck();
                let cursor = &denise.frame_cursor;
                assert_eq!(cursor.active, hpos >= 36);
                if hpos >= 36 {
                    assert_eq!(cursor.frame_x, (hpos - 36 + 1) * 4);
                    assert_eq!(cursor.frame_y, Some(row));
                }
            }
            denise.strobe_strhor();
            assert_eq!(denise.frame_cursor.frame_x, (ccks - 36) * 4);
            for hpos in 0..36 {
                denise.step_cck();
                let cursor = &denise.frame_cursor;
                assert_eq!(cursor.active, hpos < 18);
                if hpos < 18 {
                    assert_eq!(cursor.frame_x, (ccks - 36 + hpos + 1) * 4);
                    assert_eq!(cursor.frame_y, Some(row));
                }
            }
            denise.step_cck();
            assert!(denise.frame_cursor.active);
            assert_eq!(denise.frame_cursor.frame_x, 4);
            assert_eq!(denise.frame_cursor.frame_y, Some(row + 2));
        }
    }
}
