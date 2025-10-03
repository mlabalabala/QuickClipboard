use image::{RgbaImage, ImageBuffer, GrayImage};
use imageproc::template_matching::{match_template_parallel, MatchTemplateMethod};
use rayon::prelude::*;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};

#[derive(Clone)]
pub struct CapturedFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub struct StitchResult {
    pub new_content_y: u32,
    pub new_content_height: u32,
}

pub struct ImageStitcher;

impl ImageStitcher {
    /// BGRA转RGBA（并行转换）
    pub fn bgra_to_rgba_image(bgra: &[u8], width: u32, height: u32) -> RgbaImage {
        let mut rgba = vec![0u8; bgra.len()];
        rgba.par_chunks_exact_mut(4)
            .zip(bgra.par_chunks_exact(4))
            .for_each(|(dst, src)| {
                dst[0] = src[2]; // R
                dst[1] = src[1]; // G
                dst[2] = src[0]; // B
                dst[3] = src[3]; // A
            });
        ImageBuffer::from_raw(width, height, rgba).unwrap()
    }

    pub fn should_stitch_frame_ex(
        last_img: &RgbaImage,
        current_img: &RgbaImage,
        last_content_offset: u32,
        last_content_height: u32,
        current_content_offset: u32,
        current_content_height: u32,
    ) -> Option<StitchResult> {
        if Self::is_duplicate_frame(last_img, current_img) {
            return None;
        }

        let (scroll_amount, match_quality) = match Self::find_best_match_simple(
            last_img, current_img,
            last_content_offset, last_content_height,
            current_content_offset, current_content_height
        ) {
            Some(result) => result,
            None => return None,
        };

        if match_quality > 0.08 { // 拒绝低质量匹配
            return None;
        }

        if scroll_amount < 2 { // 忽略小于2px的抖动
            return None;
        }
        
        let new_content_height = scroll_amount.min(current_content_height);
        let new_content_y = (current_content_offset + current_content_height).saturating_sub(new_content_height);

        if new_content_height > 5 {
            let check_h = new_content_height.min(40);
            let check_y_in_current = (current_content_offset + current_content_height).saturating_sub(check_h);
            let check_y_in_last = (last_content_offset + last_content_height).saturating_sub(check_h);

            let mut diff_pixels = 0;
            let mut total_pixels = 0;
            
            for y in (0..check_h).step_by(8) {
                for x in (0..current_img.width()).step_by(8) {
                    let p_new = current_img.get_pixel(x, check_y_in_current + y);
                    let p_old = last_img.get_pixel(x, check_y_in_last + y);
                    
                    let diff = (p_new[0] as i32 - p_old[0] as i32).abs() +
                               (p_new[1] as i32 - p_old[1] as i32).abs() +
                               (p_new[2] as i32 - p_old[2] as i32).abs();
                    
                    if diff > 30 {
                        diff_pixels += 1;
                    }
                    total_pixels += 1;
                }
            }
            
            if total_pixels > 0 && (diff_pixels as f64 / total_pixels as f64) < 0.10 {
                return None;
            }
        }

        Some(StitchResult {
            new_content_y,
            new_content_height,
        })
    }
    
    /// 高精度帧重复检测
    pub fn is_duplicate_frame(img1: &RgbaImage, img2: &RgbaImage) -> bool {
        if img1.width() != img2.width() || img1.height() != img2.height() {
            return false;
        }

        let step = 8;
        let width = img1.width();
        let height = img1.height();

        let diff_ratio: f64 = (0..height)
            .into_par_iter()
            .step_by(step as usize)
            .map(|y| {
                let mut row_diff = 0usize;
                let mut row_count = 0usize;
                for x in (0..width).step_by(step as usize) {
                    let p1 = img1.get_pixel(x, y);
                    let p2 = img2.get_pixel(x, y);
                    let diff = (p1[0] as i32 - p2[0] as i32).abs() +
                               (p1[1] as i32 - p2[1] as i32).abs() +
                               (p1[2] as i32 - p2[2] as i32).abs();
                    if diff > 30 {
                        row_diff += 1;
                    }
                    row_count += 1;
                }
                row_diff as f64 / row_count as f64
            })
            .sum::<f64>() / ((height / step) as f64);

        let similarity = 1.0 - diff_ratio;
        similarity > 0.99
    }

