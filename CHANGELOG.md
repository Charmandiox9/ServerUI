# Changelog

## v0.3.0 — 2026-11-16

- Stage 1 cerrado: PR #2 integrado, CI Linux/Windows aprobada y v0.2.0 publicada.
- Stage 2 autorizado: servicios systemd cargados, filtro y captura de logs por unidad.
- Acciones rápidas iniciar/detener/reiniciar/habilitar con confirmación explícita.
- Demo aislada, errores legibles e historial volátil de 16 resultados.
- Worker de servicios diferido, independiente y con una operación en vuelo.
- Comandos tipados sin shell/sudo, acotados en tiempo/salida y sin reintento automático.
- TUI con barras de uso, tabla de montajes y selección invertida; conserva ASCII,
  ausencia de color, modo compacto y renderizado por cambios.
- Pruebas de confirmación, cancelación, saturación, argumentos, límites y PTY demo.
- Alternativa a VM autorizada: prueba SSH con unidades desechables del usuario,
  sin sudo ni cambios polkit; procedimiento y límites documentados.
- Ocho casos reales de acciones del gestor de usuario aprobados, con estados
  contrastados, timeout responsivo y limpieza confirmada independientemente.
- Fixtures protegidos por hashes, RefUnit para evitar GC y siete regresiones
  de seguridad; no se atribuyen pruebas de permisos del gestor del sistema.
- Smoke físico por SSH de demo, métricas, procesos y consultas systemd/journal aprobado.
- Recolector de procesos optimizado con lectura acotada de proc/stat, sin FDs por
  PID ni nuevos argumentos/entorno/tareas. Conserva CPU por núcleo y baseline.
- Benchmark real en laptop, 60 s/1000 ms: CPU de 3,98 % a 0,90 % de un núcleo;
  RSS máximo de 4,15 a 3,66 MB. Regresiones Linux y smoke físico repetidos.

## v0.2.0 — 2026-11-16

- Checksums con saltos LF explícitos, compatibles con sha256sum al empaquetar desde Windows.
- Baseline Linux Ubuntu 22.04/glibc 2.35 y comprobación ABI en CI.
- LinuxProvider de solo lectura con CPU, RAM, swap, montajes, uptime y procesos.
- Procesos demo/reales con filtro nombre/PID, orden estable, desplazamiento y detalle.
- Identidad PID + inicio, cierre de inspección al desaparecer/reutilizar PID.
- CPU pendiente hasta baseline válido; CPU de proceso en porcentaje de un núcleo.
- Inicialización/recolección en worker con canales acotados y peticiones coalescidas.
- Snapshots compartidos sin copiar la lista por cada tecla.
- Lector de teclado bloqueante y cola acotada para evitar consumo activo de poll Unix.
- Errores del worker estructurados y limpieza del terminal reservada al hilo de UI.
- Pruebas Linux contra /proc/stat, /proc/meminfo, uptime y stat -f; fixtures de procesos.
- PTY y SSH Ubuntu extendidos al modo real; regresión corta de CPU en CI.
- Funcionamiento general confirmado por el usuario en una laptop Ubuntu Server 22.04.5 x86_64 mediante SSH.

Sin acciones administrativas ni funcionalidades de Stage 2. Windows conserva modo demo.

## v0.1.0 — 2026-11-16

- Workspace Rust con núcleo independiente, proveedores, TUI y binario separados.
- MockProvider determinista, snapshots tipados y escenarios normal/vacío/error.
- CLI demo con refresco configurable, ASCII y ausencia de color.
- Dashboard simulado, pantallas provisionales de Procesos, Servicios y Ajustes.
- Navegación por teclado, ayuda contextual, resize y modo compacto.
- Loop por eventos con control de ticks y renderizado solo ante cambios.
- Restauración del terminal en salida, errores y pánicos con unwinding.
- Pruebas de dominio, mock, runner, CLI, buffers y guard de terminal.
- CI Linux/Windows y prueba Linux PTY con fixture de fallos independiente.
- Sondeo Unix de descriptores para corregir pérdida de teclado durante resize.
- Fixture SSH Ubuntu 24.04 aislado y harness de medición de recursos del release.
- Convenciones, plan aprobado y checklist de aceptación documentados.

Sin métricas ni administración real del host. Primera versión de demostración.
