use core::fmt::Debug;

macro_rules! register_bits {
    ($getter_name: ident, $setter_name:ident, $offset: literal) => {
        pub fn $getter_name(&self) -> bool {
            const MASK: u8 = (1 << $offset);

            (self.value & MASK) != 0
        }
        pub fn $setter_name(&mut self, value: bool) {
            const MASK: u8 = (1 << $offset);

            if value {
                self.value |= MASK;
            } else {
                self.value &= (!MASK);
            }
        }
    };
}

pub trait Max7456Register: Clone + Copy + Debug {
    fn new(value: u8) -> Self;
    fn address() -> u8;
    fn value(&self) -> u8;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VM0 {
    value: u8,
}

impl VM0 {
    register_bits!(is_pal, set_pal, 6);
    register_bits!(_osd_enabled, set_osd_enabled, 3);
    register_bits!(_software_reset, _set_software_reset, 1);
}

impl Max7456Register for VM0 {
    fn new(value: u8) -> Self {
        Self { value }
    }

    fn address() -> u8 {
        0x00
    }

    fn value(&self) -> u8 {
        self.value
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DMM {
    value: u8,
}

impl Max7456Register for DMM {
    fn new(value: u8) -> Self {
        Self { value }
    }

    fn address() -> u8 {
        0x04
    }

    fn value(&self) -> u8 {
        self.value
    }
}

impl DMM {
    register_bits!(_get_8bit_mode, set_8bit_mode, 6);
    register_bits!(memory_clearing_in_progress, set_clear_memory, 2);
    register_bits!(_get_auto_increment, set_auto_increment, 0);
}

#[derive(Clone, Copy, Debug)]
pub struct DMAH {
    value: u8,
}

impl Max7456Register for DMAH {
    fn new(value: u8) -> Self {
        Self { value }
    }

    fn address() -> u8 {
        0x05
    }

    fn value(&self) -> u8 {
        self.value
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DMAL {
    value: u8,
}

impl Max7456Register for DMAL {
    fn new(value: u8) -> Self {
        Self { value }
    }

    fn address() -> u8 {
        0x06
    }

    fn value(&self) -> u8 {
        self.value
    }
}

pub struct DMA {}

impl DMA {
    fn build_display_memory_address(address: u16) -> (DMAL, DMAH) {
        let low = (address & 0xff) as u8;
        let high = (((address >> 8) & 0xff) as u8) & 0x01;

        (DMAL::new(low), DMAH::new(high))
    }

    pub fn build_address_from_row_column(row: u8, col: u8) -> (DMAL, DMAH) {
        const COLUMN_COUNT: u16 = 30;
        let addr = (row as u16) * COLUMN_COUNT + (col as u16);

        Self::build_display_memory_address(addr)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CMM {
    value: u8,
}

impl Max7456Register for CMM {
    fn new(value: u8) -> Self {
        Self { value: value }
    }

    fn address() -> u8 {
        0x08
    }

    fn value(&self) -> u8 {
        self.value
    }
}

impl CMM {
    pub fn writing_mode() -> Self {
        Self::new(0xa0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CMAH {
    value: u8,
}

impl Max7456Register for CMAH {
    fn new(value: u8) -> Self {
        Self { value }
    }

    fn address() -> u8 {
        0x09
    }

    fn value(&self) -> u8 {
        self.value
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CMAL {
    value: u8,
}

impl Max7456Register for CMAL {
    fn new(value: u8) -> Self {
        Self { value }
    }

    fn address() -> u8 {
        0x0a
    }

    fn value(&self) -> u8 {
        self.value
    }
}

pub struct CMA {}

impl CMA {
    pub fn character_address(character: u8) -> CMAH {
        CMAH::new(character)
    }

    pub fn byte_in_character_address(byte: u8) -> CMAL {
        CMAL::new(byte & 0x3f)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CMDI {
    value: u8,
}

impl Max7456Register for CMDI {
    fn new(value: u8) -> Self {
        Self { value }
    }

    fn address() -> u8 {
        0x0b
    }

    fn value(&self) -> u8 {
        self.value
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OSDBL {
    value: u8,
}

impl Max7456Register for OSDBL {
    fn new(value: u8) -> Self {
        Self { value }
    }

    fn address() -> u8 {
        0x6c
    }

    fn value(&self) -> u8 {
        self.value
    }
}

impl OSDBL {
    register_bits!(
        _black_level_control_disabled,
        disable_black_level_control,
        4
    );
}

#[derive(Clone, Copy, Debug)]
pub struct STAT {
    value: u8,
}

impl Max7456Register for STAT {
    fn new(value: u8) -> Self {
        STAT { value }
    }

    fn address() -> u8 {
        0xa0
    }

    fn value(&self) -> u8 {
        self.value
    }
}

impl STAT {
    register_bits!(pal_detected, _set_pal_detected, 0);
    register_bits!(ntsc_detected, _set_ntsc_detected, 1);
    register_bits!(character_memory_busy, _set_character_memory_busy, 5);
    register_bits!(vsync_not_in_progress, _set_vsync, 4);
}

#[derive(Clone, Copy, Debug)]
pub struct OSDM {
    value: u8,
}

impl Max7456Register for OSDM {
    fn new(value: u8) -> Self {
        OSDM { value }
    }

    fn address() -> u8 {
        0x0c
    }

    fn value(&self) -> u8 {
        self.value
    }
}
