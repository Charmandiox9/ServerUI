# Aceptación de ServerUI Stage 1

Fecha: 16 de noviembre de 2026. Estado: Stage 1 implementado y validado; el
usuario confirmó funcionamiento general en su laptop Ubuntu Server 22.04.5
x86_64 por SSH y autorizó commits, PR y release v0.2.0 con changelog. Publicar
desde main tras CI aprobada en el PR y el commit integrado. Stage 0 conserva
su evidencia y release v0.1.0.

## Entregables

- Dashboard Linux con CPU, RAM, swap, uptime y capacidad disponible por montaje.
- Procesos visibles de solo lectura: nombre/PID, orden CPU/RAM/PID/nombre,
  dirección reversible, selección estable, paginación y detalle.
- Identidad PID + inicio; desaparición y reutilización tratadas con aviso.
- MockProvider conserva aislamiento del host y añade procesos/discos ficticios.
- CPU pendiente hasta baseline válido; centésimas enteras y porcentaje de un
  núcleo para procesos. Ausencia y errores no se representan como ceros.
- Worker de recolección y lector de teclado independientes, canales acotados,
  peticiones coalescidas y snapshots compartidos mediante Arc.
- TUI/CLI con modo explícito, ASCII, ausencia de color, ayuda, resize y errores.
- CI ampliada con tests Linux reales, PTY/SSH en ambos modos y regresión de CPU.
- README, CHANGELOG, plan, ADR 0002 y convenciones actualizados.

## Verificaciones ejecutadas

Rust/Cargo 1.98.1. Windows x86_64 MSVC para demo; Linux x86_64 GNU en el contenedor
Rust 1.98.1/Debian 13 usado en Stage 0, ejecutado como UID 1000, sin capacidades,
con workspace de solo lectura y target/caché dentro del contenedor. Kernel WSL2
5.15.167.4; AMD Ryzen 5 7535HS, 12 procesadores lógicos.

| Comprobación | Resultado |
| --- | --- |
| cargo fmt --all -- --check | Aprobado |
| Clippy workspace/all-targets/locked/-D warnings | Aprobado Windows y Linux |
| cargo test --workspace --locked | 35 aprobados Windows; 39 Linux |
| cargo build --release --locked | Aprobado Windows y Linux |
| Demo Windows interactiva | Dashboard, filtro sshd, detalle PID 101 y salida limpia observados |
| Linux PTY | Nueve casos aprobados: demo, vacío, error, dos real, probe normal/error/pánico/background |
| SSH Ubuntu 24.04 | Cuatro sesiones aprobadas: dos demo y dos Linux real, UID 1001 |
| Benchmark corto demo corregido | Aprobado: CPU por debajo de 1 %, memoria y respuesta dentro de objetivos |
| Benchmark real corregido, 120 s | Aprobado; cifras debajo |
| GitHub Actions Stage 1 | Requerido en PR y main antes de publicar; runs enlazados en GitHub |

En Windows se utilizó CARGO_INCREMENTAL=0 por las restricciones de hard links del
sandbox, sin cambiar la configuración persistente del proyecto o del usuario.

Las 39 pruebas Linux incluyen tres integraciones que comparan memoria/swap con
/proc/meminfo, uptime con /proc/uptime y CPU global con deltas de /proc/stat.
La observación inicial registró CPU 0,66 % por ambos métodos, RAM total
7970410496 bytes y cinco montajes comparados con stat -f. Se tolera variación
entre lecturas dinámicas; no se exige igualdad de muestras tomadas en instantes
distintos. Se comprueba CPU inicial pendiente, CPU posterior disponible y
presencia del propio proceso, sin pretender contar procesos ocultos.

Una integración crea exclusivamente su propio sleep efímero, lo elimina y
comprueba que desaparece de snapshots posteriores. También verifica que no se
acumulen descriptores. Comandos externos y señales de cleanup existen solo en
fixtures de pruebas; el adaptador de producción no los ejecuta.

Los tests deterministas cubren lista de 10000 entradas, orden estable, filtro
insensible a mayúsculas por nombre/PID, límite de filtro, selección conservada,
PID reutilizado, montajes desplazables, snapshots compartidos, worker bloqueado,
1000 peticiones coalescidas y error/pánico de inicialización. Render cubre modos,
secciones, estados y tamaños 80×24, 40×12, 20×5, 1×1 y 0×0.

PTY conserva termios antes/después y comprueba cursor/pantalla alternativa.
El caso background-panic comprueba que el hilo de UI sigue dibujando antes de
restaurar la pantalla. SSH usa loopback dentro de Ubuntu 24.04 con claves
efímeras y StrictModes; sin red externa, puertos publicados, socket Docker ni
--privileged. La TUI remota se ejecuta como usuario normal y espera datos RAM
reales antes de enviar teclas. Las secuencias ejercitan filtro/inspección y resize.

