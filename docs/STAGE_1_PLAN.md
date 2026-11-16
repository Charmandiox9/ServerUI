# ServerUI Stage 1 v0.2.0

El usuario autorizó comenzar Stage 1 el 16 de noviembre de 2026, condicionado al
cierre de Stage 0. Se comprobó v0.1.0 publicada y CI satisfactoria en el commit
3a1f46764749b000c01b1a1e5920f8c7e8de8a32 de main. El PR #1 está integrado.

Fuente: sección 6 del charter: Dashboard y procesos de solo lectura, métricas
verificadas en Linux y UI fluida. Se implementan CPU, RAM, swap, discos montados,
uptime y procesos visibles con filtro por nombre/PID, orden CPU/memoria/PID/nombre
e inspección. No hay kill, renice, systemd, APT, red ni operaciones privilegiadas.

Linux sin --demo selecciona LinuxProvider. Otros SO conservan demo y explican
que el proveedor real requiere Linux. Tras la implementación y la prueba del
usuario en Ubuntu Server real, se autorizaron commits, PR y release v0.2.0 el
16 de noviembre de 2026. Publicar después de verificar CI en PR y main.

## Implementación incremental

1. Ampliar modelos/estado y MockProvider con procesos/discos; probar filtro,
   orden, selección e inspección sin reloj ni sistema real.
2. Añadir LinuxProvider con sysinfo 0.40.3, cfg Linux y features system/disk.
   Excluir GPU, comandos, entorno y tareas; no retener un FD por proceso.
3. Mover inicialización y consultas a un worker estándar con canales acotados y
   una consulta en vuelo. Salida de UI independiente de lecturas lentas.
4. Integrar CLI y TUI: DEMO o LINUX / SOLO LECTURA, tabla desplazable, filtro,
   orden, selección estable, detalles, CPU pendiente y errores legibles.
5. Ampliar pruebas Linux/PTY/SSH y CI; verificar fmt, Clippy, tests y release.
   Actualizar README, CHANGELOG y ACCEPTANCE_STAGE_1.md con evidencia efectiva.

Archivos: domain/application, mock/nuevo linux, main/runner/nuevo sampler,
input/render/hook de pánico, tests/scripts, manifiestos/lock, CI y documentación.
Se mantienen cuatro crates y núcleo independiente de sysinfo y terminal.

## Semántica y límites

- CPU global 0–100 %, proceso 100 % por núcleo y potencialmente mayor que 100 %.
  Centésimas de porcentaje enteras; NaN/infinito son ausencia. Primera muestra
  o proceso nuevo: pendiente. Refrescos antes del mínimo reutilizan caché.
- RAM usada: total menos disponible según sysinfo Linux; comparar con
  MemTotal/MemAvailable. Swap en bytes y uptime en segundos.
- Discos por montaje, sin sumar volúmenes potencialmente duplicados; marcar
  datos no disponibles. Refrescar discos a lo sumo cada 5 s.
- Limitar snapshots a 10000 procesos, 64 montajes y texto acotado; anunciar
  recorte. La lista incluye solo procesos visibles; hidepid/permisos pueden
  ocultar otros y no se puede afirmar cuántos faltan.
- Identidad PID + inicio: cerrar inspección si desaparece; un PID reutilizado
  nunca conserva la inspección anterior.
- UI comprueba respuestas sin bloqueo y espera eventos hasta 50 ms. Canales
  capacidad 1 y una petición en vuelo impiden acumulación y respuestas fuera de
  orden. Cancelación cooperativa; no esperar un worker bloqueado al salir.
- El lector de teclado bloquea en un hilo del proceso con cola de 64 eventos;
  la UI usa recv_timeout. La decisión y medición de la regresión de poll Unix
  están registradas en ADR 0002.

## Aceptación

- Demo Windows/Linux funcional, proveedor Linux como usuario normal y métricas
  comparadas con fuentes independientes. CPU inicial pendiente y posterior
  válida; proceso de test visible y estable por identidad.
- Filtrar/ordenar/inspeccionar sin operaciones administrativas; desaparición y
  reutilización de PID explícitas. Lectura lenta/error no bloquea UI ni salida.
- Tests en tamaños extremos, texto no confiable, filtro y detalles; conservar
  regresión SIGWINCH y restauración del terminal. fmt/Clippy/tests/release.
- Smoke PTY/SSH demo y real y mediciones de consumo/latencia con entorno. <30 MB
  es la preferencia del núcleo del charter, no garantía para cualquier host.
- Registrar lo ejecutado y lo pendiente; una VM completa sigue siendo distinta
  de un contenedor. No cerrar Stage 1 por inferencia si falta evidencia.

Referencias oficiales: https://docs.rs/sysinfo/0.40.3/sysinfo/ y sus fuentes,
revisadas antes de utilizar las interfaces concretas.
