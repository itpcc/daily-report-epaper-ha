use crate::{
    api_error::ApiError,
    model::{CalendarMapArc, DateInfo, DateInfoEventMode, WeatherInfo, WeatherInfoArc},
    Config,
};
use ical::parser::Component;
use itertools::Itertools;
use std::io::Cursor;
use time::{format_description::well_known::Iso8601, Date, PrimitiveDateTime};
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt};
use tokio::task::JoinSet;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

async fn fetch_holiday(cfg: Config, calendar: CalendarMapArc) -> Result<(), ApiError> {
    let res = reqwest::Client::new()
        .get(cfg.ical_holiday.clone())
        .send()
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?
        .bytes()
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?;

    let holiday_icals = ical::IcalParser::new(Cursor::new(res))
        .flat_map(|cr| {
            let Ok(c) = cr else {
                return None;
            };

            Some(c.events)
        })
        .collect_vec()
        .into_iter()
        .flatten()
        .collect_vec();

    let mut calendar = calendar.write().await;

    holiday_icals.into_iter().for_each(|evnt| {
        let (Some(dtstart), Some(summary)) = (
            evnt.get_property("DTSTART")
                .and_then(|p| p.value.to_owned()),
            evnt.get_property("SUMMARY")
                .and_then(|p| p.value.to_owned()),
        ) else {
            return;
        };

        let Ok(date) = Date::parse(&dtstart, &Iso8601::DATE) else {
            return;
        };

        let c_nty = calendar.entry(date).or_insert_with(|| DateInfo {
            date,
            holiday: Default::default(),
            events: Default::default(),
        });
        c_nty.holiday.replace(summary);
    });

    Ok(())
}

async fn fetch_event(cfg: Config, calendar: CalendarMapArc) -> Result<(), ApiError> {
    let res = reqwest::Client::new()
        .get(cfg.ical_event.clone())
        .send()
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?
        .bytes()
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?;

    let event_icals = ical::IcalParser::new(Cursor::new(res))
        .flat_map(|cr| {
            let Ok(c) = cr else {
                return None;
            };

            Some(c.events)
        })
        .collect_vec()
        .into_iter()
        .flatten()
        .collect_vec();

    let mut calendar = calendar.write().await;

    event_icals.into_iter().for_each(|evnt| {
        let (Some(dtstart), Some(summary), Some(uid)) = (
            evnt.get_property("DTSTART"),
            evnt.get_property("SUMMARY")
                .and_then(|p| p.value.to_owned()),
            evnt.get_property("UID").and_then(|p| p.value.to_owned()),
        ) else {
            return;
        };

        let Some(destart_val) = dtstart.value.as_ref() else {
            return;
        };

        let dstart_tzid = dtstart
            .params
            .as_ref()
            .and_then(|prm| {
                prm.iter().find_map(|(p_name, p_vals)| {
                    if p_name != "TZID" {
                        return None;
                    }

                    p_vals.first()
                })
            })
            .unwrap_or(&cfg.tz);
        let tz = timezones::get_by_name(dstart_tzid).unwrap_or(timezones::db::UTC);
        let Ok(dtstart_pdt) = PrimitiveDateTime::parse(destart_val, &Iso8601::DATE) else {
            return;
        };
        let dtstart_odt = match dtstart_pdt.assume_timezone(tz) {
            OffsetResult::Some(t) => t,
            OffsetResult::Ambiguous(t, _) => t,
            OffsetResult::None => {
                return;
            }
        };
        let dstart_date = dtstart_odt.date();

        let c_nty = calendar.entry(dstart_date).or_insert_with(|| DateInfo {
            date: dstart_date,
            holiday: Default::default(),
            events: Default::default(),
        });
        c_nty.events.insert(
            uid,
            DateInfoEventMode {
                time: dtstart_odt,
                name: summary,
            },
        );
    });

    Ok(())
}

async fn fetch_weather(cfg: Config, weather: WeatherInfoArc) -> Result<(), ApiError> {
    let res = reqwest::Client::new()
        .get(format! {"{}/api/states/weather.forecast_home", cfg.ha_url})
        .bearer_auth(cfg.ha_token.clone())
        .send()
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?
        .json::<WeatherInfo>()
        .await
        .map_err(|e| ApiError::InternalError(e.into()))?;

    weather.write().await.replace(res);

    Ok(())
}

async fn fetch(
    cfg: Config,
    calendar: CalendarMapArc,
    weather: WeatherInfoArc,
) -> Result<(), ApiError> {
    let mut set = JoinSet::new();

    set.spawn(fetch_holiday(cfg.clone(), calendar.clone()));
    set.spawn(fetch_event(cfg.clone(), calendar.clone()));
    set.spawn(fetch_weather(cfg.clone(), weather.clone()));

    while let Some(res) = set.join_next().await {
        if let Ok(Err(e)) = res {
            return Err(e);
        }
    }

    Ok(())
}

pub async fn setup(
    cfg: Config,
    calendar: CalendarMapArc,
    weather: WeatherInfoArc,
) -> Result<JobScheduler, JobSchedulerError> {
    let mut sched = JobScheduler::new().await?;

    // Run init job
    if let Err(e) = fetch(cfg.clone(), calendar.clone(), weather.clone()).await {
        tracing::error!("Cron init: Unable to fetch holiday: {:?}", e);
    }
    tracing::info!("Cron init: Run success");

    // Add async job
    sched
        .add(Job::new_async("0 */5 * * * *", move |_uuid, _l| {
            let clnd = calendar.clone();
            let wth = weather.clone();
            let cfg_c = cfg.clone();
            Box::pin(async move {
                tracing::debug!("Cron job: start");
                if let Err(e) = fetch(cfg_c, clnd.clone(), wth.clone()).await {
                    tracing::error!("Cron init: Unable to fetch holiday: {:?}", e);
                }
                tracing::info!("Cron job: Run success");
            })
        })?)
        .await?;

    // Feature 'signal' must be enabled
    sched.shutdown_on_ctrl_c();

    // Add code to be run during/after shutdown
    sched.set_shutdown_handler(Box::new(|| {
        Box::pin(async move {
            tracing::info!("Cron Shut down done");
        })
    }));

    Ok(sched)
}
