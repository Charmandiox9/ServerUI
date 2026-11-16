# Aceptación Stage 2 / v0.3.0

16 de noviembre de 2026. **Aceptación funcional completada con la alternativa
SSH autorizada; publicación pendiente de CI en PR/main.**
El usuario sustituyó el requisito de VM por pruebas controladas por SSH en
`USUARIO@IP_AUTORIZADA`. La optimización física de CPU y las acciones del fixture
pasaron; el usuario autorizó expresamente ejecutar y limpiar las unidades.
Ver `SERVICE_ACCEPTANCE_SSH.md` para el alcance del gestor de usuario, sin sudo
ni cambios de polkit, y límites respecto al gestor del sistema y al snapshot.

## Revisión de preparación previa a la optimización

El 16 de noviembre de 2026 el usuario autorizó commits, PR y release con changelog
condicionados a que todo Stage 2 estuviera completo. Se contrastaron el árbol de
trabajo, los contratos/pruebas de servicios y las secciones 6, 7 y 11 del Charter
original. Resultado: **no apto para publicar como Stage 2 terminado**.

| Criterio | Estado comprobado |
| --- | --- |
| Servicios, logs, acciones rápidas y confirmación | Implementados; demo y lectura real verificadas |
| Calidad Windows/Linux y terminal | Evidencia aprobada registrada debajo; no se repitió para esta revisión documental |
| Acciones autorizadas y errores controlados en VM | Pendiente; requisito de salida explícito de la sección 6 del Charter |
| Rendimiento físico de CPU | Pendiente; benchmark real incumple el umbral del proyecto |
| README y changelog | Preparados como desarrollo sin publicar |
| CI remota, integración y release | Pendientes del cierre; no se han ejecutado |

La autorización condicional se conserva para cuando se resuelvan estos pendientes.
En esta revisión no se hicieron commits, PR, tags ni releases; no se sustituye la
validación systemd real por mocks o por el smoke físico de solo lectura.

## Entregado

Servicios cargados del sistema: listado, filtro, selección, captura de journal,
iniciar/detener/reiniciar/habilitar con confirmación y registro volátil de 16
resultados. Demo simula sin acceso al host. Dashboard/procesos siguen de lectura.
Comandos sin shell ni elevación, una petición de servicios en vuelo, worker creado
al primer acceso, salida/tiempo limitados. TUI sin animaciones ni nuevos ticks,
barras CPU/RAM/swap, tabla de montajes y selección invertida.

## Evidencia

Rust 1.98.1. Windows x86_64 MSVC y contenedor Ubuntu 22.04 x86_64 GNU como UID
1000, sin red ni capacidades, repositorio de solo lectura. Fuentes vendorizadas
temporales para compilar sin descargar. Kernel WSL2 6.18.58.1, 12 CPU lógicas.

| Comprobación ejecutada | Resultado |
| --- | --- |
| cargo fmt --all -- --check | Aprobado |
| cargo clippy --workspace --all-targets --locked -- -D warnings | Aprobado Windows/Linux |
| cargo test --workspace --locked | 49 aprobados Windows, 53 Linux |
| cargo build --release --locked | Aprobado Windows/Linux |
| Demo Windows en ConPTY | Dashboard, Servicios, propuesta Detener, cancelación n y salida limpia observadas |
| Empaquetador y SHA-256 portátil | Regresión aprobada en Windows |
| ABI Linux Ubuntu 22.04 | GLIBC_2.34 requerido, inferior al baseline 2.35 |
| PTY Linux | Nueve casos de ciclo de vida, teclado y resize aprobados |
| PTY demo Stage 2 | Cancelación, confirmación única, historial, logs, filtro, ayuda y cleanup aprobados |
| SSH Ubuntu 22.04 | Cuatro sesiones demo/real aprobadas, UID 1001, claves efímeras y loopback aislado |
| Detector visual Impeccable | Sin hallazgos; no sustituye inspección del buffer |

Se utilizó CARGO_INCREMENTAL=0 en Windows por las restricciones de hard links del
sandbox, sin configuración persistente. Las comprobaciones corresponden al árbol
local de `feat/stage-2-services`; no hay CI remota de estos cambios ni publicación.

### Rendimiento medido

