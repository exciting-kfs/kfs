pub fn little_u32_from_slice(slice: &[u8]) -> u32 {
	let mut result = 0;

	for (i, s) in slice.iter().enumerate() {
		result |= (*s as u32) << (8 * i);
	}

	result
}
