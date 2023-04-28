# STARKLINGS

### Un tutorial interactivo para ponerte en marcha con Cairo y Starknet

<p align="right">
<a href="https://discord.gg/onlydust">
<img src="https://img.shields.io/badge/Discord-6666FF?style=for-the-badge&logo=discord&logoColor=white" />
</a>
<a href="https://twitter.com/intent/follow?screen_name=onlydust_xyz">
<img src="https://img.shields.io/badge/Twitter-1DA1F2?style=for-the-badge&logo=twitter&logoColor=white" />
</a>
</p>

---

## Instalación y ejecución

Asegúrate de que tienes Rust y Cargo instalados con la cadena de herramientas `default`.  
Con rustup `curl https://sh.rustup.rs -sSf | sh -s`

1. Clona el repositorio y entra en el directorio,  
   `git clone https://github.com/shramee/starklings-cairo1.git && cd starklings-cairo1`.
2. Ejecuta `cargo run --bin starklings`, esto puede tardar un poco la primera vez.
3. Deberías ver este mensaje de introducción, ¡ejecuta `cargo run --bin starklings watch` cuando estés listo!

```
Starklings - Un tutorial interactivo para aprender Cairo y Starknet

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
primer ejercicio. ¡Asegúrate de tener tu editor abierto!
```

## Inspiración

-   [Rustlings](https://github.com/rust-lang/rustlings), starklings is forked from Rustlings. Thanks to all the original [authors and contributors](https://github.com/rust-lang/rustlings)

## Testing

#### Para los tests relacionados con Cairo

```
cargo test cairo
```

#### Para todos los test

```
cargo test
```

## Contribución

Gracias por tu interés en el proyecto. Puedes hacer un fork del repositorio, crear una rama con un nombre descriptivo (quizás el número de incidencia y una o dos palabras para describirla) y enviar un pull request a la rama `dev` de este repositorio.

### Branches o Ramas

Tenemos 2 ramas activas,

1. `dev` Aquí es donde ocurre el nuevo desarrollo. Todos los pull requests deben hacerse a esta rama.
2. `main` Esta es para clonar y ejecutar starklings. `dev` se fusiona en `main` después de un segundo conjunto de pruebas.

### Añadir nuevos ejercicios

1. Los nuevos ejercicios se pueden añadir en el directorio `./exercises`.
2. Inserte información sobre el ejercicio en el archivo `./info.toml`. Por ejemplo:
    ```toml
    [[exercises]]
    name = "new_exercise"
    path = "exercises/new_module/new_exercise.cairo"
    mode = "compile" # or "test"
    hint = """"""
    ```
3. Comprueba que los [test](#testing) pasan.
4. Envía tu PR a la rama `dev` del repositorio.

### Actualización de la lógica de Rust/versión de Cairo

1. [Test](#testing) de tus cambios.
2. Asegúrate de tener las soluciones a todos los ejercicios en el directorio `./solutions`.
3. Ejecute `cargo run --bin starklings compile_solutions` para confirmar que todas las soluciones de los ejercicios siguen compilando.
4. Haz un pull request a la rama `dev` del repositorio.

### Fusión de `dev` en `main` (mantenedores)

1. Crea un PR de la rama `dev` a la rama `master`.
2. Ejecutar todas las pruebas, y comprobar las soluciones con `cargo run --bin starklings compile_solutions`.
3. Compruebe que no se han fusionado nuevos cambios en `dev` desde que se creó el PR.
4. Si todo tiene sentido, ¡fusione!
