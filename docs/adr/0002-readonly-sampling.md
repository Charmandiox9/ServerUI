# ADR 0002 Recolección de solo lectura fuera de la UI

Fecha: 16 de noviembre de 2026. Adoptado para Stage 1 autorizado por el usuario.

Linux puede tardar enumerando procesos o consultando montajes. Un worker estándar
dedicado inicializa el proveedor y recoge snapshots, con peticiones/resultados
de capacidad 1 y una sola consulta pendiente. Coalescer peticiones redundantes y
ejecutar en serie evita acumulación y respuestas fuera de orden.

La UI comprueba respuestas sin bloquear y espera eventos un máximo de 50 ms.
Comparte snapshots inmutables mediante Arc para evitar copiar listas por tecla.
El núcleo conserva tipos estándar y transiciones comprobables.

Al salir se marca cancelación y desconectan canales; no se hace join bloqueante.
Una syscall iniciada no puede cancelarse con seguridad desde otro hilo; el
proceso termina sin esperar ese worker. No crear workers adicionales para
sortear bloqueos. Un pánico del recolector se convierte en error estructurado;
el hook de restauración solo actúa para pánicos del hilo de UI.

LinuxProvider utiliza sysinfo con features system/disk y refrescos específicos.
Se excluyen argumentos, entorno, ejecutables, tareas de proceso y GPU. CPU usa
baselines; los datos sin muestra válida se identifican. La presentación usa
PID + inicio y muestra montajes separados, con límites explícitos.

La lectura de teclado también usa un único hilo bloqueante, con una cola de 64
eventos y un recv_timeout estándar en la UI. La medición encontró ~1,8 % de un
núcleo por la cola submilisegundo del poll temporizado de Crossterm/Unix, incluso
en un reproductor sin proveedor ni aplicación. Conservar el backend Unix que
resuelve SIGWINCH pero leer sin timeout evita ese sondeo activo. El lector es
propiedad del proceso; no hacer join de un read bloqueado durante cleanup. CI
conserva un benchmark corto y la regresión simultánea de resize/teclado.
