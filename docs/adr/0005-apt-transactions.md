# ADR 0005 Operaciones APT con simulación y confirmación

19 de noviembre de 2026. Stage 3, v0.4.0.

`apt-cache --names-only search` consulta índices locales. `apt-get --simulate`
produce el plan textual de cambios `Inst`, `Conf` y `Remv`. Se aceptan nombres
Debian ASCII validados, con destino `:native` al construir la única posición de
paquete. La búsqueda interpreta la entrada literalmente. El límite es de 100
resultados y 100 líneas de plan. Las lecturas tienen un plazo de 8 s y un límite
de 256 KiB por flujo. APT conserva su resolución de dependencias y autenticación
de repositorios configurada. La actualización individual usa `--only-upgrade`.

La primera entrada a Paquetes consulta una lista curada de nombres exactos en
una sola petición. Una búsqueda vacía vuelve a esa lista. Las recomendaciones
se basan en disponibilidad local, sin estadísticas de instalación ni red.

Vincular la confirmación a paquete, operación y plan. Volver a simular y comparar
antes de solicitar autorización. Instalar y actualizar pasan `--no-remove`.
La simulación de eliminar rechaza planes con otros paquetes afectados. Mantener
el comportamiento habitual de APT para configuración existente y usar
`--force-confold` ante un conflicto de configuración dpkg. La ventana entre
simulación y adquisición del lock no es atómica. Cambios concurrentes en el
sistema o la configuración de APT pueden alterar el plan que aplique el gestor.

Las mutaciones invocan `/usr/bin/pkexec --disable-internal-agent /usr/bin/apt-get`
con argumentos separados y entrada cerrada. Se requiere una política y un medio
de autorización ya disponibles en el equipo. ServerUI nunca define reglas ni
abre un agente en la misma terminal. Cada confirmación solicita una única
ejecución. APT y dpkg usan sus locks, con fallo inmediato si el lock está ocupado.

Un worker diferido de APT funciona en paralelo con métricas y servicios. El FD
de progreso de APT proporciona el avance por fases. El canal a la UI tiene
capacidad uno. Se drena la salida y se conservan 24 líneas recientes. La UI
sigue navegable y aplaza la salida voluntaria mientras espera el resultado.
No hay timeout de pared ni cancelación forzada durante dpkg. Una caída o
desconexión abrupta puede interrumpir la operación o dejar su resultado incierto.
Verificar los registros de APT y `dpkg --audit` antes de intentar recuperación.

Tras instalar un único paquete nuevo, la opción de deshacer prepara una
eliminación, con otra simulación y confirmación. Eliminar conserva conffiles y
no revierte efectos de scripts de mantenedor. Para actualizaciones, eliminaciones,
paquetes con dependencias nuevas o recuperaciones con efectos secundarios se
requiere un plan del operador, como respaldos o snapshots previos cuando aplique.

Referencias [apt-get de Ubuntu 22.04](https://manpages.ubuntu.com/manpages/jammy/man8/apt-get.8.html),
[pkexec](https://polkit.pages.freedesktop.org/polkit/pkexec.1.html) y
[formato de progreso en APT](https://sources.debian.org/src/apt/2.6.1/apt-pkg/install-progress.cc/).
