//! Registry tests, to ensure that transfer syntaxes are properly
//! registered when linked together in a separate program.

use dicom_encoding::transfer_syntax::TransferSyntaxIndex;
use dicom_transfer_syntax_registry::TransferSyntaxRegistry;

fn assert_fully_supported<T>(registry: T, mut uid: &'static str, name: &'static str)
where
    T: TransferSyntaxIndex,
{
    let ts = registry.get(uid);
    assert!(ts.is_some(), "Registry did not provide TS {}", uid);
    let ts = ts.unwrap();
    if uid.ends_with("\0") {
        uid = &uid[0..uid.len() - 1];
    }
    assert_eq!(ts.uid(), uid);
    assert_eq!(ts.name(), name);
    assert!(ts.fully_supported());
}

#[test]
fn contains_base_ts() {
    let registry = TransferSyntaxRegistry;

    // contains implicit VR little endian and is fully supported
    assert_fully_supported(registry, "1.2.840.10008.1.2", "Implicit VR Little Endian");

    // should work the same for trailing null characters
    assert_fully_supported(registry, "1.2.840.10008.1.2\0", "Implicit VR Little Endian");

    // contains explicit VR little endian and is fully supported
    assert_fully_supported(registry, "1.2.840.10008.1.2.1", "Explicit VR Little Endian");

    // contains explicit VR big endian and is fully supported
    assert_fully_supported(registry, "1.2.840.10008.1.2.2", "Explicit VR Big Endian");
}
