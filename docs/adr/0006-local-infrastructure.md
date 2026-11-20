# ADR 0006 Contenedores locales e inventario de infraestructura

Stage 4, v0.5.0.

Docker usa el cliente absoluto `/usr/bin/docker` con `--host
unix:///var/run/docker.sock`. Podman usa `/usr/bin/podman` con
`--remote=false`. Los adaptadores quitan las variables que eligen servidores
remotos. Los permisos dependen de la política existente del equipo. Se usan
los almacenes locales del motor, sin elevar la TUI ni instalar reglas.

Los comandos autorizados son list, logs, start, stop y restart. Las operaciones
usan argumentos separados y el ID hexadecimal completo de 64 caracteres.
La confirmación captura motor, ID y operación. Antes de logs y cambios se
comprueba el ID mediante un listado filtrado porque el motor también admite
identificadores por nombre. El listado y la acción son llamadas separadas.
Detener y reiniciar pasan
diez segundos de gracia al motor, que puede forzar la terminación después.
Se espera hasta cinco segundos para consultas y 25 para acciones. Un timeout
del cliente deja el estado potencialmente incierto. Se ofrece refrescar el
listado y no se reintentan mutaciones automáticamente. Una desconexión o
salida de la TUI tampoco garantiza que el motor cancele la acción.

Un worker independiente y diferido mantiene una petición en vuelo para
Contenedores y Red y discos. El listado se limita a 256 contenedores.
Los logs se solicitan con `--tail 80`, sin seguimiento, y se muestran los
dos flujos del cliente agrupados por separado. Cada flujo tiene un límite de
256 KiB y las líneas se neutralizan antes de mostrarse.

La vista de red consulta `ip -br address show` y distingue la ausencia de
direcciones de la falta de `ip`. En un sistema mínimo se consulta el estado
en `/sys/class/net`. La vista de bloque usa columnas explícitas de `lsblk`
y la de montajes reusa las capacidades del snapshot. Ambas consultas de
inventario se limitan a 256 entradas. No se consultan rutas remotas ni se
envían paquetes. El modo demo permanece aislado.

Referencias [Docker list](https://docs.docker.com/reference/cli/docker/container/ls/),
[Docker logs](https://docs.docker.com/reference/cli/docker/container/logs/),
[Podman ps en Ubuntu 22.04](https://manpages.ubuntu.com/manpages/jammy/man1/podman-ps.1.html) y
[opciones de Podman](https://docs.podman.io/en/stable/markdown/podman.1.html).
