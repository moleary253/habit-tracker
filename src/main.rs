use chrono::Duration;
use habit_tracker::{Habit, HabitType};
use plotters::prelude::*;
use std::env::args;
use std::fs;
use std::io::{Error, ErrorKind};

fn io_error(text: &str) -> Result<(), Box<dyn std::error::Error + 'static>> {
    Err(Box::new(Error::new(ErrorKind::Other, text)))
}

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let database_file = "data.json";
    let file_data = match fs::read_to_string(database_file) {
        Err(_) => {
            fs::write(database_file, serde_json::to_string(&Vec::<Habit>::new())?)?;
            fs::read_to_string(database_file)?
        }
        Ok(data) => data,
    };
    let mut habits: Vec<Habit> = serde_json::from_str(&file_data)?;

    let args = Vec::from_iter(args());
    if args.len() < 2 {
        help();
        return Ok(());
    }

    match args[1].as_str() {
        "h" | "help" => {
            help();
        }
        "l" | "list" => {
            list(&habits);
        }
        "c" | "create" => {
            if args.len() < 3 {
                return io_error("Please enter the name of the habit you want to create.");
            }
            let name = &args[2];
            let habit_type = if args.len() > 3 {
                match args[3].as_str() {
                    "n" | "numerical" => HabitType::numerical(),
                    "c" | "checklist" => HabitType::checklist(Vec::from_iter(
                        args.iter().skip(4).map(|s| s.as_str()),
                    )),
                    kind => {
                        return io_error(
			    format!("'{}' is not a type of habit. Please enter n(umerical) or c(hecklist)", kind).as_str()
			);
                    }
                }
            } else {
                HabitType::numerical()
            };
            create(&mut habits, &name, habit_type);
        }
        "a" | "add" => {
            if args.len() < 3 {
                return io_error("Please enter the name of the habit you want to add to.");
            }
            let name = &args[2];
            let progress = if args.len() > 3 {
                i32::from_str_radix(&args[3], 10)?
            } else {
                1_i32
            };
            add(&mut habits, &name, progress)?;
        }
        command @ ("f" | "finish" | "unf" | "unfinish") => {
            let (finishing, command) = match command {
                "f" | "finish" => (true, "finish"),
                "unf" | "unfinish" => (false, "unfinish"),
                _ => panic!("Command didn't match second time when matched first time."),
            };
            if args.len() < 3 {
                return io_error(
                    format!(
                        "Please enter the name of the habit you want to {}.",
                        command
                    )
                    .as_str(),
                );
            }
            if args.len() < 4 {
                return io_error("Please enter the objective of the habit you want to change.");
            }
            let name = &args[2];
            let objective = &args[3];
            mark_objective(&mut habits, &name, &objective, finishing)?;
        }
        "p" | "plot" => {
            if args.len() < 3 {
                return io_error("Please enter the name of the habit you want to plot.");
            }
            let name = &args[2];
            let duration = if args.len() < 4 {
                Duration::days(7)
            } else {
                Duration::days(i64::from_str_radix(&args[3], 10)?)
            };
            plot(&habits, &name, duration)?;
        }
        command => {
            help();
            return io_error(format!("Didn't understand command '{}.'", command).as_str());
        }
    }

    fs::write(database_file, serde_json::to_string(&habits)?)?;

    Ok(())
}

fn help() {
    println!(
        "
habit_tracker:

    h(elp):
Print this message.

    l(ist):
List all habits in the database.

    c(reate) name [type] [objective 1]...:
Creates a habit. Type can be c(hecklist) or n(umerical). If a checklist habit, the objectives must be specified. Default numerical.

    a(dd) name [progress]:
Adds progress to a habit. Progress defaults to 1.

    f(inish) name objective:
Finishes an objective of a checklist habit.

    unf(inish) name objective:
Unfinishes an objective of a checklist habit.

    p(lot) name [days]:
Plots the progress of the habit over the past [days] days. Days defaults to 7. Saves the graph at graphs/[name].png
"
    )
}

fn list(habits: &Vec<Habit>) {
    for habit in habits {
        println!("{}", habit.display());
    }
}

fn create<'a>(habits: &mut Vec<Habit<'a>>, name: &'a str, habit_type: HabitType<'a>) {
    habits.push(Habit::new(name, habit_type));
}

fn add<'a>(
    habits: &mut Vec<Habit<'a>>,
    name: &'a str,
    progress: i32,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut iter = habits.iter().enumerate().filter(|(_, h)| h.name() == name);
    match iter.next() {
        None => io_error(format!("Habit {} doesn't seem to exist.", name).as_str()),
        Some((i, _)) => {
            habits[i].add_progress(progress);
            Ok(())
        }
    }
}

fn mark_objective<'a>(
    habits: &mut Vec<Habit<'a>>,
    name: &'a str,
    objective: &'a str,
    finished: bool,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    match habits.iter().position(|h| h.name() == name) {
        None => io_error(format!("Habit {} doesn't seem to exist.", name).as_str()),
        Some(i) => match habits[i].mark_objective(&objective, finished) {
            Err(e) => io_error(&e),
            _ => Ok(()),
        },
    }
}

fn plot<'a>(
    habits: &Vec<Habit<'a>>,
    name: &'a str,
    duration: Duration,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    match habits.iter().position(|h| h.name() == name) {
        None => io_error(format!("Habit {} doesn't seem to exist.", name).as_str()),
        Some(i) => {
            let file_name = format!("graphs/{}.png", name);
            let backend = BitMapBackend::new(file_name.as_str(), (1600, 900));
            let root = backend.into_drawing_area();
            root.fill(&WHITE)?;

            let root = root.margin(10, 10, 10, 10);

            habits[i].plot(&root, duration)?;

            root.present()?;

            Ok(())
        }
    }
}
