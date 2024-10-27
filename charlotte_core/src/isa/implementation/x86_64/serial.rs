use core::fmt::{self, Write};

use crate::arch::{Api, ArchApi, Serial};

#[allow(unused)]
pub enum ComPort {
    COM1 = 0x3F8,
    COM2 = 0x2F8,
    COM3 = 0x3E8,
    COM4 = 0x2E8,
    COM5 = 0x5F8,
    COM6 = 0x4F8,
    COM7 = 0x5E8,
    COM8 = 0x4E8,
}

pub struct SerialPort {
    io_port: u16,
}

impl SerialPort {
    pub fn try_new(com_port: ComPort) -> Option<Self> {
        let port = SerialPort {
            io_port: com_port as u16,
        };
        ArchApi::outb(port.io_port + 1, 0x00); // Disable all interrupts
        ArchApi::outb(port.io_port + 3, 0x80); // Enable DLAB (set baud rate divisor)
        ArchApi::outb(port.io_port + 0, 0x03); // Set divisor to 3 (lo byte) 38400 baud
        ArchApi::outb(port.io_port + 1, 0x00); //                  (hi byte)
        ArchApi::outb(port.io_port + 3, 0x03); // 8 bits, no parity, one stop bit
        ArchApi::outb(port.io_port + 2, 0xC7); // Enable FIFO, clear them, with 14-byte threshold
        ArchApi::outb(port.io_port + 4, 0x0B); // IRQs enabled, RTS/DSR set
        ArchApi::outb(port.io_port + 4, 0x1E); // Set in loopback mode, test the serial chip
        ArchApi::outb(port.io_port + 0, 0xAE); // Test serial chip (send byte 0xAE and check if serial returns same byte)

        if ArchApi::inb(port.io_port) != 0xAE {
            None
        } else {
            ArchApi::outb(port.io_port + 4, 0x0F);
            Some(port)
        }
    }
    fn is_transmit_empty(&self) -> i32 {
        (ArchApi::inb(self.io_port + 5) & 0x20).into()
    }
    fn received(&self) -> bool {
        (ArchApi::inb(self.io_port + 5) & 1) != 0
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c)?
        }
        Ok(())
    }
    fn write_char(&mut self, c: char) -> fmt::Result {
        while self.is_transmit_empty() == 0 {}
        if c.is_ascii() {
            if c == '\n' {
                ArchApi::outb(self.io_port, '\r' as u8);
                ArchApi::outb(self.io_port, '\n' as u8);
            } else {
                ArchApi::outb(self.io_port, c as u8);
            }
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

impl Serial for SerialPort {
    fn read_char(&mut self) -> char {
        while !self.received() {}
        ArchApi::inb(self.io_port) as char
    }
    fn put_char(&mut self, c: char) {
        self.write_char(c).unwrap()
    }
}
