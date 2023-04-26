use crate::exercise::{Exercise, ExerciseList};
use crate::project::RustAnalyzerProject;
use crate::run::{reset, run};
use crate::verify::verify;
use argh::FromArgs;
use console::Emoji;
use notify::DebouncedEvent;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::ffi::OsStr;
use std::fs;
use std::io::{self, prelude::*};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[macro_use]
mod ui;
mod exercise;
mod project;
mod run;
mod starklings_runner;
mod starklings_tester;
mod verify;

// In sync with crate version
const VERSION: &str = "5.3.0";

#[derive(FromArgs, PartialEq, Debug)]
/// starklings is a collection of small exercises to get you used to writing and reading Rust code
struct Args {
    /// show outputs from the test exercises
    #[argh(switch)]
    nocapture: bool,
    /// show the executable version
    #[argh(switch, short = 'v')]
    version: bool,
    #[argh(subcommand)]
    nested: Option<Subcommands>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Subcommands {
    Verify(VerifyArgs),
    Watch(WatchArgs),
    CompileSolutions(CompileSolutionsArgs),
    Run(RunArgs),
    Reset(ResetArgs),
    Hint(HintArgs),
    List(ListArgs),
    Paths(PathsArgs),
    Lsp(LspArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "verify")]
/// Verifies all exercises according to the recommended order
struct VerifyArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "watch")]
/// Reruns `verify` when files were edited
struct WatchArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "compile_solutions")]
/// Reruns `verify` when files were edited
struct CompileSolutionsArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "run")]
/// Runs/Tests a single exercise
struct RunArgs {
    #[argh(positional)]
    /// the name of the exercise
    name: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "reset")]
/// Resets a single exercise using "git stash -- <filename>"
struct ResetArgs {
    #[argh(positional)]
    /// the name of the exercise
    name: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "hint")]
/// Returns a hint for the given exercise
struct HintArgs {
    #[argh(positional)]
    /// the name of the exercise
    name: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "lsp")]
/// Enable rust-analyzer for exercises
struct LspArgs {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "list")]
/// Lists the exercises available in starklings
struct ListArgs {
    #[argh(switch, short = 'p')]
    /// show only the paths of the exercises
    paths: bool,
    #[argh(switch, short = 'n')]
    /// show only the names of the exercises
    names: bool,
    #[argh(option, short = 'f')]
    /// provide a string to match exercise names
    /// comma separated patterns are acceptable
    filter: Option<String>,
    #[argh(switch, short = 'u')]
    /// display only exercises not yet solved
    unsolved: bool,
    #[argh(switch, short = 's')]
    /// display only exercises that have been solved
    solved: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "paths")]
/// Enable rust-analyzer for exercises
struct PathsArgs {}