Mismo harness, Ubuntu 22.04, release, PTY 80×24, ASCII/sin color, refresco 1000 ms.
Cada muestra dura 30 s. La base es el binario Jammy publicado de v0.2.0.

| Medida | v0.2.0 demo | v0.3.0-dev demo | v0.3.0-dev real |
| --- | --- | --- | --- |
| CPU de un núcleo | 0,2000 % | 0,2333 % | 0,2666 % |
| RSS máximo, bytes | 3276800 | 3342336 | 3694592 |
| Primer frame, ms | 9,138 | 2,327 | 2,916 |
| Primera muestra, ms | 57,872 | 53,110 | 53,700 |
| Respuesta de ayuda, ms | 0,611 | 0,338 | 0,489 |

La demo final añade 65536 bytes de RSS máximo y 0,0333 puntos porcentuales de
CPU en esta muestra. La resolución del contador es 10 ms; no se interpreta esa
diferencia como una tendencia estable ni se garantiza coste idéntico. Los tres
ensayos pasan el umbral de CPU inferior a 1 % y memoria inferior a 30 MB del
harness. No se observó regresión apreciable de respuesta; tampoco se afirma una
mejora estadística de arranque. La medición real no tiene una base real equivalente
en esta sesión. No son pruebas de diez minutos, hosts grandes ni acciones systemd.

Tests puros cubren autorización vinculada a unidad/operación, cancelación,
confirmación única, desaparición, captura de teclado, filtros, historial/logs
acotados, saturación (1000 peticiones), worker lento, pánico y creación diferida.
Fixtures de procesos comprueban éxito, salida no cero, timeout y exceso
de salida; nunca invocan systemctl/journalctl ni mutan servicios.

La CI Windows detectó fragilidad en el arranque del fixture Python. Se sustituyó
por un hijo del propio ejecutable de tests Rust, manteniendo los cuatro casos y
el timeout de producción. El helper hijo está marcado ignored en la suite normal
y se invoca explícitamente desde la prueba de frontera; no es un test omitido.

Capturas de `render_preview` inspeccionadas a 80×24 y 40×12; provienen del buffer
real de Ratatui con datos simulados y no equivalen a una sesión SSH física.
Los tests también cubren controles/escapes, ausencia de color y tamaños extremos.

## Checklist de cierre actualizado

1. Alternativa SSH autorizada a la VM ejecutada sobre unidades desechables del
   usuario; recuperación por hashes completada. Sin modificar servicios existentes,
   red, SSH o polkit. No equivale a snapshot ni prueba permisos del gestor del sistema.
2. Cancelación, iniciar, detener, reiniciar y habilitar aprobados desde la TUI.
   Denegación de escritura EACCES en el directorio aislado produce error sin cambios;
   no se presenta como una prueba de denegación polkit.
3. Estados comparados mediante systemctl --user independiente: reinicio cambia
   MainPID, habilitar no inicia, fallo intencional controlado y timeout sin bloquear
   UI con job completado después. Referencias D-Bus y fixtures retirados al terminar.
4. El smoke de lectura de la laptop pasó también con el build optimizado. Queda
   el registro de publicación tras CI. El alcance de gestor de usuario y sus
   limitaciones se comunicaron antes de la autorización concreta del usuario.
5. Revisión, autorización de publicación y CI en PR/main antes de tag/release.

## Smoke exploratorio autorizado en laptop física

El 16 de noviembre de 2026 el usuario autorizó conectar y probar ServerUI por
SSH en su laptop Ubuntu Server 22.04.5 x86_64, kernel 5.15.0-190-generic,
glibc 2.35, UID 1000. Se limitó la prueba a demo y operaciones reales de lectura;
no sustituye la aceptación de mutaciones en VM. No se utilizó sudo ni se cambió
polkit, configuración de servicios o instalación existente de ServerUI.

Se copió v0.3.0-dev a `/tmp/serverui-smoke.QGEm9ENT/serverui`, con los harness
de desarrollo, y se comprobó SHA-256 idéntico al build Linux local:
`19208131509cb154f2e5e87e02b72f46cef7072d00647697eb1bfe238deb44a9`.

Pruebas aprobadas:

- Recorrido demo de servicios: cancelar, confirmar una vez, historial, logs,
  filtro, ayuda, Dashboard y cleanup. Las mutaciones de este caso son simuladas.
