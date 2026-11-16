# ADR 0001 Sondeo de eventos Unix

Fecha: 16 de noviembre de 2026. Estado: adoptado en la implementación de Stage 0.

La prueba PTY Linux reproduce pérdida de entrada cuando SIGWINCH y teclado
coinciden con Crossterm 0.29.0 y su fuente Mio predeterminada. La demo continúa
actualizando, pero la tecla de salida queda sin procesar. La misma prueba sin
resize y el caso de una sola tecla finalizan correctamente.

La fuente Mio devuelve un evento antes de recorrer todos los tokens de una
notificación; el sondeo por flancos puede dejar entrada pendiente sin volver a
notificarla. Se elige la feature oficial `use-dev-tty`, que selecciona la fuente
Unix basada en sondeo de descriptores con `poll`. No se introduce una copia de
Crossterm, un lector concurrente ni un runtime asíncrono.

La feature se activa únicamente en la dependencia Unix del binario de
composición. Los contratos y el runner siguen usando la API multiplataforma de
Crossterm. Windows conserva su backend nativo. El nombre de la feature no
significa exigir /dev/tty: Crossterm usa stdin si ya está conectado a un TTY.

`scripts/pty_smoke.py` mantiene entrada y resize concurrentes como regresión;
verifica además restauración real del terminal. El script de aceptación no
modifica el host y usa un PTY privado. La feature incorpora una dependencia
transitiva de sondeo de descriptores, justificada por esta corrección.

Referencias oficiales:

Evolución en Stage 1: se conserva esta fuente Unix, pero su lectura usa un hilo
bloqueante en vez de poll con timeout corto; ver ADR 0002 para el motivo medido.

- https://docs.rs/crossterm/0.29.0/crossterm/#feature-flags
- https://docs.rs/crossterm/0.29.0/src/crossterm/event/source/unix/mio.rs.html
- https://docs.rs/crossterm/0.29.0/src/crossterm/event/source/unix/tty.rs.html
