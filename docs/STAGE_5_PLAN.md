# Stage 5 Estabilización de ServerUI

El usuario autorizó Stage 5. El Project Charter v1.0 define como entregables
estabilización, documentación y publicación con revisión de seguridad,
compatibilidad y pruebas. La versión de trabajo es v1.0.0-dev.

## Alcance

- Corregir confirmaciones administrativas invisibles tras reducir el tamaño
  del terminal. El runner y el render comparten el mínimo de 40×12.
- Ampliar el plazo de simulación de APT tras medir 13,371 s para resolver la
  retirada de un paquete de prueba en Ubuntu 22.04 por SSH.
- Conservar los módulos publicados. Añadir la regresión de confirmación a las
  pruebas PTY y un recorrido completo con Docker y Podman en Ubuntu hospedado.
  Usar solo contenedores dedicados, sin montar sockets, alterar la red del host
  o cambiar políticas de privilegios.
- Incorporar el análisis de avisos RustSec a CI. Validar contra Cargo.lock
  y documentar los resultados y límites.
- Actualizar README, CHANGELOG y el checklist de v1.0.0. La publicación requiere
  autorización del usuario, CI en el PR y en main, archivos generados por CI
  y sus SHA-256 comprobados.

El [checklist de aceptación](ACCEPTANCE_STAGE_5.md) distingue resultados
observados de los recorridos que deben ejecutarse fuera del entorno local.