## Medición y corrección de rendimiento

El primer ensayo detectó ~1,8 % de un núcleo tanto en demo como real. Un
reproductor sin estado/proveedor obtuvo 0,0666 % con Mio y 1,799 % con la fuente
Unix temporizada; confirmó el coste de su cola submilisegundo de poll. Mantener
la fuente que corrige SIGWINCH y usar read bloqueante en un hilo con canal
eliminó esa espera activa. Se retiró el reproductor temporal y se conservó una
regresión de benchmark en CI, junto con los tests de teclado/resize.

Mediciones corregidas con binario release, PTY local xterm-256color 80×24, ASCII,
sin color y refresco 1000 ms. Recursos del proceso de aplicación; no incluyen
compilación, Docker, Python ni servidor SSH.

| Medida | Demo, 15 s | Linux real, 120,02 s |
| --- | --- | --- |
| RSS mínimo/máximo/final | 2981888 bytes | 2990080 bytes |
| RSS máximo en MB / MiB | 2,98 / 2,84375 | 2,99 / 2,8515625 |
| CPU, porcentaje de un núcleo | 0,1333 % | 0,20 % |
| Primer frame | 2,166 ms | 2,056 ms |
| Primera muestra visible | 52,712 ms | 52,626 ms |
| Respuesta de ayuda | 0,522 ms | 0,531 ms |

No hubo crecimiento de RSS observado en esas ventanas. Son muestras locales con
pocos procesos, no percentiles ni garantías en hosts grandes, montajes lentos o
WAN. La prueba determinista con 10000 entradas valida comportamiento, no un
presupuesto de CPU/RAM para cualquier host con esa cantidad de procesos.
No se hizo una prueba de diez minutos para Stage 1; la medición histórica de
Stage 0 no se reutiliza como evidencia de este código.

Tamaño de release sin empaquetar: Windows 1018368 bytes; Linux 1360032 bytes.

## Reproducción y límites

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
cargo build --locked -p serverui-tui --example terminal_probe
python3 scripts/pty_smoke.py target/release/serverui target/debug/examples/terminal_probe
python3 scripts/benchmark_demo.py target/release/serverui --seconds 15
python3 scripts/benchmark_demo.py target/release/serverui --real --seconds 120
```

Los tres últimos comandos requieren Linux; el fixture SSH y sus restricciones
están en README. Windows ejecuta demo y omite integraciones cfg Linux.

No se ha probado una VM Ubuntu Server completa, otros clientes/emuladores ni
SSH entre máquinas diferentes. Procesos solo visibles, capacidad por montaje,
exclusión predeterminada de FS de red/tmpfs y semántica de namespaces/cgroups se
documentan. La identidad de inicio usa resolución de un segundo de sysinfo.
Las syscalls ya bloqueadas no admiten cancelación forzosa: el proceso no espera
join de los hilos de lectura al terminar.

La licencia open source sigue pendiente. La publicación autorizada de v0.2.0
conserva v0.1.0 y sus binarios. Stage 2 no está autorizado.

## Corrección de instalación en Ubuntu 22.04

El usuario informó un fallo de sha256sum en Ubuntu 22.04.5 x86_64: el nombre
terminaba en CR. Se reprodujo localmente: el manifiesto existía, el archivo
también y sus bytes terminaban en CRLF. La prueba nueva
`scripts/test_package_release.py` falló antes de la corrección y pasó después de
forzar newline LF al escribir el checksum en Windows.

El binario inicial se había construido en Debian 13 y requería glibc 2.39.
Se preparó un reemplazo con `scripts/Dockerfile.ubuntu22-build`, base Ubuntu
22.04 fijada por digest, Rust 1.98.1 y compilación UID 1000. Pasaron las 39
pruebas, Clippy, release y nueve casos PTY en Ubuntu 22.04. readelf confirmó
requisito máximo GLIBC_2.34, compatible con el baseline glibc 2.35.

El mismo reemplazo pasó cuatro sesiones SSH en Ubuntu 22.04 y otras cuatro en
Ubuntu 24.04, con la TUI como UID 1001. GNU sha256sum en Ubuntu 22.04 aceptó el
manifiesto generado en Windows. Posteriormente el usuario confirmó que el
reemplazo funciona en su laptop Ubuntu Server 22.04.5 x86_64. Esta confirmación
es un smoke manual informado por el usuario; no se atribuyen pruebas específicas
adicionales ni mediciones de esa laptop que no haya comunicado.

CI cambia el runner GNU a Ubuntu 22.04 y añade validación ABI y regresión del
empaquetador en Linux/Windows. La guía de transferencia e instalación está en
`docs/INSTALL_UBUNTU.md`. La métrica de consumo anterior corresponde al build
histórico Debian; no se presenta como una nueva medición del build Jammy.
