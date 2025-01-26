// src/tasks/scheduler.rs
use chrono::{Local, NaiveTime, Datelike, Timelike};
use log::{debug, info, warn};
use crate::config::Task;
use crate::tracker::task_execution_tracker::TaskExecutionTracker;

pub(crate) fn should_execute_task(task: &Task, tracker: &TaskExecutionTracker) -> bool {
    let now = Local::now();
    let current_time = NaiveTime::from_hms_opt(
        now.hour(),
        now.minute(),
        0
    ).unwrap();

    debug!("Evaluating task: {}, current time: {:?}, current weekday: {:?}", 
        task.task_id, current_time, now.weekday());

    let task_time = match &task.start_time {
        Some(time_str) => NaiveTime::parse_from_str(time_str, "%H:%M")
            .unwrap_or_else(|e| {
                warn!("Failed to parse start time for task {}: {}, using default 09:00", task.task_id, e);
                NaiveTime::from_hms_opt(9, 0, 0).unwrap()
            }),
        None => {
            debug!("No start time provided for task {}, using default 09:00", task.task_id);
            NaiveTime::from_hms_opt(9, 0, 0).unwrap()
        }
    };

    // For repeat tasks, calculate minutes since start of day
    let minutes_since_start = |start: NaiveTime, current: NaiveTime| -> i64 {
        let start_mins = start.hour() * 60 + start.minute();
        let current_mins = current.hour() * 60 + current.minute();
        current_mins as i64 - start_mins as i64
    };

    let should_execute = match task.schedule_type.as_ref().map(|s| s.as_str()) {
        Some("DAILY_ONCE") => {
            let result = !tracker.was_executed_today(&task.task_id) && current_time >= task_time;
            debug!("DAILY_ONCE: Task {}, was_executed_today: {}, current_time >= task_time: {}, result: {}", task.task_id, !tracker.was_executed_today(&task.task_id), current_time >= task_time, result);
            result
        }
        Some("DAILY_REPEAT") => {
            let interval = task.interval.unwrap_or(0) as i64;
            if interval == 0 {
                debug!("DAILY_REPEAT: Task {}: interval is 0, skipping", task.task_id);
                false
            } else {
                let mins_since_start = minutes_since_start(task_time, current_time);
                let result = mins_since_start >= 0 && mins_since_start % interval == 0;
                debug!("DAILY_REPEAT: Task {}, mins_since_start: {}, interval: {}, result: {}", task.task_id, mins_since_start, interval, result);
                result
            }
        }
        Some("WEEKLY_ONCE") => {
            let current_day = now.weekday().to_string().to_uppercase();
            let day_matches = task.day_of_week.as_ref().map_or(false, |day| day.to_uppercase() == current_day);
            let result = !tracker.was_executed_today(&task.task_id) && day_matches && current_time >= task_time;
            debug!("WEEKLY_ONCE: Task {}, current_day: {}, task_day: {:?}, day_matches: {}, current_time >= task_time: {}, result: {}", task.task_id, current_day, task.day_of_week, day_matches, current_time >= task_time, result);
            result
        }
        Some("WEEKLY_REPEAT") => {
            let current_day = now.weekday().to_string().to_uppercase();
            let day_matches = task.day_of_week.as_ref().map_or(false, |day| day.to_uppercase() == current_day);
            let interval = task.interval.unwrap_or(0) as i64;
            if !day_matches {
                debug!("WEEKLY_REPEAT: Task {}, current_day: {}, task_day: {:?}, day_matches: false, skipping", task.task_id, current_day, task.day_of_week);
                false
            } else if interval == 0 {
                debug!("WEEKLY_REPEAT: Task {}: interval is 0, skipping", task.task_id);
                false
            } else {
                let mins_since_start = minutes_since_start(task_time, current_time);
                let result = mins_since_start >= 0 && mins_since_start % interval <= 1;
                debug!("WEEKLY_REPEAT: Task {}, mins_since_start: {}, interval: {}, result: {}", task.task_id, mins_since_start, interval, result);
                result
            }
        }
        Some("DATE_ONCE") => {
            match &task.date {
                Some(date_str) => {
                    let task_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
                    let result = !tracker.was_executed_today(&task.task_id) && now.date_naive().eq(&task_date) && current_time >= task_time;
                    debug!("DATE_ONCE: Task {}, task_date: {}, current_date: {}, current_time >= task_time: {}, result: {}", task.task_id, task_date, now.date_naive(), current_time >= task_time, result);
                    result
                },
                None => {
                    warn!("DATE_ONCE: Task {} has no date specified, skipping", task.task_id);
                    false
                }
            }
        }
        Some("DATE_REPEAT") => {
            match &task.date {
                Some(date_str) => {
                    let task_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
                    let interval = task.interval.unwrap_or(0) as i64;
                    if !now.date_naive().eq(&task_date) || interval == 0 {
                        debug!("DATE_REPEAT: Task {}, task_date: {}, current_date: {}, interval: {}, skipping", task.task_id, task_date, now.date_naive(), interval);
                        false
                    } else {
                        let mins_since_start = minutes_since_start(task_time, current_time);
                        let result = mins_since_start >= 0 && mins_since_start % interval <= 1;
                        debug!("DATE_REPEAT: Task {}, mins_since_start: {}, interval: {}, result: {}", task.task_id, mins_since_start, interval, result);
                        result
                    }
                },
                None => {
                    warn!("DATE_REPEAT: Task {} has no date specified, skipping", task.task_id);
                    false
                }
            }
        }
        _ => {
            debug!("Task {} has unknown or no schedule type, skipping", task.task_id);
            false
        }
    };

    if should_execute {
        info!("Task {} should be executed now", task.task_id);
    }
    should_execute
}