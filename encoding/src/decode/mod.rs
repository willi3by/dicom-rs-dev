//! This module contains all DICOM data element decoding logic.

use self::explicit_le::ExplicitVRLittleEndianDecoder;
use self::implicit_le::{ImplicitVRLittleEndianDecoder, StandardImplicitVRLittleEndianDecoder};
use byteordered::Endianness;
use dicom_core::header::{DataElementHeader, SequenceItemHeader};
use dicom_core::Tag;
use snafu::{Backtrace, Snafu};
use std::io::{self, Read};

pub mod basic;
pub mod explicit_be;
pub mod explicit_le;
pub mod implicit_le;

/// Module-level error type:
/// for errors which may occur while decoding DICOM data.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    #[snafu(display("Failed to read the beginning (tag) of the header"))]
    ReadHeaderTag {
        backtrace: Option<Backtrace>,
        source: io::Error,
    },
    #[snafu(display("Failed to read the item header"))]
    ReadItemHeader {
        backtrace: Backtrace,
        source: io::Error,
    },
    #[snafu(display("Failed to read the header's item length field"))]
    ReadItemLength {
        backtrace: Backtrace,
        source: io::Error,
    },
    #[snafu(display("Failed to read the header's tag field"))]
    ReadTag {
        backtrace: Backtrace,
        source: io::Error,
    },
    #[snafu(display("Failed to read the header's reserved bytes"))]
    ReadReserved {
        backtrace: Backtrace,
        source: io::Error,
    },
    #[snafu(display("Failed to read the header's element length field"))]
    ReadLength {
        backtrace: Backtrace,
        source: io::Error,
    },
    #[snafu(display("Failed to read the header's value representation"))]
    ReadVr {
        backtrace: Backtrace,
        source: io::Error,
    },
    #[snafu(display("Bad sequence item header"))]
    BadSequenceHeader {
        source: dicom_core::header::SequenceItemHeaderError,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

/** Obtain the default data element decoder.
 * According to the standard, data elements are encoded in Implicit
 * VR Little Endian by default.
 */
pub fn default_reader() -> StandardImplicitVRLittleEndianDecoder {
    ImplicitVRLittleEndianDecoder::default()
}

/** Obtain a data element decoder for reading the data elements in a DICOM
 * file's Meta information. According to the standard, these are always
 * encoded in Explicit VR Little Endian.
 */
pub fn file_header_decoder() -> ExplicitVRLittleEndianDecoder {
    ExplicitVRLittleEndianDecoder::default()
}

/** Type trait for reading and decoding basic data values from a data source.
 *
 * This trait aims to provide methods for reading binary numbers based on the
 * source's endianness. Unlike `Decode`, this trait is not object safe.
 * However, it doesn't have to because there are, and only will be, two
 * possible implementations (`LittleEndianBasicDecoder` and
 * `BigEndianBasicDecoder`).
 */
pub trait BasicDecode {
    /// Retrieve the source's endianness, as expected by this decoder.
    fn endianness(&self) -> Endianness;

    /// Decode an unsigned short value from the given source.
    fn decode_us<S>(&self, source: S) -> std::io::Result<u16>
    where
        S: Read;

    /// Decode a sequence of unsigned shorts value from the given source
    /// into the given destination.
    fn decode_us_into<S>(&self, mut source: S, dst: &mut [u16]) -> std::io::Result<()>
    where
        S: Read,
    {
        for v in dst.iter_mut() {
            *v = self.decode_us(&mut source)?;
        }

        Ok(())
    }

    /// Decode an unsigned long value from the given source.
    fn decode_ul<S>(&self, source: S) -> std::io::Result<u32>
    where
        S: Read;

    /// Decode a sequence of unsigned long values from the given source
    /// into the given destination.
    fn decode_ul_into<S>(&self, mut source: S, dst: &mut [u32]) -> std::io::Result<()>
    where
        S: Read,
    {
        for v in dst.iter_mut() {
            *v = self.decode_ul(&mut source)?;
        }

        Ok(())
    }

    /// Decode an unsigned very long value from the given source.
    fn decode_uv<S>(&self, source: S) -> std::io::Result<u64>
    where
        S: Read;

    /// Decode a sequence of unsigned very long values from the given source
    /// into the given destination.
    fn decode_uv_into<S>(&self, mut source: S, dst: &mut [u64]) -> std::io::Result<()>
    where
        S: Read,
    {
        for v in dst.iter_mut() {
            *v = self.decode_uv(&mut source)?;
        }

        Ok(())
    }

    /// Decode a signed short value from the given source.
    fn decode_ss<S>(&self, source: S) -> std::io::Result<i16>
    where
        S: Read;

    /// Decode a sequence of signed short values from the given source
    /// into the given destination.
    fn decode_ss_into<S>(&self, mut source: S, dst: &mut [i16]) -> std::io::Result<()>
    where
        S: Read,
    {
        for v in dst.iter_mut() {
            *v = self.decode_ss(&mut source)?;
        }

        Ok(())
    }

    /// Decode a signed long value from the given source.
    fn decode_sl<S>(&self, source: S) -> std::io::Result<i32>
    where
        S: Read;

    /// Decode a sequence of signed long values from the given source
    /// into the given destination.
    fn decode_sl_into<S>(&self, mut source: S, dst: &mut [i32]) -> std::io::Result<()>
    where
        S: Read,
    {
        for v in dst.iter_mut() {
            *v = self.decode_sl(&mut source)?;
        }

        Ok(())
    }

    /// Decode a signed very long value from the given source.
    fn decode_sv<S>(&self, source: S) -> std::io::Result<i64>
    where
        S: Read;

    /// Decode a sequence of signed very long values from the given source
    /// into the given destination.
    fn decode_sv_into<S>(&self, mut source: S, dst: &mut [i64]) -> std::io::Result<()>
    where
        S: Read,
    {
        for v in dst.iter_mut() {
            *v = self.decode_sv(&mut source)?;
        }

        Ok(())
    }

    /// Decode a single precision float value from the given source.
    fn decode_fl<S>(&self, source: S) -> std::io::Result<f32>
    where
        S: Read;

    /// Decode a sequence of single precision float values from the given source
    /// into the given destination.
    fn decode_fl_into<S>(&self, mut source: S, dst: &mut [f32]) -> std::io::Result<()>
    where
        S: Read,
    {
        for v in dst.iter_mut() {
            *v = self.decode_fl(&mut source)?;
        }

        Ok(())
    }

    /// Decode a double precision float value from the given source.
    fn decode_fd<S>(&self, source: S) -> std::io::Result<f64>
    where
        S: Read;

    /// Decode a sequence of double precision float values from the given source
    /// into the given destination.
    fn decode_fd_into<S>(&self, mut source: S, dst: &mut [f64]) -> std::io::Result<()>
    where
        S: Read,
    {
        for v in dst.iter_mut() {
            *v = self.decode_fd(&mut source)?;
        }

        Ok(())
    }

    /// Decode a DICOM attribute tag from the given source.
    fn decode_tag<S>(&self, mut source: S) -> std::io::Result<Tag>
    where
        S: Read,
    {
        let g = self.decode_us(&mut source)?;
        let e = self.decode_us(source)?;
        Ok(Tag(g, e))
    }
}

impl<T: ?Sized> BasicDecode for Box<T>
where
    T: BasicDecode,
{
    fn endianness(&self) -> Endianness {
        (**self).endianness()
    }

    fn decode_us<S>(&self, source: S) -> std::io::Result<u16>
    where
        S: Read,
    {
        (**self).decode_us(source)
    }

    fn decode_us_into<S>(&self, source: S, dst: &mut [u16]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_us_into(source, dst)
    }

    fn decode_ul<S>(&self, source: S) -> std::io::Result<u32>
    where
        S: Read,
    {
        (**self).decode_ul(source)
    }

    fn decode_ul_into<S>(&self, source: S, dst: &mut [u32]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_ul_into(source, dst)
    }

    fn decode_uv<S>(&self, source: S) -> std::io::Result<u64>
    where
        S: Read,
    {
        (**self).decode_uv(source)
    }

    fn decode_uv_into<S>(&self, source: S, dst: &mut [u64]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_uv_into(source, dst)
    }

    fn decode_ss<S>(&self, source: S) -> std::io::Result<i16>
    where
        S: Read,
    {
        (**self).decode_ss(source)
    }

    fn decode_ss_into<S>(&self, source: S, dst: &mut [i16]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_ss_into(source, dst)
    }

    fn decode_sl<S>(&self, source: S) -> std::io::Result<i32>
    where
        S: Read,
    {
        (**self).decode_sl(source)
    }

    fn decode_sl_into<S>(&self, source: S, dst: &mut [i32]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_sl_into(source, dst)
    }

    fn decode_sv<S>(&self, source: S) -> std::io::Result<i64>
    where
        S: Read,
    {
        (**self).decode_sv(source)
    }

    fn decode_sv_into<S>(&self, source: S, dst: &mut [i64]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_sv_into(source, dst)
    }

    fn decode_fl<S>(&self, source: S) -> std::io::Result<f32>
    where
        S: Read,
    {
        (**self).decode_fl(source)
    }

    fn decode_fl_into<S>(&self, source: S, dst: &mut [f32]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_fl_into(source, dst)
    }

    fn decode_fd<S>(&self, source: S) -> std::io::Result<f64>
    where
        S: Read,
    {
        (**self).decode_fd(source)
    }

    fn decode_fd_into<S>(&self, source: S, dst: &mut [f64]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_fd_into(source, dst)
    }

    fn decode_tag<S>(&self, source: S) -> std::io::Result<Tag>
    where
        S: Read,
    {
        (**self).decode_tag(source)
    }
}

impl<'a, T: ?Sized> BasicDecode for &'a T
where
    T: BasicDecode,
{
    fn endianness(&self) -> Endianness {
        (**self).endianness()
    }

    fn decode_us<S>(&self, source: S) -> std::io::Result<u16>
    where
        S: Read,
    {
        (**self).decode_us(source)
    }

    fn decode_us_into<S>(&self, source: S, dst: &mut [u16]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_us_into(source, dst)
    }

    fn decode_ul<S>(&self, source: S) -> std::io::Result<u32>
    where
        S: Read,
    {
        (**self).decode_ul(source)
    }

    fn decode_ul_into<S>(&self, source: S, dst: &mut [u32]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_ul_into(source, dst)
    }

    fn decode_uv<S>(&self, source: S) -> std::io::Result<u64>
    where
        S: Read,
    {
        (**self).decode_uv(source)
    }

    fn decode_uv_into<S>(&self, source: S, dst: &mut [u64]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_uv_into(source, dst)
    }

    fn decode_ss<S>(&self, source: S) -> std::io::Result<i16>
    where
        S: Read,
    {
        (**self).decode_ss(source)
    }

    fn decode_ss_into<S>(&self, source: S, dst: &mut [i16]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_ss_into(source, dst)
    }

    fn decode_sl<S>(&self, source: S) -> std::io::Result<i32>
    where
        S: Read,
    {
        (**self).decode_sl(source)
    }

    fn decode_sl_into<S>(&self, source: S, dst: &mut [i32]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_sl_into(source, dst)
    }

    fn decode_sv<S>(&self, source: S) -> std::io::Result<i64>
    where
        S: Read,
    {
        (**self).decode_sv(source)
    }

    fn decode_sv_into<S>(&self, source: S, dst: &mut [i64]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_sv_into(source, dst)
    }

    fn decode_fl<S>(&self, source: S) -> std::io::Result<f32>
    where
        S: Read,
    {
        (**self).decode_fl(source)
    }

    fn decode_fl_into<S>(&self, source: S, dst: &mut [f32]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_fl_into(source, dst)
    }

    fn decode_fd<S>(&self, source: S) -> std::io::Result<f64>
    where
        S: Read,
    {
        (**self).decode_fd(source)
    }

    fn decode_fd_into<S>(&self, source: S, dst: &mut [f64]) -> std::io::Result<()>
    where
        S: Read,
    {
        (**self).decode_fd_into(source, dst)
    }

    fn decode_tag<S>(&self, source: S) -> std::io::Result<Tag>
    where
        S: Read,
    {
        (**self).decode_tag(source)
    }
}

/** Type trait for reading and decoding DICOM data elements.
 *
 * The specific behaviour of decoding, even when abstracted from the original source,
 * may depend on the transfer syntax.
 */
pub trait Decode {
    /** Fetch and decode the next data element header from the given source.
     * This method returns only the header of the element. At the end of this operation, the source
     * will be pointing at the element's value data, which should be read or skipped as necessary.
     *
     * Decoding an item or sequence delimiter is considered valid, and so should be properly handled
     * by the decoder. The value representation in this case should be `UN`.
     *
     * Returns the expected header and the exact number of bytes read from the source.
     */
    fn decode_header<S>(&self, source: &mut S) -> Result<(DataElementHeader, usize)>
    where
        S: ?Sized + Read;

    /** Fetch and decode the next sequence item head from the given source. It is a separate method
     * because value representation is always implicit when reading item headers and delimiters.
     * This method returns only the header of the item. At the end of this operation, the source
     * will be pointing at the beginning of the item's data, which should be traversed if necessary.
     */
    fn decode_item_header<S>(&self, source: &mut S) -> Result<SequenceItemHeader>
    where
        S: ?Sized + Read;

    /// Decode a DICOM attribute tag from the given source.
    fn decode_tag<S>(&self, source: &mut S) -> Result<Tag>
    where
        S: ?Sized + Read;
}

impl<T: ?Sized> Decode for Box<T>
where
    T: Decode,
{
    fn decode_header<S>(&self, source: &mut S) -> Result<(DataElementHeader, usize)>
    where
        S: ?Sized + Read,
    {
        (**self).decode_header(source)
    }

    fn decode_item_header<S>(&self, source: &mut S) -> Result<SequenceItemHeader>
    where
        S: ?Sized + Read,
    {
        (**self).decode_item_header(source)
    }

    fn decode_tag<S>(&self, source: &mut S) -> Result<Tag>
    where
        S: ?Sized + Read,
    {
        (**self).decode_tag(source)
    }
}

impl<'a, T: ?Sized> Decode for &'a T
where
    T: Decode,
{
    fn decode_header<S>(&self, source: &mut S) -> Result<(DataElementHeader, usize)>
    where
        S: ?Sized + Read,
    {
        (**self).decode_header(source)
    }

    fn decode_item_header<S>(&self, source: &mut S) -> Result<SequenceItemHeader>
    where
        S: ?Sized + Read,
    {
        (**self).decode_item_header(source)
    }

    fn decode_tag<S>(&self, source: &mut S) -> Result<Tag>
    where
        S: ?Sized + Read,
    {
        (**self).decode_tag(source)
    }
}

/** Type trait for reading and decoding DICOM data elements from a specific source
 * reader type.
 *
 * The specific behaviour of decoding, even when abstracted from the original source,
 * may depend on the transfer syntax.
 */
pub trait DecodeFrom<S: ?Sized + Read> {
    /** Fetch and decode the next data element header from the given source.
     * This method returns only the header of the element. At the end of this operation, the source
     * will be pointing at the element's value data, which should be read or skipped as necessary.
     *
     * Decoding an item or sequence delimiter is considered valid, and so should be properly handled
     * by the decoder. The value representation in this case should be `UN`.
     *
     * Returns the expected header and the exact number of bytes read from the source.
     */
    fn decode_header(&self, source: &mut S) -> Result<(DataElementHeader, usize)>;

    /** Fetch and decode the next sequence item head from the given source. It is a separate method
     * because value representation is always implicit when reading item headers and delimiters.
     * This method returns only the header of the item. At the end of this operation, the source
     * will be pointing at the beginning of the item's data, which should be traversed if necessary.
     */
    fn decode_item_header(&self, source: &mut S) -> Result<SequenceItemHeader>;

    /// Decode a DICOM attribute tag from the given source.
    fn decode_tag(&self, source: &mut S) -> Result<Tag>;
}

impl<S: ?Sized, T: ?Sized> DecodeFrom<S> for &T
where
    S: Read,
    T: DecodeFrom<S>,
{
    fn decode_header(&self, source: &mut S) -> Result<(DataElementHeader, usize)> {
        (**self).decode_header(source)
    }

    fn decode_item_header(&self, source: &mut S) -> Result<SequenceItemHeader> {
        (**self).decode_item_header(source)
    }

    fn decode_tag(&self, source: &mut S) -> Result<Tag> {
        (**self).decode_tag(source)
    }
}

impl<S: ?Sized, T: ?Sized> DecodeFrom<S> for Box<T>
where
    S: Read,
    T: DecodeFrom<S>,
{
    fn decode_header(&self, source: &mut S) -> Result<(DataElementHeader, usize)> {
        (**self).decode_header(source)
    }

    fn decode_item_header(&self, source: &mut S) -> Result<SequenceItemHeader> {
        (**self).decode_item_header(source)
    }

    fn decode_tag(&self, source: &mut S) -> Result<Tag> {
        (**self).decode_tag(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_decode_from<T: DecodeFrom<dyn Read>>(_decoder: &T) {}

    #[allow(unused)]
    fn boxed_decoder_from_is_decoder_from<T>(decoder: T)
    where
        T: DecodeFrom<dyn Read>,
    {
        is_decode_from(&decoder);
        let boxed = Box::new(decoder);
        is_decode_from(&boxed);
        let erased = boxed as Box<dyn DecodeFrom<dyn Read>>;
        is_decode_from(&erased);
    }
}