- Demo normal/vacía/error y Linux real en ASCII/sin color y modo predeterminado.
  Resize a 40×12 y SIGWINCH, teclado, salida y restauración termios/cursor/pantalla.
- CPU tras baseline válido, RAM total 3734,0 MiB comparada con `/proc/meminfo`
  y uptime comparado con `/proc/uptime`; no se atribuye una comparación de CPU
  independiente ni precisión de RAM usada que este smoke no midió.
- Filtro e inspección de ServerUI; PID/nombre contrastados con `/proc/PID/comm`.
- Lista systemd contrastada con consulta independiente (146 unidades admitidas),
  filtro `cron.service`, estado `active` coincidente y captura de journal abierta.
  El contenido del journal no se volcó a herramientas ni documentación.
- Historial real vacío comprobado al finalizar: cero acciones reales ejecutadas.

El primer intento del harness esperaba exclusivamente el contador de servicios,
pero esta máquina activa el aviso previsto de lista limitada por unidades no
admitidas. Se corrigió esa expectativa del harness temporal y el recorrido pasó;
no se cambió el binario. Esa omisión sigue siendo un límite documentado.

También se abrió una sesión TTY directa mediante `ssh -tt` desde Windows: se
observaron Dashboard, filtro `serverui`, detalle del proceso y salida con código
0, cursor y pantalla restaurados. Este caso sí atraviesa el transporte SSH de
la TUI; las mediciones siguientes usan un PTY local en la laptop remota.

### Rendimiento físico inicial, antes de optimizar

Refresco 1000 ms, PTY 80×24, ASCII/sin color, cuatro CPU lógicas. Son ventanas
cortas en una máquina con otros servicios activos; no son comparaciones
estadísticas controladas. El contador de CPU tiene resolución de 10 ms.

| Medida | v0.3.0-dev real, 60,05 s | v0.3.0-dev demo, 15,01 s | v0.2.0 real, 15,05 s |
| --- | --- | --- | --- |
| CPU de un núcleo | 3,9801 % | 0,5331 % | 3,1229 % |
| RSS máximo, bytes | 4149248 | 3137536 | 3526656 |
| Primer frame, ms | 7,868 | 4,769 | 2,919 |
| Primera muestra, ms | 54,613 | 56,269 | 103,860 |
| Respuesta de ayuda, ms | 0,653 | 0,653 | 0,741 |

La medición real de 60 s falló explícitamente la aserción de CPU inferior a 1 %
de un núcleo del harness. Memoria y respuesta sí cumplieron sus umbrales. La demo
pasó. v0.2.0 también falló la CPU en modo real; no se atribuye todo el coste a las
mejoras visuales ni se descarta una diferencia entre versiones con estos datos.

Una repetición instrumentada de 15,04 s del mismo binario nuevo obtuvo 2,5263 %
total: hilo de UI 0,3989 %, recolector 2,0610 % y lector de teclado 0 %. Apunta al
trabajo de recolección real como coste dominante, no a espera activa del teclado.
No separa todavía enumeración de procesos y lectura de montajes. La variación
entre ventanas impide afirmar una cifra estable o cerrar el análisis de CPU.

Se añadieron únicamente harness temporales bajo target local y el directorio
temporal remoto; no hubo instrumentación en el código de producción ni cambios
del binario. La funcionalidad de lectura quedó probada; rendimiento físico de CPU
queda pendiente de optimización/aceptación. Los resultados de contenedor previos
no se extrapolan a esta laptop.

## Límites

Solo unidades cargadas, no todos los servicios instalados. Nombres restringidos
ASCII sin patrones/rutas/escapes; los no admitidos se omiten con aviso. Máximo
2048 servicios, 80 líneas de log de 256 caracteres, 256 KiB por salida y 5 s por
comando. Los logs pueden estar incompletos por permisos. Resultados fallidos
usan errores acotados sin volcar diagnósticos o entorno. El historial no persiste.

Matar el cliente systemctl no cancela necesariamente un job aceptado ni revierte
efectos; al salir no se espera join del worker. Las acciones pueden afectar
dependencias y SSH. No hay sudo, editor polkit, rollback automático ni garantías
de salud por un exit code exitoso. No hay pruebas VM ni de systemd real aceptadas
por inferencia desde Docker. APT y demás stages siguen fuera del alcance.

