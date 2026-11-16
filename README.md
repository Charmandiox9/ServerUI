# ServerUI

Administración visual de Ubuntu Server desde la terminal, construida en Rust.
**Stage 1 / v0.2.0: Dashboard y procesos de solo lectura.**
LinuxProvider lee CPU, RAM, swap, discos, uptime y procesos visibles. La TUI no
modifica el sistema ni ofrece operaciones privilegiadas.

La [release v0.2.0](https://github.com/Charmandiox9/ServerUI/releases/tag/v0.2.0)
incluye binarios x86_64 Linux GNU y Windows MSVC, más checksums SHA-256. Linux
permite lectura real y demo; Windows conserva demo. La release anterior v0.1.0
corresponde al Stage 0. La licencia open source sigue pendiente de elección;
no se incluye una por inferencia.

Para instalar en Ubuntu Server 22.04 x86_64, seguir
[la guía de instalación](docs/INSTALL_UBUNTU.md). El baseline GNU se compila sobre
Ubuntu 22.04 y CI comprueba requisitos glibc no superiores a 2.35. Los manifiestos
SHA-256 usan saltos LF incluso cuando se generan desde Windows.

## Ejecutar desde el código

Se necesita Rust 1.98.1 mediante rustup y linker de C en Linux, o herramientas
MSVC/Windows SDK en Windows. Es el toolchain fijado y la versión comprobada.
En una terminal Linux, como usuario normal:

```sh
cargo run --locked
cargo run --locked -- --ascii --no-color
cargo run --locked -- --refresh-ms 500
```

En Linux y Windows, demo aislada sin consultar métricas del host:

```sh
cargo run --locked -- --demo
cargo run --locked -- --demo --ascii --no-color
cargo run --locked -- --demo --demo-scenario empty
cargo run --locked -- --demo --demo-scenario error
```

Sin --demo, otras plataformas explican que el proveedor real requiere Linux.
Stdin y stdout deben ser TTY; help/version funcionan con salida redirigida.
Para SSH, ejecutar el binario Linux dentro de la sesión del usuario. No se
requieren daemon, cliente SSH propio, Docker ni privilegios de root para usarlo.

--refresh-ms admite 100–60000 ms; predeterminado 1000. Consultas reales anteriores
al intervalo mínimo de CPU de sysinfo reutilizan la muestra cacheada. Discos se
refrescan a lo sumo cada 5 s. r solicita actualización sin encolar peticiones
ilimitadas; si hay una consulta en vuelo se coalesce la petición.

--ascii evita glifos Unicode y --no-color conserva los colores del terminal.
También se respeta NO_COLOR no vacío. Los estados se identifican con texto.

## Navegación

| Tecla | Acción |
| --- | --- |
| Tab / Shift+Tab / izquierda / derecha | Cambiar sección |
| 1–4 | Dashboard, Procesos, Servicios, Ajustes |
| Arriba/abajo o j/k | Seleccionar proceso; desplazar montajes en Dashboard |
| Home / End | Primera/última fila o montaje |
| / | Editar filtro de procesos por nombre o PID |
| Enter / Escape durante filtro | Terminar edición conservando el filtro |
| Ctrl+U durante filtro | Limpiar filtro |
| s / S | Cambiar criterio de orden / invertir dirección |
| Enter en Procesos | Inspeccionar proceso seleccionado |
| Escape | Cerrar ayuda o detalle |
| r | Actualizar o reintentar |
| ? | Abrir/cerrar ayuda |
| q / Ctrl+C | Salir; en el filtro q es texto, Ctrl+C siempre sale |

El orden recorre CPU, RAM, PID y nombre; los empates se resuelven por identidad
para evitar saltos. La selección se conserva al actualizar y reordenar. Si el
proceso inspeccionado desaparece o cambia su inicio con el mismo PID, se cierra
el detalle con aviso. La identidad usa la resolución en segundos de sysinfo;
no se ofrecen acciones basadas en esa identidad.

La ayuda captura navegación pero permite refrescos y salida. Servicios sigue
siendo un placeholder de Stage 2. Ajustes muestra opciones de esta ejecución,
sin editor persistente. Se admiten 80×24, modo compacto 40×12 y aviso en tamaños
menores sin perder salida por teclado. No se requieren ratón, truecolor ni iconos.

## Interpretar las métricas

- CPU global: 0–100 %; CPU de procesos: 100 % representa un núcleo lógico y puede
  superar 100 %. -- significa que todavía no existe un baseline válido.
- RAM y swap se convierten de bytes a MiB (1024²). RAM usada en Linux sigue la
  definición total menos disponible de sysinfo, verificada frente a /proc/meminfo.
- La tabla muestra RSS, no memoria virtual. El detalle distingue ambas y muestra
  PID, padre, nombre, estado, CPU, inicio Unix y tiempo activo.
- Discos se muestran por montaje como espacio disponible para el usuario frente
  a capacidad total. No se suman montajes que puedan compartir dispositivo.
  Volúmenes sin datos se marcan no disponibles. sysinfo excluye por defecto
  tmpfs y FS de red; en un contenedor pueden verse montajes del entorno anfitrión.
- Procesos visibles no equivale a todos los procesos del host. Permisos, hidepid
  y namespaces pueden limitar la lista; /proc puede reflejar memoria/CPU del host
  y no los límites del cgroup. No se afirma detectar procesos ocultos.
- Snapshot limitado a 10000 procesos y 64 montajes; el recorte se anuncia. Un
  fallo conserva la última muestra con estado de datos antiguos, no inventa ceros.

No se leen argumentos, entorno, rutas de ejecutables, GPU ni tareas/hilos por
proceso. Los nombres, montajes y errores se neutralizan antes de renderizar.
No hay kill, renice, systemd, APT, administración de red/discos ni escritura de
configuración o historial.

## Arquitectura

| Crate | Responsabilidad |
| --- | --- |
| serverui-core | Modelos, SystemProvider, estado y transiciones puras |
| serverui-providers | MockProvider; LinuxProvider cfg Linux con sysinfo mínimo |
| serverui-tui | Teclas, render y ciclo de vida del terminal |
| serverui | CLI, composición, worker de recolección y lector de teclado |

La UI no consulta al proveedor. El recolector inicializa y lee fuera del hilo de
UI; tiene canales de capacidad 1 y una petición pendiente. Snapshots inmutables
se comparten con Arc para no clonar miles de filas por tecla. El lector de
teclado bloquea en su propio hilo con cola de 64 eventos. La UI espera mediante
canales estándar hasta 50 ms y redibuja ante cambios. No requiere runtime async.

Al salir se desconecta/cancela de forma cooperativa, sin join que bloquee la UI.
Una syscall de recolección o lectura de teclado ya bloqueada no puede cancelarse
forzosamente: son hilos del proceso, liberados al terminar este. Pánicos del
recolector se convierten en errores; el hook de limpieza solo restaura el terminal
ante pánico del hilo de UI. Ver ADR 0001 y 0002.

## Desarrollo y verificación

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
```

Los tests de estado/render/mock no leen el host ni dependen del reloj real.
Linux añade pruebas de integración que comparan /proc y stat -f, esperan entre
muestras CPU y crean un proceso sleep efímero que ellas mismas eliminan. Esas
operaciones de fixture no forman parte de la aplicación. Windows verifica demo.

```sh
cargo build --locked -p serverui-tui --example terminal_probe
python3 scripts/pty_smoke.py target/release/serverui target/debug/examples/terminal_probe
python3 scripts/benchmark_demo.py target/release/serverui --seconds 60
python3 scripts/benchmark_demo.py target/release/serverui --real --seconds 120
```

El probe induce fallos/pánicos solo en un ejemplo separado. PTY verifica modos
termios, cursor y pantalla tras salida, error y pánico; incluye un pánico de
background para evitar restaurar una UI activa. El benchmark informa RSS, CPU
por núcleo, primer frame/muestra y ayuda. Los umbrales son controles de desarrollo,
no garantías para cualquier host, FS remoto o conexión WAN.

CI repite fmt/Clippy/tests/release en Linux y Windows; Linux añade PTY, regresión
corta de CPU y SSH Ubuntu. Para repetir SSH con Docker, construir primero el
release Linux:

```sh
docker build --tag serverui-ssh-smoke:local - < scripts/Dockerfile.ubuntu-smoke
docker run --rm --network none --cap-drop ALL \
  --cap-add SETUID --cap-add SETGID --cap-add SYS_CHROOT \
  --cap-add CHOWN --cap-add DAC_OVERRIDE --cap-add KILL --cap-add AUDIT_WRITE \
  --security-opt no-new-privileges \
  --mount "type=bind,source=$PWD,target=/workspace,readonly" \
  serverui-ssh-smoke:local python3 /workspace/scripts/ssh_smoke.py /workspace/target/release/serverui
```

El fixture genera claves temporales y ejecuta demo y real como usuario remoto
normal. sshd solo escucha por loopback del contenedor; no hay puertos publicados,
red externa ni sockets del host montados. Las capacidades son de sshd, nunca de
ServerUI. Construir la imagen sí descarga paquetes. No sustituye una VM completa.

El terminal se restaura tras salida, error y pánico con unwinding. SIGKILL,
panic=abort, fallo del emulador o desconexión abrupta no garantizan cleanup; si
fuera necesario usar stty sane y reset en el shell Linux.

Convenciones en AGENTS.md; plan en docs/STAGE_1_PLAN.md, decisión en ADR 0002 y
resultados efectivos en docs/ACCEPTANCE_STAGE_1.md. Stage 2 no está autorizado.