    fn find_best_match_simple(
        last_img: &RgbaImage,
        current_img: &RgbaImage,
        last_content_offset: u32,
        last_content_height: u32,
        current_content_offset: u32,
        current_content_height: u32,
    ) -> Option<(u32, f64)> {
        let template_height = (current_content_height / 4).max(30).min(80);
        
        let search_start = last_content_offset;
        let search_range = last_content_height;
        let template_start_y = current_content_offset;
        
        if template_start_y + template_height > current_img.height() || search_start + search_range > last_img.height() {
            return None;
        }

        let (match_y_in_search_area, match_quality) = Self::try_match_at_position(
            last_img, current_img, template_start_y, template_height, search_start, search_range
        );
        
        let scroll_amount = match_y_in_search_area;
        
        Some((scroll_amount, match_quality))
    }

    /// 尝试从当前帧指定位置匹配
    fn try_match_at_position(
        last_img: &RgbaImage,
        current_img: &RgbaImage,
        start_y: u32,
        template_height: u32,
        search_start: u32,
        search_range: u32,
    ) -> (u32, f64) {
        let width = last_img.width();
        
        // 从当前帧指定位置提取模板
        let template = Self::extract_sub_image(current_img, 0, start_y, width, template_height);
        let template_gray = Self::rgba_to_gray(&template);
        
        // 在上一帧搜索
        let search_area = Self::extract_sub_image(last_img, 0, search_start, width, search_range);
        let search_gray = Self::rgba_to_gray(&search_area);
        
        let result = match_template_parallel(&search_gray, &template_gray, MatchTemplateMethod::SumOfSquaredErrorsNormalized);

        let mut best_y = 0u32;
        let mut min_error = f64::MAX;

        for (y, row) in result.enumerate_rows() {
            for (x, _col, pixel) in row {
                if x == 0 {
                    let error = pixel[0] as f64 / 255.0;
                    if error < min_error {
                        min_error = error;
                        best_y = y as u32;
                    }
                }
            }
        }

        (best_y, min_error)
    }

    /// RGBA转灰度
    fn rgba_to_gray(rgba_img: &RgbaImage) -> GrayImage {
        let (width, height) = rgba_img.dimensions();
        let mut gray_data = vec![0u8; (width * height) as usize];

        gray_data.par_iter_mut()
            .enumerate()
            .for_each(|(i, pixel)| {
                let rgba_pixel = rgba_img.get_pixel((i as u32) % width, (i as u32) / width);
                *pixel = ((rgba_pixel[0] as u32 * 299 +
                           rgba_pixel[1] as u32 * 587 +
                           rgba_pixel[2] as u32 * 114) / 1000) as u8;
            });

        GrayImage::from_raw(width, height, gray_data).unwrap()
    }

    /// 提取子图像
    fn extract_sub_image(img: &RgbaImage, x: u32, y: u32, width: u32, height: u32) -> RgbaImage {
        let mut sub_img = RgbaImage::new(width, height);
        for dy in 0..height {
            for dx in 0..width {
                let pixel = img.get_pixel(x + dx, y + dy);
                sub_img.put_pixel(dx, dy, *pixel);
            }
        }
        sub_img
    }

    /// 提取指定区域（BGRA格式）
    pub fn extract_region(data: &[u8], width: u32, start_y: u32, extract_height: u32) -> Vec<u8> {
        let bytes_per_pixel = 4;
        let start_idx = (start_y * width * bytes_per_pixel) as usize;
        let end_idx = start_idx + (extract_height * width * bytes_per_pixel) as usize;
        data[start_idx..end_idx].to_vec()
    }

    /// 创建预览图
    pub fn create_preview(frames: &[CapturedFrame]) -> CapturedFrame {
        if frames.is_empty() {
            return CapturedFrame { data: vec![], width: 0, height: 0 };
        }

        let width = frames[0].width;
        let total_height: u32 = frames.iter().map(|f| f.height).sum();

        // 直接拼接所有帧，不进行任何缩放
        let mut stitched_data = Vec::with_capacity((width * total_height * 4) as usize);
        for frame in frames {
            stitched_data.extend_from_slice(&frame.data);
        }

        CapturedFrame {
            data: stitched_data,
            width,
            height: total_height,
        }
    }

    /// BGRA转PNG
    pub fn bgra_to_png(bgra: &[u8], width: u32, height: u32) -> Vec<u8> {
        let mut rgba = vec![0u8; bgra.len()];
        rgba.par_chunks_exact_mut(4)
            .zip(bgra.par_chunks_exact(4))
            .for_each(|(dst, src)| {
                dst[0] = src[2];
                dst[1] = src[1];
                dst[2] = src[0];
                dst[3] = src[3];
            });

        let mut png_bytes = Vec::new();
        let encoder = PngEncoder::new(&mut png_bytes);
        let _ = encoder.write_image(&rgba, width, height, ExtendedColorType::Rgba8);
        png_bytes
    }
}