## Optimización adoptada y verificación posterior

El usuario autorizó optimizar y sustituir la validación VM por una prueba
controlada por SSH. Se midió el proveedor: el coste dominante fue la enumeración
de procesos. Se adoptó ADR 0004, con una lectura acotada de proc/PID/stat y sin
FDs por proceso, argumentos, entorno, tareas, dependencias nuevas o unsafe.
sysinfo conserva CPU global, RAM/swap, discos y boot time. CPU de procesos
mantiene 100 % por núcleo; nuevos/reutilizados quedan pendientes y RSS es aproximada.

| Verificación posterior | Resultado |
| --- | --- |
| fmt/Clippy workspace all-targets | Aprobados Windows/Linux |
| cargo test workspace locked | 49 Windows, 57 Linux |
| Release Windows/Linux | Aprobadas; ABI máxima GLIBC_2.34 |
| PTY Linux / demo Stage 2 | Nueve casos y recorrido de servicios aprobados |
| Benchmark contenedor 30 s demo/real | CPU 0,2000 % / 0,2333 %, aprobados |
| Smoke físico optimizado | Demo, resize/teclado, métricas/procesos, systemd/journal y cleanup aprobados |
| Seguridad de fixtures Python | Seis tests Windows y seis Ubuntu, sin ejecutar systemctl real |

Pruebas nuevas del parser: paréntesis/controles/no UTF-8 en comm, truncado,
overflow, formato nativo de auxv, CPU multinúcleo, tiempo inyectado, reutilización
de PID incluso dentro del mismo segundo para CPU, desaparición y limpieza del
baseline. Se conservan las integraciones de proc/memoria/uptime/discos y FDs.

Benchmark físico optimizado, kernel 5.15, cuatro CPU lógicas, 60,00 s, PTY 80×24,
ASCII/sin color y refresco 1000 ms: **CPU 0,8999 % de un núcleo**, RSS máximo
3657728 bytes, primer frame 2,765 ms, primera muestra 54,185 ms, ayuda 0,588 ms.
La aserción CPU <1 % que fallaba antes pasó. Frente a la muestra previa de 60 s
(3,9801 %, RSS 4149248), representa ~77 % menos CPU y menor RSS. La carga del host
varía; no es una garantía universal ni una prueba de diez minutos.

El build optimizado se conserva en `/tmp/serverui-opt.1jT64OLH/serverui`; no
reemplaza ninguna instalación. El anterior permanece en su directorio temporal.
SHA-256: `0cb24500c59bbf8a416af7a956bbbeac525bbc001381b8df2aa5f3b73ebf58111`.
Este smoke de lectura no ejecutó mutaciones; el recorrido autorizado posterior
de acciones se registra a continuación.

## Acciones reales autorizadas y recuperación

El usuario autorizó explícitamente crear, probar y retirar los fixtures.
`service_real_smoke.py` validó hashes, UID 1000 y ControlGroup del gestor del
usuario antes de iniciar la TUI optimizada. Se usó la dependencia python3-dbus
ya instalada, solo en el harness, para mantener RefUnit mientras duraba la prueba.

Ocho casos aprobados: cancelación sin acción/historial, inicio active, reinicio
con MainPID nuevo, parada inactive, denegación de escritura de enable, enable sin
inicio, servicio fallido con Result=exit-code, y timeout con UI responsiva y job
que finaliza posteriormente. El historial contiene una sola solicitud lenta.
Termios, cursor y pantalla restaurados. No se invocó sudo ni se cambió polkit.

El primer intento no ejecutó acciones porque systemd descargó unidades inactivas.
Se detectó además un reset-failed innecesario al limpiar unidades descargadas;
se corrigió el helper y se completó cleanup. Después se usaron referencias D-Bus
para mantenerlas cargadas y el recorrido completo pasó. Esto afectó al harness,
sin cambios de código de producción. Siete pruebas de seguridad del helper,
incluida limpieza de unidades descargadas/fallidas, pasaron en Windows y Ubuntu.

Cleanup retiró los cuatro archivos de unidades, directorio de enlaces y manifiesto.
Una consulta SSH independiente confirmó `FIXTURES_ABSENT`. Los registros del
journal permanecen. No se probaron privilegios del gestor del sistema, boot del
host ni rollback integral; la excepción de validación es explícita y autorizada.
