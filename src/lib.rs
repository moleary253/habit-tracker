use chrono::{Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
                for (date, prog) in &self.progress {
                    result = result + format!("\n\t{}: {}", date, prog).as_str();
                }
                result
            }
        }
    }

    pub fn add_progress(&mut self, progress: i32) {
        let entry = self
            .progress
            .entry(
                Local::now()
                    .checked_sub_signed(Duration::hours(2))
                    .unwrap()
                    .date_naive(),
            )
            .or_insert(0);
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
                if !((*self.progress.entry(Local::now().date_naive()).or_default() & flag_to_set
                    != 0)
                    ^ (finished))
                {
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
}
