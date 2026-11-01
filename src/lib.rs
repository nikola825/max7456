#![no_std]
use core::fmt::Debug;

use embedded_hal::spi::SpiBus as SpiBusBlocking;
use embedded_hal_async::spi::SpiBus as SpiBusAsync;

use crate::registers::{CMA, CMDI, CMM, DMA, DMM, Max7456Register, OSDBL, OSDM, STAT, VM0};

mod registers;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Max7456Error<
    SpiErrorType: embedded_hal_async::spi::Error,
    CsErrorType: embedded_hal::digital::Error,
> {
    CsToggleError(CsErrorType),
    SpiError(SpiErrorType),
    DetectTimeout(u8),
    MemoryClearTimeout(u8),
    CharacterMemoryWriteTimeout(u8),
}

impl<SpiErrorType: embedded_hal_async::spi::Error, CsErrorType: embedded_hal::digital::Error>
    From<SpiErrorType> for Max7456Error<SpiErrorType, CsErrorType>
{
    fn from(value: SpiErrorType) -> Self {
        Self::SpiError(value)
    }
}

impl<SpiErrorType: embedded_hal_async::spi::Error, CsErrorType: embedded_hal::digital::Error>
    Max7456Error<SpiErrorType, CsErrorType>
{
    fn from_cs_error(cs_error: CsErrorType) -> Self {
        Self::CsToggleError(cs_error)
    }
}

struct SPITransaction<
    'a,
    Bus: SpiBusAsync + SpiBusBlocking,
    CsPin: embedded_hal::digital::OutputPin,
    Delay: embedded_hal_async::delay::DelayNs,
> {
    spi: &'a mut Bus,
    cs: &'a mut CsPin,
    delay: &'a mut Delay,
}

impl<
    'a,
    Bus: SpiBusAsync + SpiBusBlocking,
    CsPin: embedded_hal::digital::OutputPin,
    Delay: embedded_hal_async::delay::DelayNs,
> SPITransaction<'a, Bus, CsPin, Delay>
{
    async fn new(
        max7456: &'a mut Max7456<Bus, CsPin, Delay>,
    ) -> Result<Self, Max7456Error<Bus::Error, CsPin::Error>> {
        let transaction = SPITransaction {
            spi: &mut max7456.inner_spi_do_not_use_without_transaction,
            cs: &mut max7456.inner_cs_do_not_use_without_transaction,
            delay: &mut max7456.delay,
        };

        transaction
            .cs
            .set_low()
            .map_err(Max7456Error::<Bus::Error, CsPin::Error>::from_cs_error)?;

        transaction.delay.delay_ns(50).await;

        Ok(transaction)
    }

    async fn read_register<Register: Max7456Register>(
        mut self,
    ) -> Result<Register, Max7456Error<Bus::Error, CsPin::Error>> {
        let address = Register::address() | 0x80;

        let mut buffer = [address, 0xff];
        SpiBusAsync::transfer_in_place(&mut self.spi, &mut buffer).await?;

        Ok(Register::new(buffer[1]))
    }

    async fn write_register<Register: Max7456Register>(
        mut self,
        register: Register,
    ) -> Result<(), Max7456Error<Bus::Error, CsPin::Error>> {
        let address = Register::address() & 0x7f;

        let buffer = [address, register.value()];
        SpiBusAsync::write(&mut self.spi, &buffer).await?;

        self.delay.delay_ns(50).await;

        Ok(())
    }

    async fn write_byte(mut self, byte: u8) -> Result<(), Max7456Error<Bus::Error, CsPin::Error>> {
        SpiBusAsync::write(&mut self.spi, &[byte]).await?;

        self.delay.delay_ns(50).await;

        Ok(())
    }
}

impl<
    'a,
    Bus: SpiBusAsync + SpiBusBlocking,
    CsPin: embedded_hal::digital::OutputPin,
    Delay: embedded_hal_async::delay::DelayNs,
> Drop for SPITransaction<'a, Bus, CsPin, Delay>
{
    fn drop(&mut self) {
        self.cs.set_high().unwrap();
    }
}

/// SPI parameters:
/// bit_order = MsbFirst;
/// mode = MODE_0;
/// frequency = up to 10MHz
pub struct Max7456<
    Bus: SpiBusAsync + SpiBusBlocking,
    CsPin: embedded_hal::digital::OutputPin,
    Delay: embedded_hal_async::delay::DelayNs,
> {
    inner_spi_do_not_use_without_transaction: Bus,
    inner_cs_do_not_use_without_transaction: CsPin,
    delay: Delay,
    video_mode0: VM0,
}

impl<
    Bus: SpiBusAsync + SpiBusBlocking,
    CsPin: embedded_hal::digital::OutputPin,
    Delay: embedded_hal_async::delay::DelayNs,
