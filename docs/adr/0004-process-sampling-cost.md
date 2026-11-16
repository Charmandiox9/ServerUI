# ADR 0004 Una lectura acotada por proceso Linux

16 de noviembre de 2026. Optimización de Stage 2 autorizada por el usuario.

En la laptop física el benchmark real incumple CPU <1 % de un núcleo. El perfil
por hilo localiza el coste dominante en el recolector. Un probe con 30 muestras
obtiene 18,3997 ms/refresco de procesos, 0,8389 ms de CPU/memoria globales y
0,5241 ms de discos; estos últimos solo se actualizan cada 5 s. strace agregado
registra miles de aperturas, lecturas y statx, sin registrar argumentos ni datos.

Se adopta sustituir exclusivamente la enumeración de procesos de sysinfo por una
lectura acotada de /proc/PID/stat, con APIs seguras de std y sin dependencias
nuevas. sysinfo conserva CPU global, memoria, discos y boot time. No se consultan
argumentos, entorno, ejecutables, usuarios ni tareas. No se retienen FDs por PID.
El coste se verificó en el build completo; no se reduce el refresco de 1 s.

El parser debe tolerar espacios, paréntesis y bytes no UTF-8 en comm, comprobar
PID y campos necesarios y rechazar registros incompletos/excesivos. La CPU por
proceso usa delta de ticks dividido por tiempo monotónico y AT_CLKTCK; 100 % es
un núcleo. RSS usa páginas por AT_PAGESZ. Esos dos parámetros se leen una vez del
auxv del propio proceso, sin exponer su contenido. La identidad interna incluye
los ticks de inicio; la interfaz conserva el inicio Unix en segundos existente.
Procesos nuevos/reutilizados o contadores regresivos quedan pendientes de CPU.

Límites: hasta 10000 procesos, 4096 bytes por stat, cache/baselines acotados y
desapariciones tratadas como carreras normales. RSS de proc/stat es aproximada,
como otras lecturas proc, y no se presenta como contabilidad exacta. Inaccesibles
no se convierten en ceros ni en conteos de procesos ocultos. Las comparaciones
Linux y lifecycle/FD, así como PTY/SSH, se conservan antes de adoptar el cambio.

Referencias: documentación proc del kernel, proc_pid_stat(5) y getauxval(3).

Validación: 49 tests Windows/57 Linux, fmt/Clippy/release, PTY/SSH y lectura
física aprobados. La comparación de 30 muestras en la laptop obtuvo
71,2021 ms frente a 18,7996 ms por lista de procesos. El benchmark completo de
60 s pasó de 3,9801 % a 0,8999 % de un núcleo; RSS máximo de 4149248 a 3657728
bytes. Son muestras no concurrentes en un host con otra carga, no percentiles ni
garantía universal. El probe se conserva como ejemplo `sampling_cost` y el
benchmark PTY original sigue siendo la regresión de CPU del binario completo.