fn main() {
    let args: Args = argh::from_env();

    if args.version {
        println!("v{VERSION}");
        std::process::exit(0);
    }

    if args.nested.is_none() {
        println!("\n{WELCOME}\n");
    }

    if !Path::new("info.toml").exists() {
        println!(
            "{} debe ejecutarse desde el directorio starklings",
            std::env::current_exe().unwrap().to_str().unwrap()
        );
        println!("Try `cd starklings/`!");
        std::process::exit(1);
    }

    if !rustc_exists() {
        println!("No podemos encontrar `rustc`.");
        println!("Pruebe a ejecutar `rustc --version` para diagnosticar su problema.");
        println!("Para obtener instrucciones sobre cómo instalar Rust, consulte el README.");
        std::process::exit(1);
    }

    let toml_str = &fs::read_to_string("info.toml").unwrap();
    let mut exercises = toml::from_str::<ExerciseList>(toml_str).unwrap().exercises;
    let command = args.nested.unwrap_or_else(|| {
        println!("{DEFAULT_OUT}\n");
        std::process::exit(0);
    });
    match command {
        Subcommands::List(subargs) => {
            if !subargs.paths && !subargs.names {
                println!("{:<17}\t{:<46}\t{:<7}", "Name", "Path", "Status");
            }
            let mut exercises_done: u16 = 0;
            let filters = subargs.filter.clone().unwrap_or_default().to_lowercase();
            exercises.iter().for_each(|e| {
                let fname = format!("{}", e.path.display());
                let filter_cond = filters
                    .split(',')
                    .filter(|f| !f.trim().is_empty())
                    .any(|f| e.name.contains(f) || fname.contains(f));
                let status = if e.looks_done() {
                    exercises_done += 1;
                    "Hecho"
                } else {
                    "Pendiente"
                };
                let solve_cond = {
                    (e.looks_done() && subargs.solved)
                        || (!e.looks_done() && subargs.unsolved)
                        || (!subargs.solved && !subargs.unsolved)
                };
                if solve_cond && (filter_cond || subargs.filter.is_none()) {
                    let line = if subargs.paths {
                        format!("{fname}\n")
                    } else if subargs.names {
                        format!("{}\n", e.name)
                    } else {
                        format!("{:<17}\t{fname:<46}\t{status:<7}\n", e.name)
                    };
                    // Somehow using println! leads to the binary panicking
                    // when its output is piped.
                    // So, we're handling a Broken Pipe error and exiting with 0 anyway
                    let stdout = std::io::stdout();
                    {
                        let mut handle = stdout.lock();
                        handle.write_all(line.as_bytes()).unwrap_or_else(|e| {
                            match e.kind() {
                                std::io::ErrorKind::BrokenPipe => std::process::exit(0),
                                _ => std::process::exit(1),
                            };
                        });
                    }
                }
            });
            let percentage_progress = exercises_done as f32 / exercises.len() as f32 * 100.0;
            println!(
                "Progreso: Has completado {} / {} ejercicios ({:.1} %).",
                exercises_done,
                exercises.len(),
                percentage_progress
            );
            std::process::exit(0);
        }

        Subcommands::Paths(_) => {
            exercises.iter().for_each(|e| {
                println!("{}", e.path.display());
            });
            std::process::exit(0);
        }

        Subcommands::Run(subargs) => {
            let exercise = find_exercise(&subargs.name, &exercises);

            run(exercise).unwrap_or_else(|_| std::process::exit(1));
        }

        Subcommands::Reset(subargs) => {
            let exercise = find_exercise(&subargs.name, &exercises);

            reset(exercise).unwrap_or_else(|_| std::process::exit(1));
        }

        Subcommands::Hint(subargs) => {
            let exercise = find_exercise(&subargs.name, &exercises);

            println!("{}", exercise.hint);
        }

        Subcommands::Verify(_subargs) => {
            verify(&exercises, (0, exercises.len())).unwrap_or_else(|_| std::process::exit(1));
        }

        Subcommands::Lsp(_subargs) => {
            let mut project = RustAnalyzerProject::new();
            project
                .get_sysroot_src()
                .expect("No se encuentra la ruta de la cadena de herramientas, ¿tiene instalado `rustc`?");
            project
                .exercises_to_json()
                .expect("No se pueden analizar los archivos de ejercicios de starklings");

            if project.crates.is_empty() {
                println!("No se ha encontrado ningún ejercicio, asegúrate de que estás en la carpeta `starklings`");
            } else if project.write_to_disk().is_err() {
                println!("Error al escribir rust-project.json en disco para rust-analyzer");
            } else {
                println!("Generado con éxito rust-project.json");
                println!("rust-analyzer analizará ahora los ejercicios, reinicie su servidor de idiomas o editor")
            }
        }

        Subcommands::CompileSolutions(_subargs) => {
            let exercises_base = PathBuf::from("exercises/");
            let solutions_base = PathBuf::from("solutions/");
            exercises.iter_mut().for_each(|mut ex| {
                ex.path = solutions_base
                    .clone()
                    .join(ex.path.strip_prefix(&exercises_base).unwrap());
            });
            match watch(&exercises) {
                Err(e) => {
                    println!("Error: {e:?}");
                    std::process::exit(1);
                }
                Ok(WatchStatus::Finished) => {
                    let emoji = Emoji("🎉", "★");
                    println!("{emoji} ¡Todas las soluciones compilan!{emoji}");
                }
                Ok(WatchStatus::Unfinished) => {
                    println!("Se detuvo la comprobación de soluciones.");
                }
            }
        }

        Subcommands::Watch(_subargs) => match watch(&exercises) {
            Err(e) => {
                println!(
                    "Error: No se pudo ver su progreso. El mensaje de error era {e:?}."
                );
                println!("Lo más probable es que te hayas quedado sin espacio en disco o que se haya alcanzado el `límite de inotify`.");
                std::process::exit(1);
            }
            Ok(WatchStatus::Finished) => {
                println!(
                    "{emoji} All exercises completed! {emoji}",
                    emoji = Emoji("🎉", "★")
                );
                println!("\n{FINISH_LINE}\n");
            }
            Ok(WatchStatus::Unfinished) => {
                println!("¡Esperamos que estés disfrutando aprendiendo sobre Rust!");
                println!("Si quieres continuar trabajando en los ejercicios más adelante, puedes simplemente ejecutar `starklings watch` de nuevo");
            }
        },
    }
}

