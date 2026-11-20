# Aceptación de v1.0.0

Stage 5 consolida el comportamiento de v0.5.0. Ubuntu Server x86_64 es la
plataforma de operación y Ubuntu 22.04/glibc 2.35 es el baseline Linux.
Windows x86_64 ofrece modo demo. Terminal interactiva con teclado, SSH,
ASCII y sin color. El tamaño mínimo para confirmar operaciones es 40×12.

## Seguridad y compatibilidad

| Comprobación | Evidencia requerida |
| --- | --- |
| Aislamiento de la demo | PTY/SSH con demo y sin acceso a herramientas reales |
| Cambios administrativos | Unidad, plan o ID exactos, confirmación, cancelación, denegación, fallo y sin reintentos de mutaciones |
| Confirmación visible | Redimensionar bajo 40×12 y pulsar `y`; comprobar que el estado se conserva y repetir con el diálogo visible |
| Límites de ejecución | Fixtures de salida, plazo, entradas inválidas y canal de una petición en vuelo |
| Terminal | Salida, error, resize, pánico con unwinding y restauración |
| Rendimiento | Regresión de CPU/RSS de demo con refresco de 1000 ms |
| Compatibilidad | fmt, Clippy, tests, builds release en Ubuntu y Windows, y validación de glibc |
| Dependencias | Cargo.lock revisado con RustSec y tratamiento de avisos aplicables |
| Archivos de release | Artefactos de la CI de main, versión de ambos binarios y cinco archivos verificados por SHA-256 |

## Motores y autorización en Ubuntu

El recorrido de CI `scripts/engine_vm_smoke.py` está destinado al runner
Ubuntu 22.04 hospedado por GitHub. Crea un contenedor exclusivo y etiquetado
por motor a partir de una imagen fijada por digest. Comprueba desde la TUI
cancelar, iniciar, leer stdout y stderr, reiniciar y detener. Contrasta
estado y hora de inicio mediante el cliente real y retira solo sus fixtures.
Esta prueba está pendiente de ejecución en CI. El runner es una VM
desechable. [Servicios del runner](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2204-Readme.md) ·
[Aislamiento de los runners hospedados](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

La evidencia previa de servicios usa el gestor de usuario por SSH con la
excepción autorizada en Stage 2. La de APT usa apt/dpkg reales en sandbox y
autorización simulada. Las pruebas del gestor systemd del sistema y de
polkit real por SSH requieren un entorno aislado con permisos preparados.
[Aceptación de servicios](ACCEPTANCE_STAGE_2.md) ·
[Aceptación de APT](ACCEPTANCE_STAGE_3.md) ·
[Aceptación de contenedores](ACCEPTANCE_STAGE_4.md).

## Evidencia local de v1.0.0-dev

- Rust 1.98.1. Formato, Clippy con avisos como errores, 55 tests Windows y 67
  Ubuntu 22.04. Se compilaron los binarios release en ambas plataformas.
  El binario Linux requiere glibc 2.34. Pasó la prueba del empaquetador.
- La prueba PTY redujo la terminal a 36×10 durante la confirmación, pulsó
  `y` y verificó que el contenedor simulado seguía en ejecución. Al recuperar
  el tamaño, se volvió a proponer y confirmar la detención. La demo Windows
  confirmó un cambio visible y restauró la terminal.
- Pasaron cuatro recorridos SSH aislados, la regresión PTY de salida, error
  y pánico, la demo de servicios, las transacciones APT desechables y los
  clientes simulados de Docker y Podman con inventario real. El smoke de
  motores reales se rechazó fuera de un runner hospedado según lo previsto.
- En la muestra de demo de 15 s a refresco de 1000 ms, el contenedor local
  obtuvo 0,20 % de un núcleo, 3 629 056 bytes de RSS máximo y 0,383 ms para
  abrir la ayuda.
- El 20 de noviembre de 2026, la consulta por nombre y versión a OSV no
  devolvió avisos para los 112 paquetes de registro de Cargo.lock. Falta
  ejecutar también `cargo audit` en CI. [Consulta de OSV](https://google.github.io/osv.dev/post-v1-querybatch/).

## Publicación

Los resultados de CI y de los binarios de publicación se registran al cerrar
el PR. La licencia open source queda sujeta a la decisión del propietario.
Los límites de autorización probados deben constar en las notas de release.