> Max7456<Bus, CsPin, Delay>
{
    /// SPI parameters:
    /// bit_order = MsbFirst;
    /// mode = MODE_0;
    /// frequency = up to 10MHz
    pub fn new(spi: Bus, cs: CsPin, delay: Delay) -> Self {
        Self {
            inner_cs_do_not_use_without_transaction: cs,
            inner_spi_do_not_use_without_transaction: spi,
            delay,
            video_mode0: Default::default(),
        }
    }

    async fn read_register<Register: Max7456Register>(
        &mut self,
    ) -> Result<Register, Max7456Error<Bus::Error, CsPin::Error>> {
        let transaction = SPITransaction::new(self).await?;

        transaction.read_register().await
    }

    async fn write_register<Register: Max7456Register>(
        &mut self,
        register: Register,
    ) -> Result<(), Max7456Error<Bus::Error, CsPin::Error>> {
        let transaction = SPITransaction::new(self).await?;

        transaction.write_register(register).await
    }

    async fn write_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<(), Max7456Error<Bus::Error, CsPin::Error>> {
        for byte in bytes {
            let transaction = SPITransaction::new(self).await?;
            transaction.write_byte(*byte).await?;
        }
        Ok(())
    }

    pub async fn init(
        &mut self,
        font_pack: Option<&[[u8; 64]; 256]>,
    ) -> Result<(), Max7456Error<Bus::Error, CsPin::Error>> {
        const EXPECTED_OSDM_ON_INIT: u8 = 0b00011011;
        self.delay.delay_ms(50).await;

        let mut osdm: OSDM = OSDM::new(0);
        let mut detected = false;
        for _ in 0..10 {
            osdm = self.read_register().await?;
            if osdm.value() == EXPECTED_OSDM_ON_INIT {
                detected = true;
                break;
            }
            self.delay.delay_ms(10).await;
        }

        if !detected {
            return Err(Max7456Error::DetectTimeout(osdm.value()));
        }

        let mut dmm: DMM = DMM::new(0);
        let mut display_memory_cleared = false;
        for _ in 0..10 {
            dmm = self.read_register().await?;
            if !dmm.memory_clearing_in_progress() {
                display_memory_cleared = true;
                break;
            }
            self.delay.delay_ms(10).await;
        }

        if !display_memory_cleared {
            return Err(Max7456Error::MemoryClearTimeout(dmm.value()));
        }

        if let Some(font_pack) = font_pack {
            self.load_font(font_pack).await?;
        }

        let mut osdbl: OSDBL = self.read_register().await?;
        osdbl.disable_black_level_control(false);
        self.write_register(osdbl).await?;

        self.video_mode0 = self.read_register().await?;
        self.video_mode0.set_osd_enabled(true);
        self.video_mode0.set_pal(true);
        self.write_register(self.video_mode0).await?;
        self.video_mode0 = self.read_register().await?;

        Ok(())
    }

    pub async fn auto_detect_pal_ntsc(
        &mut self,
    ) -> Result<(), Max7456Error<Bus::Error, CsPin::Error>> {
        let stat: STAT = self.read_register().await?;
        if stat.pal_detected() && !self.video_mode0.is_pal() {
            self.video_mode0.set_pal(true);
            self.write_register(self.video_mode0).await?;
            self.video_mode0 = self.read_register().await?;
        } else if stat.ntsc_detected() && self.video_mode0.is_pal() {
            self.video_mode0.set_pal(false);
            self.write_register(self.video_mode0).await?;
            self.video_mode0 = self.read_register().await?;
        }
        Ok(())
    }

    pub async fn write_string_to_display(
        &mut self,
        row: u8,
        col: u8,
        string: &[u8],
    ) -> Result<(), Max7456Error<Bus::Error, CsPin::Error>> {
        let mut dmm: DMM = self.read_register().await?;
        dmm.set_8bit_mode(false);
        dmm.set_auto_increment(true);

        let (dmal, dmah) = DMA::build_address_from_row_column(row, col);
        self.write_register(dmah).await?;
        self.write_register(dmal).await?;

        self.write_register(dmm).await?;

        self.write_bytes(string).await?;

        self.write_bytes(&[0xff]).await?;

        Ok(())
    }

    pub async fn clear_display(&mut self) -> Result<(), Max7456Error<Bus::Error, CsPin::Error>> {
        let mut dmm: DMM = self.read_register().await?;
        dmm.set_clear_memory(true);

        self.write_register(dmm).await?;

        while dmm.memory_clearing_in_progress() {
            dmm = self.read_register().await?;
        }

        Ok(())
    }

    async fn load_font(
        &mut self,
        font_data: &[[u8; 64]; 256],
    ) -> Result<(), Max7456Error<Bus::Error, CsPin::Error>> {
        for character in 0..256usize {
            let character_address = CMA::character_address(character as u8);
            self.write_register(character_address).await.unwrap();
            for byte in 0..54usize {
                let byte_address = CMA::byte_in_character_address(byte as u8);
                self.write_register(byte_address).await.unwrap();

                let cmdi = CMDI::new(font_data[character as usize][byte]);

                self.write_register(cmdi).await.unwrap();
            }

            let cmm = CMM::writing_mode();
            self.write_register(cmm).await.unwrap();

            let mut stat: STAT = STAT::new(0);
            let mut write_done = false;

            for _ in 0..10 {
                stat = self.read_register().await.unwrap();
                if !stat.character_memory_busy() {
                    write_done = true;
                    break;
                }
                self.delay.delay_ms(10).await;
            }

            if !write_done {
                return Err(Max7456Error::CharacterMemoryWriteTimeout(stat.value()));
            }
        }

        Ok(())
    }

    pub async fn wait_for_vsync(&mut self) -> Result<(), Max7456Error<Bus::Error, CsPin::Error>> {
        let mut stat: STAT = self.read_register().await?;

        while stat.vsync_not_in_progress() {
            stat = self.read_register().await?;
        }

        Ok(())
    }
}
