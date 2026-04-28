//! M6 — save/load round-trip and filesystem helper tests.
//!
//! Exercises [`save_new_from_game`], [`save_to_bytes`], [`save_from_bytes`],
//! [`save_to_game`], the version/name accessors, and the directory listing
//! functions. The listing tests use `tempfile` so they don't pollute the
//! shared `std::env::temp_dir()`.

use std::ffi::CString;
use std::ptr;

use adventerm_ffi::{
    game_action_move, game_free, game_new_seeded, game_player_pos, save_delete,
    save_format_version, save_free, save_from_bytes, save_list_count, save_list_free,
    save_list_modified_unix, save_list_name, save_list_open, save_list_path, save_name,
    save_new_from_game, save_slot_path, save_slugify, save_to_bytes, save_to_game,
    save_version, CDirection, CMoveOutcome, FfiError, GameHandle, SaveHandle, SaveListing,
};

/// Construct a `SaveHandle` from a fresh seeded game named `name`.
fn make_save(seed: u64, name: &str) -> (*mut GameHandle, *mut SaveHandle) {
    let game = game_new_seeded(seed);
    let cname = CString::new(name).unwrap();
    let mut save: *mut SaveHandle = ptr::null_mut();
    let rc = save_new_from_game(game, cname.as_ptr(), &mut save);
    assert_eq!(rc, FfiError::Ok as i32);
    assert!(!save.is_null());
    (game, save)
}

/// Serialize a save handle to a `Vec<u8>` via the two-call discovery.
fn save_bytes(save: *const SaveHandle) -> Vec<u8> {
    let mut needed = 0usize;
    let rc = save_to_bytes(save, ptr::null_mut(), 0, &mut needed);
    assert_eq!(rc, FfiError::BufferTooSmall as i32);
    let mut buf = vec![0u8; needed];
    let mut written = 0usize;
    let rc = save_to_bytes(save, buf.as_mut_ptr(), buf.len(), &mut written);
    assert_eq!(rc, FfiError::Ok as i32);
    assert_eq!(written, needed);
    buf
}

#[test]
fn bytes_round_trip_preserves_state() {
    let game = game_new_seeded(42);
    // Drive a few actions to dirty the state.
    let mut outcome = CMoveOutcome::default();
    game_action_move(game, CDirection::Right as u8, &mut outcome);
    game_action_move(game, CDirection::Down as u8, &mut outcome);

    let cname = CString::new("test").unwrap();
    let mut save: *mut SaveHandle = ptr::null_mut();
    assert_eq!(
        save_new_from_game(game, cname.as_ptr(), &mut save),
        FfiError::Ok as i32
    );

    let bytes = save_bytes(save);

    let mut restored: *mut SaveHandle = ptr::null_mut();
    assert_eq!(
        save_from_bytes(bytes.as_ptr(), bytes.len(), &mut restored),
        FfiError::Ok as i32
    );

    let mut restored_game: *mut GameHandle = ptr::null_mut();
    assert_eq!(
        save_to_game(restored, &mut restored_game),
        FfiError::Ok as i32
    );

    let mut x1 = 0usize;
    let mut y1 = 0usize;
    let mut x2 = 0usize;
    let mut y2 = 0usize;
    assert_eq!(game_player_pos(game, &mut x1, &mut y1), FfiError::Ok as i32);
    assert_eq!(
        game_player_pos(restored_game, &mut x2, &mut y2),
        FfiError::Ok as i32
    );
    assert_eq!((x1, y1), (x2, y2));

    save_free(save);
    save_free(restored);
    game_free(game);
    game_free(restored_game);
}

#[test]
fn save_to_game_keeps_save_handle_valid() {
    let (game, save) = make_save(7, "keep");
    let mut restored: *mut GameHandle = ptr::null_mut();
    assert_eq!(save_to_game(save, &mut restored), FfiError::Ok as i32);
    // Second restoration must succeed too — clone semantics.
    let mut restored2: *mut GameHandle = ptr::null_mut();
    assert_eq!(save_to_game(save, &mut restored2), FfiError::Ok as i32);
    save_free(save);
    game_free(game);
    game_free(restored);
    game_free(restored2);
}

#[test]
fn save_name_and_version_round_trip() {
    let (game, save) = make_save(3, "Alpha Run");

    let mut needed = 0usize;
    assert_eq!(
        save_name(save, ptr::null_mut(), 0, &mut needed),
        FfiError::BufferTooSmall as i32
    );
    assert_eq!(needed, "Alpha Run".len() + 1);
    let mut buf = vec![0u8; needed];
    assert_eq!(
        save_name(save, buf.as_mut_ptr(), buf.len(), &mut needed),
        FfiError::Ok as i32
    );
    assert_eq!(&buf[..buf.len() - 1], b"Alpha Run");
    assert_eq!(buf[buf.len() - 1], 0);

    let mut version = 0u32;
    assert_eq!(save_version(save, &mut version), FfiError::Ok as i32);
    assert_eq!(version, save_format_version());

    save_free(save);
    game_free(game);
}

