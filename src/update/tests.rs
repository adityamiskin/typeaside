use super::*;

const TEST_KEY: &str = "untrusted comment: minisign public key 7D77563D2CA7988E\nRWSOmKcsPVZ3fd6byHe9YSYvAMv1iDlpcoqw+ZMNrvCzr0FYwR10RDlh\n";
const MANIFEST: &str = "{\"version\":\"99.0.0\",\"files\":{\"TypeAside.AppImage\":\"6ce765c53e97f87ed2607ed3604a36bdc9b40d83ce1b642f46659be869c0c5bc\"}}\n";
const MANIFEST_SIGNATURE: &str = "untrusted comment: signature from minisign secret key\nRUSOmKcsPVZ3fSVlENFckPRq9/q0Qh+ulluFwxagj4fQbl8oSZMfysfpZvnyuIGUNZ9okn/itgkdyU5Pd2chczYIn+FHFSwn2Aw=\ntrusted comment: timestamp:1792650611\tfile:manifest\thashed\n4V4aNgZKbO0pTXVVjQt4gMk0ZyXUmmrVpsPY7tJYkO8555N6MjzyJ3lvxGucK0SC+tuJX+kyrywYCjxIs+jJDg==\n";
const ARTIFACT_SIGNATURE: &str = "untrusted comment: signature from minisign secret key\nRUSOmKcsPVZ3fW569vJJHmT0gue680ZiLc+dSbJhlhUjPEmMafqXN7wSEcWcLHAxR7WjLXBi70AZOmeoLnB6mr8sNtcwCgs8Sgs=\ntrusted comment: timestamp:1792650611\tfile:artifact\thashed\nJ+plFWCDfsD9oy8SrLM1Qyi3gVWftcEduhoeXAPgELkVuNazDRS7dtzOnFJZmfqIzt7z9Xe4tvPQHlwzJGXFCA==\n";

#[test]
fn signed_manifest_binds_version_and_checksums() {
    assert!(
        verified_manifest(
            MANIFEST.as_bytes(),
            MANIFEST_SIGNATURE,
            TEST_KEY,
            "1.0.0",
            Some("TypeAside.AppImage")
        )
        .unwrap()
        .is_some()
    );
    assert!(
        verified_manifest(
            MANIFEST.as_bytes(),
            MANIFEST_SIGNATURE,
            TEST_KEY,
            "99.0.0",
            None
        )
        .unwrap()
        .is_none()
    );
    assert!(
        verified_manifest(
            MANIFEST.as_bytes(),
            MANIFEST_SIGNATURE,
            TEST_KEY,
            "100.0.0",
            None
        )
        .unwrap()
        .is_none()
    );
    for body in [
        MANIFEST.replace("99.0.0", "100.0.0"),
        MANIFEST[..MANIFEST.len() - 2].to_owned(),
    ] {
        assert!(
            verified_manifest(body.as_bytes(), MANIFEST_SIGNATURE, TEST_KEY, "1.0.0", None)
                .is_err()
        );
    }
    assert!(
        verified_manifest(
            MANIFEST.as_bytes(),
            ARTIFACT_SIGNATURE,
            TEST_KEY,
            "1.0.0",
            None
        )
        .is_err()
    );
    assert!(
        verified_manifest(
            MANIFEST.as_bytes(),
            MANIFEST_SIGNATURE,
            include_str!("../../packaging/minisign.pub"),
            "1.0.0",
            None
        )
        .is_err()
    );
}

#[test]
fn streaming_cap_rejects_extra_bytes_without_writing_them() {
    for length in [0, 7, 8, 9, 100] {
        let mut output = Vec::new();
        let result = copy_capped(&mut &vec![1; length][..], &mut output, 8, |_| {});
        assert_eq!(result.is_ok(), length <= 8);
        assert!(output.len() <= 8);
    }
    struct Interrupted;
    impl Read for Interrupted {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "truncated",
            ))
        }
    }
    assert!(copy_capped(&mut Interrupted, &mut Vec::new(), 8, |_| {}).is_err());
}

#[test]
fn staged_version_requires_exact_expected_version() {
    assert!(check_version_output(b"0.2.40\n", "0.2.40").is_ok());
    for output in [
        b"0.2.39".as_slice(),
        b"TypeAside 0.2.40",
        b"0.2.40\nextra",
        b"",
        b"\xff",
    ] {
        assert!(check_version_output(output, "0.2.40").is_err());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn appimage_swap_requires_staged_version_and_keeps_previous() {
    let directory = TemporaryDirectory::new(&std::env::temp_dir()).unwrap();
    let root = &directory.0;
    let installer = root.join("downloaded");
    let destination = root.join("installed.AppImage");
    let previous = root.join("installed.AppImage.previous");
    let release = b"#!/bin/sh\ntest \"$1\" = --version && echo 99.0.0\n";
    fs::write(&installer, release).unwrap();
    fs::write(&destination, b"current").unwrap();
    fs::write(&previous, b"older").unwrap();

    for (script, version) in [
        (release.as_slice(), "100.0.0"),
        (b"#!/bin/sh\nexit 1\n", "99.0.0"),
    ] {
        fs::write(&installer, script).unwrap();
        assert!(replace_appimage(&installer, &destination, version).is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"current");
        assert_eq!(fs::read(&previous).unwrap(), b"older");
    }

    fs::write(&installer, release).unwrap();
    replace_appimage(&installer, &destination, "99.0.0").unwrap();
    assert_eq!(fs::read(&destination).unwrap(), release);
    assert_eq!(fs::read(&previous).unwrap(), b"current");

    use std::os::unix::fs::PermissionsExt;
    fs::write(&installer, b"#!/bin/sh\nexec sleep 30\n").unwrap();
    fs::set_permissions(&installer, fs::Permissions::from_mode(0o755)).unwrap();
    let started = std::time::Instant::now();
    assert!(
        validate_appimage(
            &installer,
            &root.join("output"),
            "99.0.0",
            Duration::from_millis(100)
        )
        .is_err()
    );
    assert!(started.elapsed() < Duration::from_secs(5));
}
