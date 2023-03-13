use crate::io::pmio::Port;

pub(super) static CONTROL_PORT: Port = Port::new(0x64);

/// PS/2 Controller status register mask
#[repr(u8)]
#[rustfmt::skip]
#[allow(unused)]
pub enum Status {
	OBF = (1<<0), /* output buffer full */
	IBF = (1<<1), /* input buffer full  */
	SF  = (1<<2), /* system flag        */
	CD  = (1<<3), /* command / data     */
	IS  = (1<<4), /* inhibit switch     */
	TTO = (1<<5), /* transmit time-out  */
	RTO = (1<<6), /* receive time-out   */
	PE  = (1<<7), /* parity error       */
}

pub fn get_raw_status() -> u8 {
	CONTROL_PORT.read_byte()
}

pub fn test_status(status: u8, mask: Status) -> bool {
	(status & (mask as u8)) != 0
}

pub fn test_status_now(mask: Status) -> bool {
	test_status(get_raw_status(), mask)
}