#[test]
fn save_from_bytes_unsupported_version() {
    let bad = br#"{"version": 999, "name": "x", "state": null}"#;
    let mut out: *mut SaveHandle = ptr::null_mut();
    let rc = save_from_bytes(bad.as_ptr(), bad.len(), &mut out);
    // Either SaveFormat (deserialization fails first because state schema)
    // or UnsupportedSaveVersion (if serde accepts and version check fires).
    assert!(
        rc == FfiError::SaveFormat as i32
            || rc == FfiError::UnsupportedSaveVersion as i32
    );
    assert!(out.is_null());
}

#[test]
fn save_from_bytes_invalid_json() {
    let bad = b"not json at all";
    let mut out: *mut SaveHandle = ptr::null_mut();
    let rc = save_from_bytes(bad.as_ptr(), bad.len(), &mut out);
    assert_eq!(rc, FfiError::SaveFormat as i32);
    assert!(out.is_null());
}

#[test]
fn save_slugify_matches_lib() {
    let name = CString::new("My Cool Save!").unwrap();
    let mut needed = 0usize;
    assert_eq!(
        save_slugify(name.as_ptr(), ptr::null_mut(), 0, &mut needed),
        FfiError::BufferTooSmall as i32
    );
    let mut buf = vec![0u8; needed];
    assert_eq!(
        save_slugify(name.as_ptr(), buf.as_mut_ptr(), buf.len(), &mut needed),
        FfiError::Ok as i32
    );
    assert_eq!(&buf[..buf.len() - 1], b"my-cool-save");
    assert_eq!(buf[buf.len() - 1], 0);
}

#[test]
fn save_slot_path_matches_lib() {
    let dir = CString::new("/tmp/saves").unwrap();
    let name = CString::new("Hero").unwrap();
    let mut needed = 0usize;
    assert_eq!(
        save_slot_path(
            dir.as_ptr(),
            name.as_ptr(),
            ptr::null_mut(),
            0,
            &mut needed,
        ),
        FfiError::BufferTooSmall as i32
    );
    let mut buf = vec![0u8; needed];
    assert_eq!(
        save_slot_path(
            dir.as_ptr(),
            name.as_ptr(),
            buf.as_mut_ptr(),
            buf.len(),
            &mut needed,
        ),
        FfiError::Ok as i32
    );
    let path = std::str::from_utf8(&buf[..buf.len() - 1]).unwrap();
    assert!(path.ends_with("hero.json"));
    assert!(path.contains("/tmp/saves"));
}

/// Write `save`'s bytes to `path` via `std::fs`.
fn write_save_to_disk(save: *const SaveHandle, path: &std::path::Path) {
    let bytes = save_bytes(save);
    std::fs::write(path, &bytes).unwrap();
}

