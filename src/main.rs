/* Parser for Motorola mapphone dtb gpio signalmap */

use io::Error;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::ErrorKind;
use std::io::Read;
use std::str;

struct Data<'a> {
    haystack: &'a mut Vec<u8>,
    needle: &'a [u8],
    curr: usize,
}

impl Iterator for Data<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
	while self.curr < self.haystack.len() - 4 {
	    match self.haystack.get(self.curr..self.curr + 4) {
		None => {
		    continue;
		}
		Some(chunk) => {
		    if chunk == self.needle {
			self.curr += 4;
			return Some(self.curr - 4);
		    }
		}
	    }
	    self.curr += 4;
	}
	None
    }
}

fn find_aligned_str<'a>(h: &'a mut Vec<u8>, n: &'a str) -> Data<'a> {
    Data { haystack: h, needle: n.as_bytes(), curr: 0 }
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
struct GpioDesc {
    gpio: u8,
    desc: String,
}

struct Gpio<'a> {
    haystack: &'a mut Vec<u8>,
    curr: usize,
}

impl Iterator for Gpio<'_> {
    type Item = GpioDesc;
    fn next(&mut self) -> Option<Self::Item> {
	/* Get next gpio, assume data ends if a 0x3e '>' tag is found */
	if self.haystack[self.curr + 12] == 0x38 {
	    return None;
	}

	let gpio = self.haystack[self.curr];
	self.curr += 4;

	/*
	 * Get the gpio desc length, at some point we can just use
	 * CStr::from_bytes_until_nul() presumably.
	 */
	let mut desc_len = 0;
	for i in 0..32 {
	    if self.haystack[self.curr + i] == 0 {
		desc_len = i;
		break;
	    }
	}

	match self.haystack.get(self.curr..self.curr + desc_len) {
	    None => {
		return None
	    }
	    Some(buf) => {
		let mut desc: String = String::from_utf8_lossy(buf).to_string();
		self.curr += ((desc.len() + 3) / 4) * 4;
		desc = desc.trim().to_string();
		let gpio_entry = GpioDesc { gpio: gpio, desc: desc };
		return Some(gpio_entry);
	    }
	}
    }
}

fn gpio<'a>(h: &'a mut Vec<u8>, i: usize) -> Gpio<'a> {
    Gpio { haystack: h, curr: i }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
	println!("usage: {} filename", args[0]);
	return Ok(())
    }

    /* Load dtb */
    let f = File::open(args[1].as_str()).unwrap();
    let mut f = BufReader::new(f);
    let mut v : Vec<u8> = Vec::new();
    f.read_to_end(&mut v).unwrap();

    /* Find the second instance of tag GPIO */
    let mut data_start: usize = 5 * 4;
    match find_aligned_str(&mut v, "GPIO").skip(1).next() {
	None => {
	    return Err(Error::new(ErrorKind::Other, "GPIO tag not found"));
	}
	Some(offset) => {
	    data_start = offset + data_start;
	}
    }

    /* Parse the gpio signalmap for sorting */
    let mut entries :Vec<GpioDesc> = Vec::new();
    let gpios = gpio(&mut v, data_start);
    for g in gpios {
	let entry = GpioDesc { gpio: g.gpio, desc: g.desc.clone() };
	entries.push(entry);
    }
    entries.sort();
    for g in entries {
	println!("{:0>3}\tgpios = <&gpio{} {} GPIO_ACTIVE_X>;\t/* gpio_{:<3} {} */",
		 g.gpio, g.gpio / 32, g.gpio % 32, g.gpio, g.desc);
    }

    Ok(())
}