fn spawn_watch_shell(
    failed_exercise_hint: &Arc<Mutex<Option<String>>>,
    should_quit: Arc<AtomicBool>,
) {
    let failed_exercise_hint = Arc::clone(failed_exercise_hint);
    println!("¡Bienvenido al modo watch! Puedes escribir 'help' para obtener una visión general de los comandos que puedes utilizar aquí.");
    thread::spawn(move || loop {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();
                if input == "hint" {
                    if let Some(hint) = &*failed_exercise_hint.lock().unwrap() {
                        println!("{hint}");
                    }
                } else if input == "clear" {
                    println!("\x1B[2J\x1B[1;1H");
                } else if input.eq("quit") {
                    should_quit.store(true, Ordering::SeqCst);
                    println!("Bye!");
                } else if input.eq("help") {
                    println!("Comandos disponibles en modo watch:");
                    println!("  hint  - imprime la pista del ejercicio actual");
                    println!("  clear - limpia la pantalla");
                    println!("  quit  - quita modo watch");
                    println!("  help  - muestra este mensaje de ayuda");
                    println!();
                    println!("El modo Watch reevalúa automáticamente el ejercicio en curso");
                    println!("cuando edite el contenido de un archivo.")
                } else {
                    println!("unknown command: {input}");
                }
            }
            Err(error) => println!("error leyendo comando: {error}"),
        }
    });
}

fn find_exercise<'a>(name: &str, exercises: &'a [Exercise]) -> &'a Exercise {
    if name.eq("siguiente") {
        exercises
            .iter()
            .find(|e| !e.looks_done())
            .unwrap_or_else(|| {
                println!("🎉 ¡Enhorabuena! ¡Has hecho todos los ejercicios!");
                println!("🔚 ¡No hay más ejercicios que hacer a continuación!");
                std::process::exit(1)
            })
    } else {
        exercises
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| {
                println!("No se encontró ningún ejercicio para '{name}'!");
                std::process::exit(1)
            })
    }
}

enum WatchStatus {
    Finished,
    Unfinished,
}

fn watch(exercises: &[Exercise]) -> notify::Result<WatchStatus> {
    /* Clears the terminal with an ANSI escape code.
    Works in UNIX and newer Windows terminals. */
    fn clear_screen() {
        println!("\x1Bc");
    }

    let (tx, rx) = channel();
    let should_quit = Arc::new(AtomicBool::new(false));

    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))?;
    watcher.watch(Path::new("./exercises"), RecursiveMode::Recursive)?;

    clear_screen();

    let to_owned_hint = |t: &Exercise| t.hint.to_owned();
    let failed_exercise_hint = match verify(exercises.iter(), (0, exercises.len())) {
        Ok(_) => return Ok(WatchStatus::Finished),
        Err(exercise) => Arc::new(Mutex::new(Some(to_owned_hint(exercise)))),
    };
    spawn_watch_shell(&failed_exercise_hint, Arc::clone(&should_quit));
    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => match event {
                DebouncedEvent::Create(b) | DebouncedEvent::Chmod(b) | DebouncedEvent::Write(b) => {
                    if b.extension() == Some(OsStr::new("cairo")) && b.exists() {
                        let filepath = b.as_path().canonicalize().unwrap();
                        let pending_exercises = exercises
                            .iter()
                            .find(|e| filepath.ends_with(&e.path))
                            .into_iter()
                            .chain(
                                exercises
                                    .iter()
                                    .filter(|e| !e.looks_done() && !filepath.ends_with(&e.path)),
                            );
                        let num_done = exercises.iter().filter(|e| e.looks_done()).count();
                        clear_screen();
                        match verify(pending_exercises, (num_done, exercises.len())) {
                            Ok(_) => return Ok(WatchStatus::Finished),
                            Err(exercise) => {
                                let mut failed_exercise_hint = failed_exercise_hint.lock().unwrap();
                                *failed_exercise_hint = Some(to_owned_hint(exercise));
                            }
                        }
                    }
                }
                _ => {}
            },
            Err(RecvTimeoutError::Timeout) => {
                // the timeout expired, just check the `should_quit` variable below then loop again
            }
            Err(e) => println!("watch error: {e:?}"),
        }
        // Check if we need to exit
        if should_quit.load(Ordering::SeqCst) {
            return Ok(WatchStatus::Unfinished);
        }
    }
}

fn rustc_exists() -> bool {
    Command::new("rustc")
        .args(["--version"])
        .stdout(Stdio::null())
        .spawn()
        .and_then(|mut child| child.wait())
        .map(|status| status.success())
        .unwrap_or(false)
}

