use crate::exercise::{Exercise, Mode, State};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;

// Verify that the provided container of Exercise objects
// can be compiled and run without any failures.
// Any such failures will be reported to the end user.
// If the Exercise being verified is a test, the verbose boolean
// determines whether or not the test harness outputs are displayed.
pub fn verify<'a>(
    exercises: impl IntoIterator<Item = &'a Exercise>,
    progress: (usize, usize),
) -> Result<(), &'a Exercise> {
    let (num_done, total) = progress;
    let bar = ProgressBar::new(total as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("Progreso: [{bar:60.green/red}] {pos}/{len} {msg}")
            .progress_chars("#>-"),
    );
    bar.set_position(num_done as u64);
    for exercise in exercises {
        let compile_result = match exercise.mode {
            Mode::Compile => compile_and_run_interactively(exercise),
            Mode::Test => compile_and_test_interactively(exercise),
        };
        if !compile_result.unwrap_or(false) {
            return Err(exercise);
        }
        let percentage = num_done as f32 / total as f32 * 100.0;
        bar.set_message(format!("({percentage:.1} %)"));
        bar.inc(1);
    }
    Ok(())
}

// Compile the given Exercise and run the resulting binary in an interactive mode
fn compile_and_run_interactively(exercise: &Exercise) -> Result<bool, ()> {
    let progress_bar = ProgressBar::new_spinner();
    progress_bar.enable_steady_tick(100);

    progress_bar.set_message(format!("Ejecutando {exercise}..."));

    let run_state = compile_and_run_cairo(exercise, &progress_bar)?;

    progress_bar.finish_and_clear();

    Ok(prompt_for_completion(exercise, Some(run_state)))
}

// Tests the given Exercise and run the resulting binary in an interactive mode
fn compile_and_test_interactively(exercise: &Exercise) -> Result<bool, ()> {
    let progress_bar = ProgressBar::new_spinner();
    progress_bar.enable_steady_tick(100);

    progress_bar.set_message(format!("Testeando {exercise}..."));

    let run_state = compile_and_test_cairo(exercise, &progress_bar)?;

    progress_bar.finish_and_clear();

    Ok(prompt_for_completion(exercise, Some(run_state)))
}

// Compile the given Exercise and return an object with information
// about the state of the compilation
fn compile_and_run_cairo<'a, 'b>(
    exercise: &'a Exercise,
    progress_bar: &'b ProgressBar,
) -> Result<String, ()> {
    let compilation_result = exercise.run_cairo();

    if let Some(error) = compilation_result.as_ref().err() {
        progress_bar.finish_and_clear();
        warn!(
            "Compilación de {} ¡Ha fallado! Por favor, inténtelo de nuevo. Aquí está el resultado:",
            exercise
        );
        println!("{error}");
        Err(())
    } else {
        Ok(compilation_result.unwrap())
    }
}

// Tests the given Exercise and return an object with information
// about the state of the tests
fn compile_and_test_cairo<'a, 'b>(
    exercise: &'a Exercise,
    progress_bar: &'b ProgressBar,
) -> Result<String, ()> {
    let compilation_result = exercise.test_cairo();

    if let Some(error) = compilation_result.as_ref().err() {
        progress_bar.finish_and_clear();
        warn!(
            "Testing de {} ¡Ha fallado! Por favor, inténtelo de nuevo. Aquí está el resultado:",
            exercise
        );
        println!("{error}");
        Err(())
    } else {
        Ok(compilation_result.unwrap())
    }
}

fn prompt_for_completion(exercise: &Exercise, prompt_output: Option<String>) -> bool {
    let context = match exercise.state() {
        State::Done => return true,
        State::Pending(context) => context,
    };

    match exercise.mode {
        Mode::Compile => success!("Ejecutado con éxito {}!", exercise),
        Mode::Test => success!("Testeado con éxito {}!", exercise),
        // Mode::Clippy => success!("Successfully compiled {}!", exercise),
    }

    let no_emoji = env::var("NO_EMOJI").is_ok();

    let _clippy_success_msg = "¡El código está compilando y Clippy está contento!";

    let success_msg = match exercise.mode {
        Mode::Compile => "¡El código se está compilando!",
        Mode::Test => "El código se está compilando, ¡y los test pasan!",
        // Mode::Clippy => clippy_success_msg,
    };

    println!();
    if no_emoji {
        println!("~*~ {success_msg} ~*~")
    } else {
        println!("🎉 🎉  {success_msg} 🎉 🎉")
    }
    println!();

    if let Some(output) = prompt_output {
        println!("Output:");
        println!("{}", separator());
        println!("{output}");
        println!("{}", separator());
        println!();
    }

    println!("Puedes seguir trabajando en este ejercicio,");
    println!(
        "o saltar al siguiente eliminando el comentario {}:",
        style("`I AM NOT DONE`").bold()
    );
    println!();
    for context_line in context {
        let formatted_line = if context_line.important {
            format!("{}", style(context_line.line).bold())
        } else {
            context_line.line.to_string()
        };

        println!(
            "{:>2} {}  {}",
            style(context_line.number).blue().bold(),
            style("|").blue(),
            formatted_line
        );
    }

    false
}

fn separator() -> console::StyledObject<&'static str> {
    style("====================").bold()
}
