# Changelog

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