const DEFAULT_OUT: &str = r#"Starklings - Un tutorial interactivo para aprender Cairo y Starknet

       _             _    _ _
      | |           | |  | (_)
   ___| |_ __ _ _ __| | _| |_ _ __   __ _ ___
  / __| __/ _` | '__| |/ / | | '_ \ / _` / __|
  \__ \ || (_| | |  |   <| | | | | | (_| \__ \
  |___/\__\__,_|_|  |_|\_\_|_|_| |_|\__, |___/
                                     __/ |
                                    |___/

¡Gracias por instalar starklings!

¿Es tu primera vez? ¡No te preocupes, starklings está hecho para principiantes! 
Te enseñaremos un montón de cosas sobre StarkNet y Cairo.

Así es como funciona starklings:

1. Para comenzar starklings ejecuta `cargo run --bin starklings watch`
2. Se iniciará automáticamente con el primer ejercicio. ¡No te confundas por 
los mensajes de error que aparecen tan pronto como ejecutes starklings! Esto es
parte del ejercicio que debes resolver, así que abre el archivo de ejercicio en 
un editor y comienza tu trabajo de detective.
3. Si estás atascado en un ejercicio, hay una pista útil que puedes ver 
escribiendo `hint` (en modo watch), o ejecutando `cargo run --bin starklings hint
 nombre_del_ejercicio`.
4. Cuando hayas resuelto el ejercicio con éxito, elimina el comentario
`// I AM NOT DONE` para pasar al siguiente ejercicio.
5. Si un ejercicio no tiene sentido para ti, ¡por favor abre un problema en GitHub!
(https://github.com/shramee/starklings-cairo1/issues/new).

¿Todo claro? ¡Genial! Para comenzar, ejecuta `starklings watch` para obtener el 
primer ejercicio. ¡Asegúrate de tener tu editor abierto!"#;

const FINISH_LINE: &str = r#"+----------------------------------------------------+
|         ¡Has llegado a la meta!                    |
+--------------------------  ------------------------+

                          
                                 @@@@@@@@@@@@@@&                                
                          #@@@@@@@@@@@@@@@@@@@@@@@@@@@/                         
                      @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@(                     
                   &@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@(                  
                 @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@                
               @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@              
             &@@@@@@@@@@@@ @@@@@@@@@@@@@@@@&               @@@@@@@@             
            @@@@@@@@@@@,     *@@@@@@@@@@                      @@@@@@&           
           @@@@@@@@@@@@@@   @@@@@@@@@@                         *@@@@@&          
          /@@@@@@@@@@@@@@@ @@@@@@@@@               .*********/@@@@@@@@          
          @@@@@@@@@@@@@@@@@@@@@@@@               **********@@@@@@@@@@@@         
          @@@@@@@@@@@@@@@@@@@@@@                *********@@@@@@@@@@@@@@         
          @@@@@@@@@@@@@@@@@@@@                *********@@@@@@@@@@@@@@@@         
          @@@@@@@@@@@@@@@@*                 ,********&@@@@@@@@@@@@@@@@@         
          @@@@@@                          *********/@@@@@@@@@@@@@@@@@@@         
          ,@@@@@@(                      *********/@@@@@@@@@@@@@@@@@@@@          
           @@@@@@@@/,               .**********@@@@@@@%**%@@@@@@@@@@@(          
            @@@@@@@@@@*****,,,**************@@@@@@@@&******%@@@@@@@@(           
             ,@@@@@@@@@@@@/************(@@@@@@@@@@@@@******@@@@@@@@             
               @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@&              
                 @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@                
                   /@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@                   
                      *@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@                      
                          .@@@@@@@@@@@@@@@@@@@@@@@@@@@                          
                                 .@@@@@@@@@@@@&                                 
                                                                                
                                                                                

Esperamos que hayas disfrutado aprendiendo sobre Cairo y Starknet.
Si has detectado algún problema, no dudes en notificarlo en nuestro repositorio.
https://github.com/shramee/starklings-cairo1/"#;

pub const WELCOME: &str = r#"Starklings - Un tutorial interactivo para aprender Cairo y Starknet

       _             _    _ _
      | |           | |  | (_)
   ___| |_ __ _ _ __| | _| |_ _ __   __ _ ___
  / __| __/ _` | '__| |/ / | | '_ \ / _` / __|
  \__ \ || (_| | |  |   <| | | | | | (_| \__ \
  |___/\__\__,_|_|  |_|\_\_|_|_| |_|\__, |___/
                                     __/ |
                                    |___/"#;
