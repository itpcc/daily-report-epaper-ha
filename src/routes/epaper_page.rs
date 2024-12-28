use ab_glyph::{FontRef, InvalidFont, PxScale};
use axum::{
    extract::{Query, State},
    http::HeaderValue,
    response::IntoResponse,
};
use image::{
    imageops::{
        colorops::{contrast_in_place, dither},
        BiLevel,
    },
    Luma, Pixel, Rgba,
};
use imageproc::{
    drawing,
    image::{ImageFormat, Rgb, RgbImage},
    map::map_pixels,
    rect::Rect,
};
use itertools::Itertools;
use std::io::{BufWriter, Cursor};
use time::{Month, Weekday};
use time_tz::{timezones, OffsetDateTimeExt};

use crate::{
    api_error::ApiError,
    model::{
        CalendarMap, QueryRouteEPaperModel, QueryRouteEPaperOutputEnum as OutputEnum,
        WeatherInfoState,
    },
    AppState,
};

fn month_abbr(mth: Month) -> String {
    match mth {
        Month::January => "Jan",
        Month::February => "Feb",
        Month::March => "Mar",
        Month::April => "Apr",
        Month::May => "May",
        Month::June => "Jun",
        Month::July => "Jul",
        Month::August => "Aug",
        Month::September => "Sep",
        Month::October => "Oct",
        Month::November => "Nov",
        Month::December => "Dec",
    }
    .to_string()
    .to_uppercase()
}

fn substr_th(str: String, len: usize) -> String {
    let mut cnt = 0_usize;

    if str.chars().count() <= len {
        return str;
    }

    let mut res_str = str
        .chars()
        .take_while_inclusive(|c| {
            if matches!(c, 'ั' | 'ุ' ..= '\u{E3E}' | '็' ..= '๎') {
                return true;
            }

            cnt += 1;

            cnt <= len
        })
        .collect::<String>();

    res_str.push('…');

    res_str
}

