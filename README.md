# ServerUI

Administración visual de Ubuntu Server desde la terminal, construida en Rust.
**Stage 4 / v0.5.0 con contenedores, red y discos.**
Dashboard y procesos ofrecen consultas de solo lectura. Servicios, Paquetes y Contenedores
ofrecen acciones con confirmación explícita. Ejecuta la TUI como usuario normal.

La [release v0.5.0](https://github.com/Charmandiox9/ServerUI/releases/tag/v0.5.0)
incluye binarios x86_64 Linux GNU y Windows MSVC, más checksums SHA-256. Linux
permite el modo real y la demo. Windows incluye la demo. Las releases v0.1.0,
v0.2.0, v0.3.0 y v0.4.0 corresponden a los Stages 0 a 3. La licencia open source sigue pendiente de elección;
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
| 1–7 | Dashboard, Procesos, Servicios, Ajustes, Paquetes, Contenedores, Red y discos |
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
| q / Ctrl+C | Salir cuando APT termine. Durante la edición, q es texto |

El orden recorre CPU, RAM, PID y nombre; los empates se resuelven por identidad
para evitar saltos. La selección se conserva al actualizar y reordenar. Si el
proceso inspeccionado desaparece o cambia su inicio con el mismo PID, se cierra
el detalle con aviso. La identidad pública conserva resolución en segundos;
no se ofrecen acciones basadas en esa identidad.

La ayuda captura navegación pero permite refrescos y salida. Ajustes muestra opciones de esta ejecución,
sin editor persistente. Se admiten 80×24, modo compacto 40×12 y aviso en tamaños
menores sin perder salida por teclado. No se requieren ratón, truecolor ni iconos.

## Servicios, logs y acciones rápidas (Stage 2)

En Servicios, la primera entrada consulta las unidades `.service` cargadas en el
systemd del sistema. No enumera todos los archivos instalados ni plantillas sin
instancia. `/` filtra por unidad, j/k y Home/End seleccionan, Enter abre las últimas
80 líneas visibles del journal; j/k desplaza y Escape vuelve. `r` actualiza la
lista o la captura de logs. No hay seguimiento continuo ni consultas de servicios
en cada tick de métricas. Nombres escapados o patrones no admitidos se omiten con
aviso; máximo 2048 unidades y salida de comando máxima de 256 KiB.

| Tecla en Servicios | Acción propuesta |
| --- | --- |
| a | Iniciar |
| d | Detener |
| R | Reiniciar |
| e | Habilitar al arranque; no inicia el servicio |
| y en confirmación | Ejecutar una vez la operación y unidad mostradas |
| n / Escape en confirmación | Cancelar sin ejecutar |
| h | Historial volátil de los últimos 16 resultados |

Cada acción identifica la unidad exacta y advierte que puede afectar dependencias
y cortar SSH. j/k permite revisar nombres largos en la confirmación. La demo
simula todas las acciones sin ejecutar comandos ni leer el host. En Linux real
se usan `/usr/bin/systemctl` y `/usr/bin/journalctl`, argumentos separados, sin
shell, stdin, pager ni agente de autenticación. Un permiso insuficiente o systemd
no disponible produce error legible. No se modifica polkit ni se eleva la TUI.

Los comandos se ejecutan en un worker independiente, creado solo al abrir
Servicios, con una petición en vuelo y plazo de 5 s. Un timeout mata el cliente,
pero no revierte ni garantiza cancelar el job ya recibido por systemd: el
resultado puede ser incierto. Comprobar con `r` antes de decidir un reintento.
Una salida correcta tampoco garantiza la salud posterior del servicio. El journal
puede estar vacío o incompleto por permisos; no se infiere acceso total.

La TUI añade barras de CPU/RAM/swap en 80×24, tabla de montajes y selección
invertida visible sin color. Conserva modo compacto, ASCII y colores ANSI
opcionales; no añade animaciones ni aumenta la frecuencia de renderizado.

## Paquetes APT en Ubuntu (Stage 3)

Abre Paquetes con `5` para ver una selección de herramientas recomendadas que
figuran en los índices locales de APT. Pulsa `/` y escribe parte de un nombre.
`Enter` busca y `r` repite la consulta. Envía una búsqueda vacía para volver
a los recomendados. La demo muestra paquetes simulados.
Selecciona con `j/k`. `i` propone instalar, `u` actualizar el paquete
instalado a su versión candidata y `d` eliminar conservando su configuración.

Revisa la simulación, incluidas las dependencias y versiones, y usa `y` para
aplicar o `n/Esc` para cancelar. ServerUI vuelve a simular antes de ejecutar.
La salida indica el progreso disponible y conserva las últimas 24 líneas en
memoria. Puedes cambiar de pestaña mientras trabaja. `q/Ctrl+C` vuelve a
Paquetes hasta que termine APT.

Las consultas funcionan con permisos de usuario. Aplicar requiere
`/usr/bin/pkexec` y autorización de la política de polkit ya configurada.
En una sesión SSH sin agente, puedes registrar `pkttyagent --process PID` en
una segunda terminal con el PID de ServerUI, cuando polkit y el agente estén
disponibles. Los índices se mantienen con el flujo de administración del equipo.

Si el plan instaló un único paquete nuevo y no instaló ni actualizó otros,
`b` prepara otra
simulación para retirarlo. Los scripts de instalación pueden dejar efectos
propios y las actualizaciones no ofrecen downgrade automático. Ante una
interrupción, comprueba con `dpkg --audit` y revisa el estado antes de repetir.
[Aceptación y límites de Stage 3](docs/ACCEPTANCE_STAGE_3.md).

## Contenedores, red y discos (Stage 4)

Pulsa `6` para abrir Contenedores. `e` alterna entre Docker y Podman, `j/k`
elige el contenedor y Enter abre una captura de logs. `r` actualiza la lista
o los logs. `a`, `d` y `R` proponen iniciar, detener y reiniciar.
Revisa el motor y el contenedor que se muestran antes de confirmar con `y`.
`n/Esc` cancela. La detención y el reinicio pueden interrumpir el servicio.

Docker usa `/var/run/docker.sock` con los permisos existentes, y Podman usa
el almacenamiento local del usuario. Los clientes se buscan en `/usr/bin`.
La demo simula ambos motores, así como sus logs y cambios de estado.

Pulsa `7` para abrir Red y discos. `n` muestra interfaces y direcciones,
`b` muestra los dispositivos de bloque y `m` muestra los montajes con
capacidad. `j/k` desplaza el inventario y `r` vuelve a consultar. En Linux,
si falta `ip`, los nombres y estados de las interfaces se leen de sysfs.
[Alcance y aceptación de Stage 4](docs/ACCEPTANCE_STAGE_4.md).

## Interpretar las métricas

- CPU global: 0–100 %; CPU de procesos: 100 % representa un núcleo lógico y puede
  superar 100 %. -- significa que todavía no existe un baseline válido.
- RAM y swap se convierten de bytes a MiB (1024²). RAM usada en Linux sigue la
  definición total menos disponible de sysinfo, verificada frente a /proc/meminfo.
- La tabla muestra RSS, no memoria virtual. El detalle distingue ambas y muestra
  PID, padre, nombre, estado, CPU, inicio Unix y tiempo activo.
- Procesos Linux usa una lectura acotada de `/proc/PID/stat` por proceso; CPU se
  calcula con ticks y tiempo monotónico. RSS de proc es aproximada. sysinfo sigue
  proporcionando CPU global, RAM/swap y discos. No se leen argumentos ni entorno.
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
La configuración y el historial de ServerUI se mantienen en memoria. Las acciones
sobre servicios y paquetes requieren confirmación.

## Arquitectura

| Crate | Responsabilidad |
| --- | --- |
| serverui-core | Modelos, SystemProvider, estado y transiciones puras |
| serverui-providers | MockProvider/MockServices; LinuxProvider y LinuxServices cfg Linux |
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
python3 scripts/service_smoke.py target/release/serverui
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

El [plan activo](docs/STAGE_4_PLAN.md) y el
[ADR de contenedores e inventario](docs/adr/0006-local-infrastructure.md)
registran el alcance y las decisiones de Stage 4.

El smoke exploratorio en la laptop física pasó para demo y funciones de lectura.
El benchmark real inicial superó el umbral de CPU (3,98 % de un núcleo en 60 s).
Tras optimizar la lectura de procesos obtuvo 0,90 % y 3,66 MB máximos de RSS al
mismo refresco de 1000 ms; ayuda 0,59 ms. Son ventanas cortas en ese equipo, no
garantías para cualquier host. Cifras y límites en docs/ACCEPTANCE_STAGE_2.md.
El usuario sustituyó la VM por una prueba controlada por SSH; el procedimiento
de unidades desechables está en docs/SERVICE_ACCEPTANCE_SSH.md.
