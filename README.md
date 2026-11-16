# ServerUI

Administración visual de Ubuntu Server desde la terminal, construida en Rust.
**Stage 0 / v0.1.0: únicamente demostración con datos simulados.** La aplicación
no consulta métricas reales, ejecuta comandos del sistema ni modifica el host.

El proyecto está vinculado a [Charmandiox9/ServerUI](https://github.com/Charmandiox9/ServerUI).
La licencia de distribución está pendiente de decisión del propietario; no se
incluye una licencia por inferencia.

## Ejecutar

Instala Rust mediante rustup y las herramientas de compilación de tu plataforma
(linker de C en Linux; herramientas MSVC y Windows SDK en Windows). El proyecto
fija Rust 1.98.1; es la única versión mínima comprobada por ahora. Rustup instala
el toolchain declarado en `rust-toolchain.toml` cuando sea necesario.

```sh
cargo run --locked -- --demo
cargo run --locked -- --demo --ascii --no-color
cargo run --locked -- --demo --refresh-ms 500
cargo run --locked -- --demo --demo-scenario empty
cargo run --locked -- --demo --demo-scenario error
cargo run --locked -- --help
```

Se requieren stdin y stdout conectados a una terminal interactiva. Sin `--demo`,
el programa informa que el proveedor real está pendiente y termina sin activar
los modos del terminal. `--help` y `--version` funcionan con salida redirigida.

`--refresh-ms` admite 100–60000 ms; el valor predeterminado es 1000. Los escenarios
son deterministas: `normal` cambia muestras, `empty` no devuelve datos y `error`
produce una muestra válida, un fallo en la segunda consulta y recuperación en
la siguiente. `r` consulta inmediatamente; también se reintenta en cada tick.
Los valores de uptime simulado avanzan por consulta, no miden tiempo del host.

`--ascii` evita bordes Unicode. `--no-color` conserva los colores predeterminados
del terminal; también se respeta una variable `NO_COLOR` no vacía. Los estados
se identifican con texto, sin depender del color. La CLI usa texto sin color.

## Teclado y pantallas

| Tecla | Acción |
| --- | --- |
| Tab / derecha | Siguiente sección |
| Shift+Tab / izquierda | Sección anterior |
| 1–4 | Dashboard, Procesos, Servicios, Ajustes |
| ? / Escape | Abrir o cerrar ayuda / cerrar ayuda |
| r | Actualizar datos simulados o reintentar |
| q / Ctrl+C | Salir, también desde la ayuda |

El Dashboard muestra CPU, RAM, swap, disco y uptime **simulados**. Procesos y
Servicios muestran el stage previsto, sin listar datos reales ni ofrecer
acciones administrativas. Ajustes informa las opciones de esta ejecución y no
guarda configuración. La ayuda captura la navegación hasta cerrarla.

Diseño completo a 80×24, compacto a 40×12. En áreas menores aparece un aviso y
continúa disponible la salida por teclado. El resize se procesa por eventos.
No se requiere ratón, truecolor, fuente de iconos ni entorno gráfico.

Para SSH, compila o instala el binario Linux en el servidor de prueba y ejecútalo
dentro de la sesión SSH como usuario normal. ServerUI no implementa un cliente
SSH propio. La compatibilidad SSH real requiere el smoke documentado en
`docs/ACCEPTANCE_STAGE_0.md`; las pruebas de buffers no la certifican.

## Arquitectura

| Crate | Responsabilidad |
| --- | --- |
| `serverui-core` | Dominio, contrato SystemProvider, estado y transiciones puras |
| `serverui-providers` | MockProvider y escenarios repetibles |
| `serverui-tui` | Teclado, render y ciclo de vida del terminal |
| `serverui` | CLI, composición, reloj y loop de eventos |

Los proveedores y la TUI dependen del núcleo; el binario los compone. Renderizar
no realiza consultas. Las acciones producen efectos tipados que el runner
despacha al proveedor. El núcleo no conoce Ratatui, Crossterm ni APIs Linux.

El loop espera eventos hasta el próximo tick, limita el trabajo por iteración y
redibuja cuando cambia el estado o tamaño. Las consultas mock son inmediatas;
no se añade un runtime asíncrono. Antes de integrar un proveedor real se deberá
ejecutar I/O lento fuera del hilo de UI y revisar cancelación y límites de cola.
LinuxProvider y las operaciones administrativas pertenecen a etapas futuras.

## Desarrollo y pruebas

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
```

CI prepara estos checks en Linux y Windows con el toolchain fijado. Los tests
cubren transiciones, errores, CLI sin TTY, escenarios mock, avance del reloj ante
ráfagas de teclado y renderizado en tamaños extremos. No requieren red ni
privilegios una vez descargadas las dependencias. Cargo.lock se versiona.

Prueba PTY en Linux, con Python 3 estándar:

```sh
cargo build --locked -p serverui-tui --example terminal_probe
python3 scripts/pty_smoke.py target/release/serverui target/debug/examples/terminal_probe
```

El probe es un ejemplo separado para inducir errores/pánicos en un subproceso,
no una opción del binario de producción. El script comprueba modos termios,
pantalla alternativa y cursor tras salida normal, Ctrl+C, error y pánico.

En Unix se activa el sondeo de descriptores de Crossterm para evitar pérdida de
entrada concurrente con resize; el motivo y la regresión están registrados en
`docs/adr/0001-unix-event-polling.md`.

También se incluye un fixture SSH sobre Ubuntu 24.04 para CI. Construye el
release Linux antes de usarlo. En un shell Linux con Docker:

```sh
docker build --tag serverui-ssh-smoke:local - < scripts/Dockerfile.ubuntu-smoke
docker run --rm --network none --cap-drop ALL \
  --cap-add SETUID --cap-add SETGID --cap-add SYS_CHROOT \
  --cap-add CHOWN --cap-add DAC_OVERRIDE --cap-add KILL --cap-add AUDIT_WRITE \
  --security-opt no-new-privileges \
  --mount "type=bind,source=$PWD,target=/workspace,readonly" \
  serverui-ssh-smoke:local python3 /workspace/scripts/ssh_smoke.py /workspace/target/release/serverui
```

El fixture crea claves efímeras y un servidor de prueba dentro del contenedor;
la aplicación remota corre como usuario normal. No publica puertos, accede a la
red externa, monta sockets ni configura SSH en el host. Las capacidades
declaradas son para el sshd del fixture, nunca para ServerUI. La construcción
de la imagen sí descarga paquetes de Ubuntu. No sustituye una VM ni pruebas
con otros clientes y emuladores.

Para medir un release Linux fuera del compilador:

```sh
python3 scripts/benchmark_demo.py target/release/serverui --seconds 600
```

El harness informa RSS en bytes, CPU como porcentaje de un núcleo, primer frame
y respuesta de ayuda en un PTY local. Sus objetivos son controles de desarrollo,
no garantías para cualquier hardware o conexión SSH.

El terminal se restaura mediante guard y hook de pánico antes de imprimir
diagnósticos. No es posible garantizar limpieza tras SIGKILL, `panic=abort`,
fallo del emulador o desconexión abrupta. Si un proceso es terminado así y el
shell queda en raw mode, usar `stty sane` y, si hace falta, `reset` en Linux.

Leer `AGENTS.md` antes de contribuir. El alcance y las decisiones están en
`docs/STAGE_0_PLAN.md`; la evidencia y validaciones pendientes están en
`docs/ACCEPTANCE_STAGE_0.md`. No se ha publicado una release ni autorizado Stage 1.
