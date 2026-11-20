# Aceptación de Stage 4

Versión v0.5.0. El smoke de demo recorre Docker simulado,
Podman simulado, cancelación, logs, red, bloque y montajes. La integración
Ubuntu ejercita el binario Linux contra clientes desechables que validan los
argumentos y simulan la aceptación o denegación, y consulta el inventario
real visible en el contenedor.

## Reproducir

Tras compilar el binario Linux, ejecuta:

```sh
python3 scripts/infrastructure_smoke.py target/release/serverui
docker build --tag serverui-ssh-smoke:local - < scripts/Dockerfile.ubuntu-smoke
docker run --rm --network none --cap-drop ALL \
  --security-opt no-new-privileges --env SERVERUI_INFRA_SANDBOX=disposable \
  --mount "type=bind,source=$PWD,target=/workspace,readonly" \
  serverui-ssh-smoke:local python3 /workspace/scripts/infrastructure_smoke.py \
  /workspace/target/release/serverui --sandbox
```

El fixture requiere un contenedor desechable cuyos clientes Docker y Podman
estén ausentes. Sirve para comprobar el adaptador, el contrato de comandos
y el flujo completo de terminal. Las mutaciones contra motores reales,
incluidos el daemon Docker y Podman rootless, requieren una VM desechable
con permisos previamente configurados y contenedores dedicados.

## Resultados locales

- Rust 1.98.1. Formato, Clippy con warnings como errores, compilación release
  y 55 tests en Windows y 67 en Ubuntu 22.04. El binario Linux requiere
  glibc 2.34, dentro del baseline 2.35.
- La demo Windows verificó ambos motores, cancelación, inicio, las tres vistas
  de inventario y restauración del terminal. El smoke PTY de Ubuntu comprobó
  además la captura de logs. La confirmación compacta 40×12 conserva el ID
  completo y el nombre del motor.
- Pasaron los recorridos Linux contra los dos clientes desechables. Se
  contrastaron los argumentos exactos, la denegación, el ID desaparecido,
  la confirmación única y ambos flujos de logs. Se comprobó el inventario
  real en Ubuntu 24.04 con `iproute2` y en Ubuntu 22.04 con fallback sysfs,
  sin montar sockets del host.
- Pasaron la regresión PTY, cuatro sesiones SSH aisladas, la demo de servicios
  y el ciclo APT con APT/dpkg reales y autorización simulada. La demo en
  Ubuntu usó 0,20 % de un núcleo, 3 510 272 bytes de RSS máximo y 0,496 ms de
  respuesta a ayuda en una muestra de 15 s con refresco de 1000 ms.

El entorno rechazó el namespace de usuario requerido por Podman rootless.

## Prueba por SSH en la laptop del usuario

El 20 de noviembre de 2026, el usuario confirmó que ServerUI funciona en su
laptop Ubuntu por SSH. La captura de `v0.5.0-dev` muestra el listado de Docker
local con contenedores reales en estado `running`.

Queda por confirmar si en esa sesión se probaron la cancelación, el inicio,
el reinicio, la detención y los logs con un contenedor desechable por motor.
Para registrar ese ciclo, indicar qué motores se probaron, qué resultado
mostró cada acción y si se retiraron los contenedores de prueba.
