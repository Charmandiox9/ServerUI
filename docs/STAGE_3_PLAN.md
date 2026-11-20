# Stage 3 Gestión de paquetes APT

El usuario autorizó continuar Stage 3 el 19 de noviembre de 2026. El alcance
se basa en el Project Charter v1.0, secciones 3, 5–11. El objetivo de versión es
v0.4.0.

## Entrega

- Buscar por nombre en la caché local de Ubuntu, instalar, actualizar un
  paquete instalado y eliminarlo. Mostrar simulación, confirmación, progreso y
  errores. Preparar la retirada de una instalación nueva de un solo paquete.
- Mostrar una selección de recomendaciones disponibles en los índices locales al
  abrir Paquetes y al enviar una búsqueda vacía.
- Mantener contratos y transiciones en `serverui-core`, adaptadores en
  `serverui-providers`, render y teclado en `serverui-tui`, workers y wiring en
  `serverui`. Reusar el worker de una petición en vuelo y la ejecución acotada
  de consultas. Conservar el demo sin I/O del host y la navegación existente
  con `1–4`. Consultar [ADR 0005](adr/0005-apt-transactions.md).
- Probar formato, Clippy, workspace y binarios release en Windows y Ubuntu.
  Verificar cancelación, autorización denegada, fallo por lock, instalación,
  progreso, recuperación, actualización y eliminación en un contenedor
  desechable. Conservar PTY/SSH, restauración y prueba de CPU en reposo.
- Actualizar README, CHANGELOG y [aceptación](ACCEPTANCE_STAGE_3.md). Publicar
  después de la aprobación de CI en PR y main y la autorización del usuario.
