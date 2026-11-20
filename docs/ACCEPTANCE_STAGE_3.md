# Aceptación de Stage 3 APT

Versión de trabajo v0.4.0. Pruebas locales de desarrollo y validación en
contenedor Ubuntu. La release v0.4.0 requiere revisión, CI y autorización.

## Reproducir

En Linux, compila `cargo build --release --locked` y ejecuta la demo PTY
con `python3 scripts/apt_smoke.py target/release/serverui`. En Windows abre
`target\release\serverui.exe --demo`, pulsa `5` y busca `demo`.

Para validar mutaciones, usar únicamente un contenedor desechable sin red,
sin socket Docker y con el repositorio montado en solo lectura. Desde Linux:

```sh
docker build --tag serverui-ssh-smoke:local - < scripts/Dockerfile.ubuntu-smoke
docker run --rm --network none --cap-drop ALL \
  --cap-add CHOWN --cap-add DAC_OVERRIDE --cap-add FOWNER \
  --cap-add SETUID --cap-add SETGID \
  --security-opt no-new-privileges -e SERVERUI_APT_SANDBOX=disposable \
  --mount "type=bind,source=$PWD,target=/workspace,readonly" \
  serverui-ssh-smoke:local python3 /workspace/scripts/apt_smoke.py \
  /workspace/target/release/serverui --sandbox
```

El fixture construye dos versiones de un paquete sin dependencias desde una
fuente APT de archivo local. Comprueba el estado y la versión con `dpkg-query`.
Simula la aprobación y denegación en el límite de `pkexec`. Las consultas,
el progreso y las operaciones de APT/dpkg usan los ejecutables reales dentro
del contenedor. Las pruebas de autorización polkit real y del agente SSH
quedan fuera de este fixture.

## Resultados locales

- Windows con Rust 1.98.1. Formato, Clippy con warnings como errores, 51 tests
  y compilación release aprobados. La demo interactiva de terminal buscó
  `demo-editor`, mostró la simulación, confirmó la instalación simulada y
  restauró el cursor y la pantalla alternativa al salir.
- Ubuntu 22.04 con Rust 1.98.1. Formato, Clippy, 61 tests y compilación release
  aprobados. El binario requiere glibc 2.34, dentro del baseline 2.35.
- Pasaron el smoke PTY de demo APT y las regresiones de systemd, restauración,
  resize y teclado. Cuatro sesiones SSH aisladas validaron demo y lectura
  como usuario normal, con restauración del terminal.
- En Ubuntu 22.04 aislado, APT instaló la versión 1, la retiró con deshacer,
  reinstaló, actualizó a la versión 2 y la eliminó. También pasaron cancelación,
  denegación, error por lock, progreso y navegación concurrente. El mismo
  recorrido pasó con el binario release en la imagen Ubuntu de las pruebas SSH.
- Regresión en demo de 15 s con refresco de 1000 ms: 0,20 % de un núcleo,
  3 604 480 bytes de RSS máximo y 0,734 ms de respuesta a ayuda. La medición
  corresponde al contenedor local.
- Con la vista de recomendados, pasaron formato, Clippy, compilación release
  y 52 tests en Windows y 62 en Ubuntu 22.04. La demo PTY y la consulta real
  de solo lectura en Ubuntu verificaron la lista inicial, la búsqueda por
  nombre y el retorno a los recomendados al enviar una búsqueda vacía. El
  mismo recorrido de demo se comprobó en la terminal de Windows.
