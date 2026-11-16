# Aceptación de ServerUI Stage 0

Fecha de validación: 16 de noviembre de 2026.

Estado: Stage 0 completado y v0.1.0 publicada el 16 de noviembre de 2026. PR #1
integrado en 3a1f46764749b000c01b1a1e5920f8c7e8de8a32 y CI Linux/Windows aprobada
antes de publicar. Se mantiene aquí la evidencia histórica.

Release: https://github.com/Charmandiox9/ServerUI/releases/tag/v0.1.0
CI: https://github.com/Charmandiox9/ServerUI/actions/runs/43878781848

## Alcance entregado

- Cuatro crates con dominio/aplicación independientes de terminal y Linux.
- SystemProvider mínimo, MockProvider determinista y resultados tipados.
- Modo demo explícito, CLI validada y rechazo de ejecución sin TTY.
- Dashboard simulado; Procesos, Servicios y Ajustes provisionales.
- Teclado, ayuda, resize, modo compacto, ASCII y ausencia de color.
- Loop por eventos, refresco configurable y reintento sin bloqueo de navegación.
- Limpieza de terminal tras salida normal, inicialización parcial, error y pánico
  con unwinding; limitaciones de terminación abrupta documentadas.
- CI Linux/Windows, pruebas PTY y fixture SSH Ubuntu aislado.
- README, CHANGELOG, AGENTS y ADR de sondeo Unix.

## Entornos y evidencia

Host de desarrollo: Windows x86_64 con Rust/Cargo 1.98.1 (toolchain fijado).
Compilación Linux: contenedor oficial Rust 1.98.1 sobre Debian 13, x86_64, sin
privilegios de usuario root (UID 1000), capacidades descartadas y proyecto montado
solo para lectura. Target y caché Cargo temporales dentro del contenedor.

Imagen de compilación utilizada:
`rust@sha256:a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546`.
Kernel Docker/WSL2 5.15.167.4; CPU AMD Ryzen 5 7535HS, 12 procesadores lógicos.

Runtime SSH: fixture Ubuntu 24.04 (base fijada en
`scripts/Dockerfile.ubuntu-smoke`), OpenSSH 9.6p1, usuario remoto `serverui-test`
UID 1001. Claves efímeras, autenticación pública y StrictModes activo. Cliente y
servidor se comunican por loopback dentro de un contenedor `--network none`, sin
puertos publicados, sin socket Docker montado y sin `--privileged`. Las
capacidades explícitas del contenedor son únicamente para ejecutar sshd y
asignar el PTY; la TUI no solicita ni usa esas capacidades.

| Validación | Resultado |
| --- | --- |
| Formato, `cargo fmt --all -- --check` | Aprobado en Windows y Linux |
| Clippy, workspace/all-targets/locked/-D warnings | Aprobado en Windows y Linux |
| Tests Rust del workspace | 25 aprobados en Windows y 25 en Linux |
| Build release | Aprobado en Windows y Linux |
| `cargo run --locked -- --demo --ascii --no-color` | Arranque, navegación y ayuda observados en PTY Windows |
| Salida normal Windows | Código cero y salida de pantalla alternativa observados |
| Pánico Windows, ejemplo terminal_probe | Restauración antes del diagnóstico observada en PTY |
| Linux PTY, seis casos | Demo normal, vacío, error; probe normal, error y pánico aprobados |
| Linux cleanup real | termios idéntico antes/después; pantalla alternativa y cursor restaurados |
| SSH Ubuntu, dos sesiones | Demo ASCII sin color, resize y q; escenario error y Ctrl+C aprobados |
| Seguridad de demo | Revisión de fuentes: sin lector de métricas, comandos, red ni escrituras de aplicación |
| Grafo de dependencias | Revisado con cargo metadata; núcleo solo depende de thiserror |
| GitHub Actions remoto | Aprobado en PR #1 y main; run final 43878781848 |

Los tests Rust cubren contrato mock, vacío/error/recuperación, retención de la
última muestra, navegación y ayuda, teclado, fuente de eventos y reloj falsos,
ticks durante ráfagas, ausencia de redraw para estado vacío sin cambios, errores
de render/entrada, CLI sin TTY, tamaños 80×24, 40×12, 20×5, 1×1 y 0×0, texto de
proveedor no confiable y limpieza parcial/idempotente/con fallos/unwinding.

