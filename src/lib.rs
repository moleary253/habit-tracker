use chrono::{Duration, Local, NaiveDate};
use plotters::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod console_backend;

const DAY_OFFSET: Duration = Duration::hours(2);

fn today() -> NaiveDate {
    Local::now()
        .checked_sub_signed(DAY_OFFSET)
        .unwrap()
        .date_naive()
}

fn days_within_last(duration: Duration) -> impl Iterator<Item = NaiveDate> {
    Local::now()
        .checked_sub_signed(DAY_OFFSET)
        .unwrap()
        .checked_sub_signed(duration)
        .unwrap()
        .date_naive()
        .iter_days()
        .take_while(|date| date <= &today())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Habit<'a> {
    progress: HashMap<NaiveDate, i32>,
    name: &'a str,
    habit_type: HabitType<'a>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HabitType<'a> {
    Checklist {
        #[serde(borrow)]
        objectives: Vec<&'a str>,
    },
    Numerical,
}

impl<'a> HabitType<'a> {
    pub fn numerical() -> HabitType<'a> {
        HabitType::Numerical
    }

    pub fn checklist(objectives: Vec<&'a str>) -> HabitType<'a> {
        HabitType::Checklist { objectives }
    }
}

use HabitType as T;

impl<'a> Habit<'a> {
    pub fn new(name: &'a str, habit_type: HabitType<'a>) -> Habit<'a> {
        Habit::<'a> {
            progress: HashMap::new(),
            name,
            habit_type,
        }
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn display(&self) -> String {
        let mut result = String::new();
        result += format!("{}: ", &self.name).as_str();

        match &self.habit_type {
            T::Checklist { objectives } => {
                let mut times_completed = Vec::with_capacity(objectives.len());
                for _ in objectives {
                    times_completed.push(0);
                }

                for date in self.progress.keys() {
                    let mut prog = self.progress[date];
                    let mut i = 0;
                    while prog > 0 {
                        if prog & 1 == 1 {
                            times_completed[i] += 1;
                        }
                        prog = prog >> 1;
                        i += 1;
                    }
                }

                for (objective, count) in objectives.iter().zip(times_completed) {
                    result = result + format!("\n\t{}: {}", objective, count).as_str();
                }
                result
            }
            T::Numerical => {
                let mut dates = Vec::from_iter(self.progress.keys());
                dates.sort();
                for date in dates {
                    result = result
                        + format!("\n\t{}: {}", date, self.progress.get(date).unwrap()).as_str();
                }
                result
            }
        }
    }

    pub fn add_progress(&mut self, progress: i32) {
        let entry = self.progress.entry(today()).or_insert(0);
        *entry += progress;
    }

    pub fn mark_objective(&mut self, objective: &'a str, finished: bool) -> Result<(), String> {
        match &self.habit_type {
            T::Checklist { objectives } => {
                let mut i = 0;
                loop {
                    if i == objectives.len() {
                        return Err(format!(
                            "Objective '{}' does not exist in {}.",
                            objective, &self.name
                        ));
                    }
                    if objectives[i] == objective {
                        break;
                    }
                    i += 1;
                }
                let flag_to_set = 1 << i as i32;
                if !((*self.progress.entry(today()).or_default() & flag_to_set != 0) ^ (finished)) {
                    return Err(format!(
                        "Objective '{}' already marked as {}.",
                        objective,
                        if finished { "finished" } else { "unfinished" }
                    ));
                }

                self.add_progress(1 << i as i32 * (if finished { 1 } else { -1 }));
                Ok(())
            }
            _ => Err(format!("{} is not a checklist habit.", &self.name)),
        }
    }

    pub fn plotting_data(
        &self,
        in_last: Duration,
    ) -> Result<Vec<(i32, i32)>, Box<dyn std::error::Error + 'static>> {
        let days = Vec::from_iter(days_within_last(in_last));

        match &self.habit_type {
            T::Checklist { .. } => {
                let mut completed = Vec::with_capacity(days.len());
                for (i, day) in days.iter().enumerate() {
                    completed.push(0);

                    let mut prog = *self.progress.get(day).unwrap_or(&0);
                    while prog > 0 {
                        if prog & 1 == 1 {
                            completed[i] += 1;
                        }
                        prog = prog >> 1;
                    }
                }
                Ok(Vec::from_iter((1 - days.len() as i32..=0).zip(completed)))
            }
            T::Numerical => {
                let mut progress_during_period = Vec::<i32>::with_capacity(days.len());
                for day in days.iter() {
                    progress_during_period.push(*self.progress.get(day).unwrap_or(&0));
                }
                Ok(Vec::from_iter(
                    (1 - days.len() as i32..=0).zip(progress_during_period),
                ))
            }
        }
    }

    pub fn plot<DB: DrawingBackend>(
        &self,
        drawing_area: &DrawingArea<DB, plotters::coord::Shift>,
        in_last: Duration,
        cumulative: bool,
    ) -> Result<(), Box<dyn std::error::Error + 'static>>
    where
        DB::ErrorType: 'static,
    {
        let font = ("sans-serif", (10).percent_height());

        let data = self.plotting_data(in_last)?;
        let data = if cumulative {
            data.iter().fold(
                Vec::with_capacity(data.len()),
                |mut vec: Vec<(i32, i32)>, (d, p)| {
                    vec.push(match &vec.len() {
                        0 => (*d, *p),
                        _ => (*d, *p + vec[vec.len() - 1].1),
                    });
                    vec
                },
            )
        } else {
            data
        };

        let title = match (&self.habit_type, cumulative) {
            (T::Checklist { .. }, false) => "Num Goals Completed by day",
            (T::Numerical, false) => "Progress by day",
            (T::Checklist { .. }, true) => "Total Goals Completed by day",
            (T::Numerical, true) => "Total Progress by day",
        };

        let mut chart = ChartBuilder::on(drawing_area)
            .margin(1)
            .set_label_area_size(LabelAreaPosition::Left, (5i32).percent_width())
            .set_label_area_size(LabelAreaPosition::Bottom, (10i32).percent_height())
            .caption(title, font)
            .build_cartesian_2d(
                (1 - data.len() as i32)..0,
                0..*data.iter().map(|(_, y)| y).max().unwrap_or(&1),
            )?;

        chart.configure_mesh().disable_mesh().draw()?;

        chart.draw_series(
            data.iter()
                .map(|(x, y)| Pixel::new((*x, *y), RGBColor(50, 100, 50))),
        )?;
        Ok(())
    }
}
