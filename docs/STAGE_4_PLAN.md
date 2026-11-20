# Stage 4 Contenedores red y discos

El usuario autorizó Stage 4. El Project Charter v1.0 incluye operaciones
comunes de Docker y Podman, red y almacenamiento inicial, con pruebas de
integración y salvaguardas. La versión es v0.5.0.

## Entrega

- Contenedores locales con selección de motor, lista, logs, iniciar, detener
  y reiniciar. Confirmar cada cambio con motor, operación e ID completo.
- Red y discos inicialmente en solo lectura. Mostrar interfaces y direcciones
  IP, inventario de bloque y montajes con capacidad. Reservar cambios de red,
  particionado, formateo y montaje para un alcance autorizado por separado.
- Contratos y estado en el núcleo, adaptadores Linux/demo en proveedores,
  entradas y vistas en la TUI y composición mediante el worker existente.
  Mantener las teclas 1–5 y la navegación compacta. Ver
  [ADR 0006](adr/0006-local-infrastructure.md).
- Verificar formato, Clippy, tests, builds y demo en Windows/Ubuntu.
  Añadir cobertura PTY de cancelación, confirmación, ID exacto, denegación,
  stdout/stderr e inventario Linux. Usar un contenedor sin red ni sockets del
  host. Registrar qué se probó con fixtures y qué requiere motores reales.