La regresión PTY encontró pérdida de entrada con resize y teclado simultáneos
en la fuente Mio de Crossterm 0.29.0. Cambiar a su feature Unix `use-dev-tty`
resolvió el caso original y los seis smoke tests; ver ADR 0001. Windows conserva
su backend nativo. La prueba de regresión permanece en CI.

En el sandbox Windows aparecieron avisos de caché incremental por permisos de
hard links entre ejecuciones. La verificación final se ejecutó con
`CARGO_INCREMENTAL=0` en ese proceso y pasó sin esos avisos; no se deshabilita la
caché incremental para los usuarios del repositorio.

El fixture SSH puede imprimir un aviso de logout/utmp porque no es un sistema
con login completo; las sesiones terminan con código cero y los asserts de
restauración pasan. No se deshabilitan las comprobaciones de autenticación para
evitar el aviso.

## Rendimiento

Medición del release Linux mediante `scripts/benchmark_demo.py`, con un PTY local
80×24, TERM=xterm-256color, ASCII, sin color y refresco de 1000 ms. Se mide el
proceso de aplicación, sin incluir compilador, servidor SSH, Python ni Docker.
Medición completada en 600,04 segundos. El harness terminó con código cero y
superó los cuatro umbrales propuestos en el plan:

| Medida | Resultado | Objetivo |
| --- | --- | --- |
| RSS a los 60 s | 2686976 bytes (2,69 MB / 2,5625 MiB) | Preferentemente < 30 MB |
| RSS mínimo / máximo / final | 2686976 / 2686976 / 2686976 bytes | Sin crecimiento sostenido observado |
| CPU en reposo, porcentaje de un núcleo | 0,0383 % | < 1 % propuesto |
| Primer frame desde lanzamiento del binario | 2,048 ms | < 1000 ms propuesto |
| Respuesta al abrir ayuda | 0,282 ms | < 100 ms propuesto |

Son resultados de una ejecución local con binario ya compilado; no son
percentiles de una campaña de benchmarks ni medidas de latencia SSH/WAN. Durante
la prueba se realizaron otras comprobaciones en el entorno, por lo que no se
presentan como mediciones de una máquina dedicada. No hubo crecimiento de RSS
en las muestras de un segundo tomadas durante esta ejecución.

Tamaños del binario release observado: Windows 840192 bytes; Linux 1022680 bytes.
No son presupuestos garantizados para toolchains o arquitecturas diferentes.

## Reproducir

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
cargo build --locked -p serverui-tui --example terminal_probe
```

En Linux, después del build:

```sh
python3 scripts/pty_smoke.py target/release/serverui target/debug/examples/terminal_probe
python3 scripts/benchmark_demo.py target/release/serverui --seconds 600
```

La construcción y ejecución del fixture SSH están documentadas en README y en
el workflow CI. El binario debe ser Linux; una compilación MSVC de Windows no
puede ejecutarse dentro del contenedor Ubuntu. El artefacto Linux local usado
en esta sesión está bajo `target/linux-validation/`, ignorado por Git.

## Límites y cierre

- No se ha probado una VM Ubuntu Server completa ni una conexión SSH entre
  máquinas distintas. El fixture verifica protocolo SSH real, teclado, tamaño y
  cleanup; no garantiza latencia WAN, todos los emuladores ni systemd.
- No hay métricas reales, operaciones privilegiadas, persistencia o módulos de
  stages posteriores. No corresponde validar APT, servicios, redes ni discos.
- La licencia open source sigue pendiente. El usuario autorizó publicar la
  release; README y notas aclaran que no se incluye un archivo de licencia.
- La protección remota de main no está configurada por esta tarea. El usuario
  autorizó publicar v0.1.0 después del flujo PR/CI; Stage 1 queda fuera.

Git local: commit inicial documental en main y commits de implementación en
`feat/stage-0`; `origin` es `https://github.com/Charmandiox9/ServerUI.git`. El remoto
estaba vacío antes de este trabajo. La release apunta al commit integrado del
PR y sus binarios provienen de CI para ese mismo commit, con checksums SHA-256.

El contenedor temporal de compilación/medición fue eliminado y no quedaron
contenedores en ejecución. Docker Desktop se inició para estas pruebas; su
cierre se solicitó al terminar. La CLI agotó el plazo de 30 s y reportó estado
`stopping`; no se forzó la terminación de procesos del host. Las imágenes de
desarrollo permanecen en la caché local para reutilización.