pub async fn epaper_page(
    State(state): State<AppState>,
    Query(q): Query<QueryRouteEPaperModel>,
) -> impl IntoResponse {
    let time_utc = time::OffsetDateTime::now_utc();
    let time_local =
        time_utc.to_timezone(timezones::get_by_name("Asia/Bangkok").unwrap_or(timezones::db::UTC));
    let time_date = time_local.date();
    let calendar = {
        let clnd = state.calendar.read().await;
        let mut clnd_n = CalendarMap::new();

        clnd.range(time_date..).take(9).for_each(|(c_date, c_nf)| {
            clnd_n.insert(*c_date, c_nf.clone());
        });

        clnd_n
    };
    let is_holiday = match time_date.weekday() {
        Weekday::Sunday | Weekday::Saturday => true,
        _ => calendar
            .get(&time_date)
            .map(|c| c.holiday.is_some())
            .unwrap_or(false),
    };
    let is_event = calendar
        .get(&time_date)
        .map(|c| !c.events.is_empty())
        .unwrap_or(false);

    // Colours
    let red = Rgb([255u8, 0u8, 0u8]);
    let black = Rgb([0u8, 0u8, 0u8]);
    let gray = Rgb([137u8, 136u8, 136u8]);
    let white = Rgb([255u8, 255u8, 255u8]);
    // Fonts
    let (font_anta, font_chakra_r, font_chakra_b, font_chakra_sb, font_material) = match (|| {
        Ok::<
            (
                FontRef<'_>,
                FontRef<'_>,
                FontRef<'_>,
                FontRef<'_>,
                FontRef<'_>,
            ),
            InvalidFont,
        >((
            FontRef::try_from_slice(include_bytes!("../../fonts/Anta/Anta-Regular.ttf"))?,
            FontRef::try_from_slice(include_bytes!(
                "../../fonts/Chakra_Petch/ChakraPetch-Regular.ttf"
            ))?,
            FontRef::try_from_slice(include_bytes!(
                "../../fonts/Chakra_Petch/ChakraPetch-Bold.ttf"
            ))?,
            FontRef::try_from_slice(include_bytes!(
                "../../fonts/Chakra_Petch/ChakraPetch-SemiBold.ttf"
            ))?,
            FontRef::try_from_slice(include_bytes!(
                "../../fonts/materialdesignicons-webfont.ttf"
            ))?,
        ))
    })() {
        Err(e) => {
            return ApiError::InternalError(e.into()).into_response();
        }
        Ok(f) => f,
    };

    // Image base
    let mut image = RgbImage::new(400, 300);
    let img_w = image.width();
    let img_h = image.height();
    let border_px = 10_u32;
    let last_update_y = img_h - border_px - 10;

    // Background
    drawing::draw_filled_rect_mut(&mut image, Rect::at(0, 0).of_size(img_w, img_h), white);

    // * today
    // Date section
    // Date day
    let date_day_box_w = 90;
    let date_day_box_h = 120;
    let date_day_box_l = border_px + 10;
    let date_day_box_t = border_px;
    let left_box_w = date_day_box_w + (date_day_box_l * 2);
    drawing::draw_filled_rect_mut(
        &mut image,
        Rect::at(date_day_box_l as i32, date_day_box_t as i32)
            .of_size(date_day_box_w, date_day_box_h),
        match is_holiday {
            true => red,
            false => black,
        },
    );

    if is_event {
        // draw dots
        (0..date_day_box_w).step_by(4).for_each(|x| {
            (0..date_day_box_h).step_by(4).for_each(|y| {
                drawing::draw_cross_mut(
                    &mut image,
                    white,
                    (date_day_box_l + x) as i32,
                    (date_day_box_t + y) as i32,
                );
            });
        });
    }

    let date_day_str = time_local.day().to_string();
    let date_day_scale = PxScale {
        x: 90.0,
        y: 90.0 * 1.5,
    };
    let (date_day_txt_sz_w, _date_day_txt_sz_h) =
        drawing::text_size(date_day_scale, &font_anta, &date_day_str);
    drawing::draw_text_mut(
        &mut image,
        white,
        (border_px
            + (u32::max(
                (border_px as f64 * 0.75) as u32,
                date_day_box_w.abs_diff(date_day_txt_sz_w) / 2,
            ))) as i32,
        border_px as i32,
        date_day_scale,
        &font_anta,
        &date_day_str,
    );
    // Date month
    let date_mth_str = month_abbr(time_local.month());
    let date_mth_scale = PxScale { x: 60.0, y: 60.0 };
    let (date_mth_txt_sz_w, date_mth_txt_sz_h) =
        drawing::text_size(date_mth_scale, &font_anta, &date_mth_str);
    drawing::draw_text_mut(
        &mut image,
        black,
        (border_px + (date_mth_txt_sz_w.abs_diff(date_day_box_w) / 2)) as i32,
        (border_px + date_day_box_h) as i32,
        date_mth_scale,
        &font_anta,
        &date_mth_str,
    );
    // Date Year
    let date_yr_str = (time_local.year() % 100).to_string();
    let date_yr_scale = PxScale { x: 90.0, y: 90.0 };
    let (date_yr_txt_sz_w, _date_yr_txt_sz_h) =
        drawing::text_size(date_yr_scale, &font_anta, &date_yr_str);
    drawing::draw_text_mut(
        &mut image,
        black,
        (border_px + (date_yr_txt_sz_w.abs_diff(date_day_box_w + date_day_box_l) / 2)) as i32,
        (border_px + date_day_box_h + date_mth_txt_sz_h + border_px) as i32,
        date_yr_scale,
        &font_anta,
        &date_yr_str,
    );

    // * Weather
    // See: https://community.home-assistant.io/t/display-materialdesign-icons-on-esphome-attached-to-screen/199790/16
    {
        let weather = state.weather.read().await;
        let weather_icon = match weather.as_ref().map(|w| w.state.clone()) {
            Some(WeatherInfoState::Cloudy) => "\u{0F0590}",
            Some(WeatherInfoState::Fog) => "\u{0F0591}",
            Some(WeatherInfoState::Hail) => "\u{0F0592}",
            Some(WeatherInfoState::Lightning) => "\u{0F0593}",
            Some(WeatherInfoState::LightningRainy) => "\u{0F067E}",
            Some(WeatherInfoState::ClearNight) => "\u{0F0594}",
            Some(WeatherInfoState::Partlycloudy) => "\u{0F0595}",
            Some(WeatherInfoState::Pouring) => "\u{0F0596}",
            Some(WeatherInfoState::Rainy) => "\u{0F0597}",
            Some(WeatherInfoState::Snowy) => "\u{0F0598}",
            Some(WeatherInfoState::SnowyRainy) => "\u{0F067F}",
            Some(WeatherInfoState::Sunny) => "\u{0F0599}",
            Some(WeatherInfoState::Windy) => "\u{0F059D}",
            Some(WeatherInfoState::WindyVariant) => "\u{0F059E}",
            Some(WeatherInfoState::Exceptional) => "?",
            None => "?",
        };
        let weather_icon_x = left_box_w + border_px + border_px;
        let weather_icon_fnt_sz = 45.0_f32;
        drawing::draw_text_mut(
            &mut image,
            black,
            (left_box_w + border_px + border_px) as i32,
            border_px as i32,
            PxScale {
                x: weather_icon_fnt_sz,
                y: weather_icon_fnt_sz,
            },
            &font_material,
            weather_icon,
        );
        if let Some(w) = weather.as_ref() {
            drawing::draw_text_mut(
                &mut image,
                black,
                (weather_icon_x + border_px + 45) as i32,
                (border_px + (f32::abs(weather_icon_fnt_sz - 30.0) / 2.0) as u32) as i32,
                PxScale { x: 30.0, y: 30.0 },
                &font_anta,
                &format! {"{:.1}{}", w.attributes.temperature, w.attributes.temperature_unit},
            );
        }
    }

    // * Event
    let status_h = 40;
    let event_fnt_sz = 16;
    let event_fnt_scale = PxScale {
        x: event_fnt_sz as f32,
        y: event_fnt_sz as f32,
    };
    let mut event_y_pos = status_h + border_px + border_px;

    for (c_date, c_info) in calendar {
        let is_holiday = c_info.holiday.is_some();
        let c_date_cln = c_date.to_calendar_date();
        let mut date_txt = format! {
            "{} {} {}",
            c_date_cln.0, month_abbr(c_date_cln.1), c_date_cln.2
        };

        if let Some(hld_txt) = c_info.holiday {
            date_txt.push_str(&format! {"—{hld_txt}"});
            date_txt = substr_th(date_txt, 32);
        }

        // Date header
        let date_box_l = left_box_w + border_px;
        let date_box_h = (event_fnt_sz as f32 * 1.5) as u32;
        drawing::draw_filled_rect_mut(
            &mut image,
            Rect::at(date_box_l as i32, event_y_pos as i32)
                .of_size(img_w - border_px - date_box_l, date_box_h),
            match is_holiday {
                true => red,
                false => black,
            },
        );
        drawing::draw_text_mut(
            &mut image,
            white,
            (date_box_l + border_px) as i32,
            (event_y_pos + ((event_fnt_sz as f32 * 0.5) / 2.0) as u32) as i32,
            event_fnt_scale,
            &font_chakra_b,
            &date_txt,
        );
        event_y_pos += date_box_h;

        if event_y_pos > (last_update_y - (border_px * 2)) {
            break;
        }

        for (_uid, event) in c_info.events {
            let event_name = substr_th(event.name, 32);

            // Draw box for better visibility on ePaper
            drawing::draw_filled_rect_mut(
                &mut image,
                Rect::at(date_box_l as i32, event_y_pos as i32)
                    .of_size(img_w - border_px - date_box_l, date_box_h),
                gray,
            );
            drawing::draw_text_mut(
                &mut image,
                match is_holiday {
                    true => red,
                    false => black,
                },
                (date_box_l + border_px) as i32,
                (event_y_pos + (border_px / 2)) as i32,
                event_fnt_scale,
                &font_chakra_r,
                &format! {"{:02}:{:02}", event.time.hour(), event.time.minute()},
            );
            drawing::draw_text_mut(
                &mut image,
                match is_holiday {
                    true => red,
                    false => black,
                },
                (date_box_l + border_px + ((event_fnt_scale.x * 2.5) as u32)) as i32,
                (event_y_pos + (border_px / 2)) as i32,
                event_fnt_scale,
                // _r is too slim when render
                &font_chakra_sb,
                &event_name,
            );
            event_y_pos += event_fnt_sz + (border_px / 2);

            if event_y_pos > (last_update_y - (border_px * 2)) {
                break;
            }
        }
    }

    // Last update
    // Draw box for better visibility on ePaper
    let last_upd_at_t = last_update_y - (border_px / 2);
    drawing::draw_filled_rect_mut(
        &mut image,
        Rect::at(0_i32, last_upd_at_t as i32).of_size(img_w, img_h - last_upd_at_t),
        gray,
    );
    drawing::draw_text_mut(
        &mut image,
        black,
        border_px as i32,
        last_update_y as i32,
        PxScale { x: 12.0, y: 12.0 },
        &font_chakra_sb,
        &format! {"Last update: {}", time_local.replace_nanosecond(0).unwrap_or(time_local)},
    );

    // Adjust contrast
    contrast_in_place(&mut image, 200.0);

    // Save the response
    let mut img_buf = BufWriter::new(Cursor::new(Vec::new()));

    if let Err(e) = match q.output {
        OutputEnum::Full => image.write_to(&mut img_buf, ImageFormat::Png),
        // Paint black only black. Otherwise White
        OutputEnum::Black | OutputEnum::BlackInvert => {
            let mut bw_img = map_pixels(&image, |_x, _y, p| {
                let red = p.channels()[0];
                let blue = p.channels()[2];
                let target_luma = red | blue;
                Luma([if q.output == OutputEnum::BlackInvert {
                    !target_luma
                } else {
                    target_luma
                }])
            });
            dither(&mut bw_img, &BiLevel);
            bw_img.write_to(&mut img_buf, ImageFormat::Png)
        }
        // Paint Red only Red. Otherwise transparent
        OutputEnum::Red => {
            let mut bw_img = map_pixels(&image, |_x, _y, p| {
                let red = p.channels()[0];
                let blue = p.channels()[2];
                Luma([(red & (!blue))])
            });
            dither(&mut bw_img, &BiLevel);
            map_pixels(&bw_img, |_x, _y, p| {
                let l = p.channels()[0];
                Rgba([l, 0_u8, 0_u8, l])
            })
            .write_to(&mut img_buf, ImageFormat::Png)
        }
    } {
        return ApiError::InternalError(e.into()).into_response();
    }

    let img_vec = match img_buf.into_inner().map(|ib| ib.into_inner()) {
        Err(e) => {
            return ApiError::InternalError(e.into()).into_response();
        }
        Ok(img_vec) => img_vec,
    };

    let mut res = img_vec.into_response();
    res.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("image/png"),
    );
    res
}
