use std::io::Cursor;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};

#[test]
fn full_screen_session_captures_mouse_and_releases_it_during_cleanup() {
    let mut output = Cursor::new(Vec::new());

    super::configure_mouse_capture(&mut output, true).expect("mouse capture is enabled");
    super::configure_mouse_capture(&mut output, false).expect("mouse capture is disabled");

    let expected = {
        let mut expected = Cursor::new(Vec::new());
        execute!(expected, EnableMouseCapture, DisableMouseCapture)
            .expect("crossterm writes mouse capture commands");
        expected.into_inner()
    };

    assert_eq!(output.into_inner(), expected);
}
