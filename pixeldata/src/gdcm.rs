//! Decode pixel data using GDCM when the default features are enabled.

use crate::*;
use dicom_encoding::adapters::DecodeError;
use dicom_encoding::transfer_syntax::TransferSyntaxIndex;
use dicom_transfer_syntax_registry::TransferSyntaxRegistry;
use gdcm_rs::{
    decode_multi_frame_compressed, decode_single_frame_compressed, Error as GDCMError,
    GDCMPhotometricInterpretation, GDCMTransferSyntax,
};
use std::{convert::TryFrom, str::FromStr};

impl<D> PixelDecoder for FileDicomObject<InMemDicomObject<D>>
where
    D: DataDictionary + Clone,
{
    fn decode_pixel_data(&self) -> Result<DecodedPixelData> {
        use super::attribute::*;

        let pixel_data = pixel_data(self).context(GetAttributeSnafu)?;
        let cols = cols(self).context(GetAttributeSnafu)?;
        let rows = rows(self).context(GetAttributeSnafu)?;

        let photometric_interpretation =
            photometric_interpretation(self).context(GetAttributeSnafu)?;
        let pi_type = GDCMPhotometricInterpretation::from_str(photometric_interpretation.as_str())
            .map_err(|_| {
                UnsupportedPhotometricInterpretationSnafu {
                    pi: photometric_interpretation.clone(),
                }
                .build()
            })?;

        let transfer_syntax = &self.meta().transfer_syntax;
        let registry =
            TransferSyntaxRegistry
                .get(&&transfer_syntax)
                .context(UnknownTransferSyntaxSnafu {
                    ts_uid: transfer_syntax,
                })?;
        let ts_type = GDCMTransferSyntax::from_str(&registry.uid()).map_err(|_| {
            UnsupportedTransferSyntaxSnafu {
                ts: transfer_syntax.clone(),
            }
            .build()
        })?;

        let samples_per_pixel = samples_per_pixel(self).context(GetAttributeSnafu)?;
        let bits_allocated = bits_allocated(self).context(GetAttributeSnafu)?;
        let bits_stored = bits_stored(self).context(GetAttributeSnafu)?;
        let high_bit = high_bit(self).context(GetAttributeSnafu)?;
        let pixel_representation = pixel_representation(self).context(GetAttributeSnafu)?;
        let rescale_intercept = rescale_intercept(self);
        let rescale_slope = rescale_slope(self);
        let number_of_frames = number_of_frames(self).context(GetAttributeSnafu)?;
        let voi_lut_function = voi_lut_function(self).context(GetAttributeSnafu)?;
        let voi_lut_function = voi_lut_function.and_then(|v| VoiLutFunction::try_from(&*v).ok());

        let decoded_pixel_data = match pixel_data.value() {
            Value::PixelSequence(v) => {
                let fragments = v.fragments();
                let gdcm_error_mapper = |source: GDCMError| DecodeError::Custom {
                    message: source.to_string(),
                    source: Some(Box::new(source)),
                };
                if fragments.len() > 1 {
                    // Bundle fragments and decode multi-frame dicoms
                    let dims = [cols.into(), rows.into(), number_of_frames.into()];
                    let fragments: Vec<_> = fragments.iter().map(|frag| frag.as_slice()).collect();
                    decode_multi_frame_compressed(
                        fragments.as_slice(),
                        &dims,
                        pi_type,
                        ts_type,
                        samples_per_pixel,
                        bits_allocated,
                        bits_stored,
                        high_bit,
                        pixel_representation as u16,
                    )
                    .map_err(gdcm_error_mapper)
                    .context(DecodePixelDataSnafu)?
                    .to_vec()
                } else {
                    decode_single_frame_compressed(
                        &fragments[0],
                        cols.into(),
                        rows.into(),
                        pi_type,
                        ts_type,
                        samples_per_pixel,
                        bits_allocated,
                        bits_stored,
                        high_bit,
                        pixel_representation as u16,
                    )
                    .map_err(gdcm_error_mapper)
                    .context(DecodePixelDataSnafu)?
                    .to_vec()
                }
            }
            Value::Primitive(p) => {
                // Non-encoded, just return the pixel data of the first frame
                p.to_bytes().to_vec()
            }
            Value::Sequence(_) => InvalidPixelDataSnafu.fail()?,
        };

        // pixels are already interpreted,
        // set new photometric interpretation
        let new_pi = match samples_per_pixel {
            1 => PhotometricInterpretation::Monochrome2,
            3 => PhotometricInterpretation::Rgb,
            _ => photometric_interpretation,
        };

        let window = if let Some(window_center) = window_center(self).context(GetAttributeSnafu)? {
            let window_width = window_width(self).context(GetAttributeSnafu)?;

            window_width.map(|width| WindowLevel {
                center: window_center,
                width,
            })
        } else {
            None
        };

        Ok(DecodedPixelData {
            data: Cow::from(decoded_pixel_data),
            cols: cols.into(),
            rows: rows.into(),
            number_of_frames,
            photometric_interpretation: new_pi,
            samples_per_pixel,
            planar_configuration: PlanarConfiguration::Standard,
            bits_allocated,
            bits_stored,
            high_bit,
            pixel_representation,
            rescale_intercept,
            rescale_slope,
            voi_lut_function,
            window,
        })
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(feature = "ndarray", feature = "image"))]
    use super::*;
    #[cfg(any(feature = "ndarray", feature = "image"))]
    use dicom_object::open_file;
    #[cfg(feature = "image")]
    use rstest::rstest;
    #[cfg(feature = "image")]
    use std::path::Path;

    #[cfg(feature = "image")]
    const MAX_TEST_FRAMES: u32 = 16;

    #[cfg(feature = "image")]
    #[rstest]
    #[case("pydicom/693_J2KI.dcm")]
    #[case("pydicom/693_J2KR.dcm")]
    #[case("pydicom/693_UNCI.dcm")]
    #[case("pydicom/693_UNCR.dcm")]
    #[case("pydicom/CT_small.dcm")]
    #[case("pydicom/JPEG-lossy.dcm")]
    #[case("pydicom/JPEG2000.dcm")]
    #[case("pydicom/JPEG2000_UNC.dcm")]
    #[case("pydicom/JPGLosslessP14SV1_1s_1f_8b.dcm")]
    #[case("pydicom/MR_small.dcm")]
    #[case("pydicom/MR_small_RLE.dcm")]
    #[case("pydicom/MR_small_implicit.dcm")]
    #[case("pydicom/MR_small_jp2klossless.dcm")]
    #[case("pydicom/MR_small_jpeg_ls_lossless.dcm")]
    #[case("pydicom/explicit_VR-UN.dcm")]
    #[case("pydicom/MR_small_bigendian.dcm")]
    #[case("pydicom/MR_small_expb.dcm")]
    #[case("pydicom/SC_rgb.dcm")]
    #[case("pydicom/SC_rgb_16bit.dcm")]
    #[case("pydicom/SC_rgb_dcmtk_+eb+cr.dcm")]
    #[case("pydicom/SC_rgb_expb.dcm")]
    #[case("pydicom/SC_rgb_expb_16bit.dcm")]
    #[case("pydicom/SC_rgb_gdcm2k_uncompressed.dcm")]
    #[case("pydicom/SC_rgb_gdcm_KY.dcm")]
    #[case("pydicom/SC_rgb_jpeg_gdcm.dcm")]
    #[case("pydicom/SC_rgb_jpeg_lossy_gdcm.dcm")]
    #[case("pydicom/SC_rgb_rle.dcm")]
    #[case("pydicom/SC_rgb_rle_16bit.dcm")]
    #[case("pydicom/color-pl.dcm")]
    #[case("pydicom/color-px.dcm")]
    #[case("pydicom/SC_ybr_full_uncompressed.dcm")]
    #[case("pydicom/color3d_jpeg_baseline.dcm")]
    #[case("pydicom/emri_small_jpeg_ls_lossless.dcm")]
    fn test_parse_dicom_pixel_data(#[case] value: &str) {
        let test_file = dicom_test_files::path(value).unwrap();
        println!("Parsing pixel data for {}", test_file.display());
        let obj = open_file(test_file).unwrap();
        let pixel_data = obj.decode_pixel_data().unwrap();
        let output_dir =
            Path::new("../target/dicom_test_files/_out/test_gdcm_parse_dicom_pixel_data");
        std::fs::create_dir_all(output_dir).unwrap();

        for i in 0..pixel_data.number_of_frames.min(MAX_TEST_FRAMES) {
            let image = pixel_data.to_dynamic_image(i).unwrap();
            let image_path = output_dir.join(format!(
                "{}-{}.png",
                Path::new(value).file_stem().unwrap().to_str().unwrap(),
                i,
            ));
            image.save(image_path).unwrap();
        }
    }

    #[cfg(feature = "ndarray")]
    #[test]
    fn test_to_ndarray_signed_word_no_lut() {
        let test_file = dicom_test_files::path("pydicom/JPEG2000.dcm").unwrap();
        let obj = open_file(test_file).unwrap();
        let options = ConvertOptions::new().with_modality_lut(ModalityLutOption::None);
        let ndarray = obj
            .decode_pixel_data()
            .unwrap()
            .to_ndarray_with_options::<i16>(&options)
            .unwrap();
        assert_eq!(ndarray.shape(), &[1, 1024, 256, 1]);
        assert_eq!(ndarray.len(), 262144);
        assert_eq!(ndarray[[0, 260, 0, 0]], -3);
    }
}