#[test]
fn list_saves_in_tempdir() {
    let dir = tempfile::tempdir().unwrap();

    let (g1, s1) = make_save(1, "Alpha");
    let (g2, s2) = make_save(2, "Beta");

    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let alpha_c = CString::new("Alpha").unwrap();
    let beta_c = CString::new("Beta").unwrap();

    let mut needed = 0usize;
    save_slot_path(
        dir_c.as_ptr(),
        alpha_c.as_ptr(),
        ptr::null_mut(),
        0,
        &mut needed,
    );
    let mut buf = vec![0u8; needed];
    save_slot_path(
        dir_c.as_ptr(),
        alpha_c.as_ptr(),
        buf.as_mut_ptr(),
        buf.len(),
        &mut needed,
    );
    let alpha_path = std::str::from_utf8(&buf[..buf.len() - 1])
        .unwrap()
        .to_string();
    write_save_to_disk(s1, std::path::Path::new(&alpha_path));

    let mut needed = 0usize;
    save_slot_path(
        dir_c.as_ptr(),
        beta_c.as_ptr(),
        ptr::null_mut(),
        0,
        &mut needed,
    );
    let mut buf = vec![0u8; needed];
    save_slot_path(
        dir_c.as_ptr(),
        beta_c.as_ptr(),
        buf.as_mut_ptr(),
        buf.len(),
        &mut needed,
    );
    let beta_path = std::str::from_utf8(&buf[..buf.len() - 1])
        .unwrap()
        .to_string();
    write_save_to_disk(s2, std::path::Path::new(&beta_path));

    let mut listing: *mut SaveListing = ptr::null_mut();
    assert_eq!(
        save_list_open(dir_c.as_ptr(), &mut listing),
        FfiError::Ok as i32
    );
    assert!(!listing.is_null());

    let mut count = 0usize;
    assert_eq!(save_list_count(listing, &mut count), FfiError::Ok as i32);
    assert_eq!(count, 2);

    let mut names = Vec::new();
    for i in 0..count {
        let mut needed = 0usize;
        save_list_name(listing, i, ptr::null_mut(), 0, &mut needed);
        let mut buf = vec![0u8; needed];
        save_list_name(listing, i, buf.as_mut_ptr(), buf.len(), &mut needed);
        names.push(
            std::str::from_utf8(&buf[..buf.len() - 1])
                .unwrap()
                .to_string(),
        );

        let mut secs = 0i64;
        assert_eq!(
            save_list_modified_unix(listing, i, &mut secs),
            FfiError::Ok as i32
        );
        assert!(secs > 0);
    }
    names.sort();
    assert_eq!(names, vec!["Alpha".to_string(), "Beta".to_string()]);

    // Out-of-range index returns OutOfRange.
    let mut secs = 0i64;
    assert_eq!(
        save_list_modified_unix(listing, count, &mut secs),
        FfiError::OutOfRange as i32
    );

    // Delete one save through the FFI and re-open the listing.
    let del_rc = save_delete(CString::new(alpha_path.as_str()).unwrap().as_ptr());
    assert_eq!(del_rc, FfiError::Ok as i32);

    save_list_free(listing);
    let mut listing: *mut SaveListing = ptr::null_mut();
    assert_eq!(
        save_list_open(dir_c.as_ptr(), &mut listing),
        FfiError::Ok as i32
    );
    let mut count = 0usize;
    save_list_count(listing, &mut count);
    assert_eq!(count, 1);

    save_list_free(listing);
    save_free(s1);
    save_free(s2);
    game_free(g1);
    game_free(g2);
}

#[test]
fn list_saves_empty_for_missing_dir() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does-not-exist");
    let dir_c = CString::new(missing.to_str().unwrap()).unwrap();

    let mut listing: *mut SaveListing = ptr::null_mut();
    assert_eq!(
        save_list_open(dir_c.as_ptr(), &mut listing),
        FfiError::Ok as i32
    );
    let mut count = 0usize;
    save_list_count(listing, &mut count);
    assert_eq!(count, 0);
    save_list_free(listing);
}

#[test]
fn save_list_path_returns_full_path() {
    let dir = tempfile::tempdir().unwrap();
    let (g, s) = make_save(11, "Solo");

    let dir_c = CString::new(dir.path().to_str().unwrap()).unwrap();
    let name_c = CString::new("Solo").unwrap();
    let mut needed = 0usize;
    save_slot_path(
        dir_c.as_ptr(),
        name_c.as_ptr(),
        ptr::null_mut(),
        0,
        &mut needed,
    );
    let mut buf = vec![0u8; needed];
    save_slot_path(
        dir_c.as_ptr(),
        name_c.as_ptr(),
        buf.as_mut_ptr(),
        buf.len(),
        &mut needed,
    );
    let solo_path = std::str::from_utf8(&buf[..buf.len() - 1])
        .unwrap()
        .to_string();
    write_save_to_disk(s, std::path::Path::new(&solo_path));

    let mut listing: *mut SaveListing = ptr::null_mut();
    save_list_open(dir_c.as_ptr(), &mut listing);
    let mut count = 0usize;
    save_list_count(listing, &mut count);
    assert_eq!(count, 1);

    let mut needed = 0usize;
    save_list_path(listing, 0, ptr::null_mut(), 0, &mut needed);
    let mut buf = vec![0u8; needed];
    save_list_path(listing, 0, buf.as_mut_ptr(), buf.len(), &mut needed);
    let listed = std::str::from_utf8(&buf[..buf.len() - 1]).unwrap();
    assert_eq!(listed, solo_path);

    save_list_free(listing);
    save_free(s);
    game_free(g);
}

#[test]
fn null_inputs_return_null_argument() {
    let mut out: *mut SaveHandle = ptr::null_mut();
    assert_eq!(
        save_new_from_game(ptr::null(), ptr::null(), &mut out),
        FfiError::NullArgument as i32
    );
    assert!(out.is_null());

    assert_eq!(
        save_to_bytes(ptr::null(), ptr::null_mut(), 0, ptr::null_mut()),
        FfiError::NullArgument as i32
    );

    let mut listing: *mut SaveListing = ptr::null_mut();
    assert_eq!(
        save_list_open(ptr::null(), &mut listing),
        FfiError::NullArgument as i32
    );
    assert!(listing.is_null());
}

#[test]
fn save_format_version_matches_lib() {
    assert_eq!(save_format_version(), adventerm_lib::SAVE_VERSION);
}
