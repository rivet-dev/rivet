use universaldb::{
	tuple::{Element, Versionstamp, pack_with_versionstamp, unpack},
	versionstamp::*,
};

#[test]
fn test_generate_versionstamp() {
	let vs1 = generate_versionstamp(100);
	let vs2 = generate_versionstamp(200);

	assert!(vs1.is_complete());
	assert!(vs2.is_complete());
	assert_eq!(vs1.user_version(), 100);
	assert_eq!(vs2.user_version(), 200);

	assert_ne!(vs1.as_bytes(), vs2.as_bytes());
}

#[test]
fn test_substitute_versionstamp_success() {
	let incomplete = Versionstamp::from([0xff; 12]);
	let tuple = vec![
		Element::String("mykey".into()),
		Element::Versionstamp(incomplete),
		Element::Int(42),
	];

	let mut packed = pack_with_versionstamp(&tuple);
	let versionstamp = generate_versionstamp(100);

	assert!(substitute_versionstamp(&mut packed, versionstamp).is_ok());

	let unpacked: Vec<Element> = unpack(&packed).unwrap();
	assert_eq!(unpacked.len(), 3);

	match &unpacked[1] {
		Element::Versionstamp(v) => {
			assert!(v.is_complete());
			assert_eq!(v.user_version(), 100);
		}
		_ => panic!("Expected versionstamp"),
	}
}

#[test]
fn test_substitute_versionstamp_no_offset() {
	let mut packed = vec![1, 2, 3];
	let versionstamp = generate_versionstamp(100);

	let result = substitute_versionstamp(&mut packed, versionstamp);
	assert!(result.is_err());
	assert!(result.unwrap_err().contains("too short"));
}

#[test]
fn test_substitute_versionstamp_invalid_offset() {
	let mut packed = vec![1, 2, 3, 4, 5];
	packed.extend_from_slice(&100u32.to_le_bytes());

	let versionstamp = generate_versionstamp(100);

	let result = substitute_versionstamp(&mut packed, versionstamp);
	assert!(result.is_err());
	assert!(result.unwrap_err().contains("Invalid versionstamp offset"));
}

#[test]
fn test_substitute_versionstamp_no_marker() {
	let mut packed = vec![1, 2, 3, 4, 5, 6, 7, 8];
	packed.extend_from_slice(&2u32.to_le_bytes());

	let versionstamp = generate_versionstamp(100);

	let result = substitute_versionstamp(&mut packed, versionstamp);
	assert!(result.is_err());
	assert!(result.unwrap_err().contains("No versionstamp marker"));
}

#[test]
fn test_substitute_versionstamp_already_complete() {
	// Create an incomplete versionstamp first
	let incomplete = Versionstamp::from([0xff; 12]);
	let tuple = vec![Element::Versionstamp(incomplete)];

	let mut packed = pack_with_versionstamp(&tuple);

	// First substitution - this should succeed
	let versionstamp1 = generate_versionstamp(50);
	assert!(substitute_versionstamp(&mut packed, versionstamp1).is_ok());

	// Now try to substitute again on the already complete versionstamp
	// We need to manually add the offset back
	packed.extend_from_slice(&1u32.to_le_bytes());

	let versionstamp2 = generate_versionstamp(100);

	let result = substitute_versionstamp(&mut packed, versionstamp2);
	// Should succeed but not modify the already complete versionstamp
	assert!(result.is_ok());
}

#[test]
fn test_pack_and_substitute_versionstamp() {
	let incomplete = Versionstamp::from([0xff; 12]);
	let tuple = vec![
		Element::String("mykey".into()),
		Element::Versionstamp(incomplete),
		Element::Int(42),
	];

	let packed = pack_and_substitute_versionstamp(&tuple, 100).unwrap();

	let unpacked: Vec<Element> = unpack(&packed).unwrap();
	match &unpacked[1] {
		Element::Versionstamp(v) => {
			assert!(v.is_complete());
			assert_eq!(v.user_version(), 100);
		}
		_ => panic!("Expected versionstamp"),
	}
}

#[test]
fn test_versionstamp_incomplete_preserves_user_version() {
	// Test that Versionstamp::incomplete(user_version) properly preserves the user version
	let user_version: u16 = 12345;
	let incomplete = Versionstamp::incomplete(user_version);

	// The versionstamp should be incomplete
	assert!(
		!incomplete.is_complete(),
		"Versionstamp should be incomplete"
	);

	// The user version should be preserved
	assert_eq!(
		incomplete.user_version(),
		user_version,
		"User version should be preserved"
	);

	// Verify the bytes structure: first 10 bytes should be 0xff, last 2 bytes should be user_version
	let bytes = incomplete.as_bytes();
	assert_eq!(bytes.len(), 12);

	// First 10 bytes should all be 0xff
	for i in 0..10 {
		assert_eq!(bytes[i], 0xff, "Byte {} should be 0xff", i);
	}

	// Last 2 bytes should contain the user version in big-endian
	let stored_user_version = u16::from_be_bytes([bytes[10], bytes[11]]);
	assert_eq!(
		stored_user_version, user_version,
		"User version bytes should match"
	);

	// Test with substitute_versionstamp - the user version from the incomplete versionstamp
	// should be ignored and replaced with the one from the generated versionstamp
	let tuple = vec![
		Element::String("test".into()),
		Element::Versionstamp(incomplete),
	];

	let mut packed = pack_with_versionstamp(&tuple);
	let new_user_version: u16 = 54321;
	let versionstamp = generate_versionstamp(new_user_version);

	assert!(substitute_versionstamp(&mut packed, versionstamp).is_ok());

	let unpacked: Vec<Element> = unpack(&packed).unwrap();
	match &unpacked[1] {
		Element::Versionstamp(v) => {
			assert!(v.is_complete());
			// The user version should be from the generated versionstamp, not the original incomplete one
			assert_eq!(
				v.user_version(),
				new_user_version,
				"Substituted versionstamp should have the new user version"
			);
		}
		_ => panic!("Expected versionstamp"),
	}
}
