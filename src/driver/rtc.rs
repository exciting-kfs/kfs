use crate::io::pmio::Port;

use time::{error::ComponentRange, Date, Month, OffsetDateTime, PrimitiveDateTime, Time};

const RTC_INDEX: Port = Port::new(0x70);
const RTC_DATA: Port = Port::new(0x71);

mod index {
	pub const SECOND: u8 = 0x00;
	pub const MINUTE: u8 = 0x02;
	pub const HOUR: u8 = 0x04;
	pub const DAY_OF_MONTH: u8 = 0x07;
	pub const MONTH: u8 = 0x08;
	pub const YEAR: u8 = 0x09;
	pub const STATUS_B: u8 = 0x0b;
}

struct RtcDateTime([u8; 6]);

impl RtcDateTime {
	pub fn new() -> Self {
		let format = rtc_read_reg(index::STATUS_B);

		let is_24_hour = format & (1 << 1) != 0;
		let is_binary = format & (1 << 2) != 0;

		let mut rtc_time = [
			rtc_read_reg(index::YEAR),
			rtc_read_reg(index::MONTH),
			rtc_read_reg(index::DAY_OF_MONTH),
			rtc_read_reg(index::HOUR),
			rtc_read_reg(index::MINUTE),
			rtc_read_reg(index::SECOND),
		];

		if !is_binary {
			for bcd in rtc_time.iter_mut() {
				*bcd = rtc_bcd_to_bin(*bcd);
			}
		}

		if !is_24_hour {
			let mut hour = rtc_time[3];
			let is_pm = (hour & 0x80) != 0;
			hour &= !0x80;

			if hour == 12 {
				hour = 0;
			}

			if is_pm {
				hour += 12;
			}

			rtc_time[3] = hour;
		}

		RtcDateTime(rtc_time)
	}

	fn try_to_datetime(self) -> Result<OffsetDateTime, ComponentRange> {
		let month = Month::try_from(self.0[1])?;

		let date = Date::from_calendar_date(self.0[0] as i32 + 2000, month, self.0[2])?;
		let time = Time::from_hms(self.0[3], self.0[4], self.0[5])?;

		Ok(PrimitiveDateTime::new(date, time).assume_utc())
	}

	pub fn to_datetime(self) -> OffsetDateTime {
		Self::try_to_datetime(self).unwrap_or(OffsetDateTime::UNIX_EPOCH)
	}
}

fn rtc_read_reg(idx: u8) -> u8 {
	RTC_INDEX.write_byte(idx);

	RTC_DATA.read_byte()
}

fn rtc_bcd_to_bin(bcd: u8) -> u8 {
	let msb = bcd & 0x80;

	let msb_masked = bcd & !0x80;

	(((msb_masked / 16) * 10) + (msb_masked % 16)) | msb
}

pub fn get_timestamp_utc() -> u64 {
	RtcDateTime::new().to_datetime().unix_timestamp() as u64
}
