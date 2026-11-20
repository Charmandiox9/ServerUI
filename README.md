# ServerUI

Administra Ubuntu Server desde una terminal local o por SSH. Consulta métricas,
procesos, red y discos, y opera servicios, paquetes APT y contenedores con
confirmación explícita.

[Descargar v1.0.0](https://github.com/Charmandiox9/ServerUI/releases/tag/v1.0.0) ·
[Instalar en Ubuntu y verificar SHA-256](docs/INSTALL_UBUNTU.md)

## Inicio rápido

- **Ubuntu x86_64** · Ejecuta `./serverui` como usuario normal. El binario usa
  glibc 2.35 como referencia de compatibilidad (Ubuntu 22.04).
- **Windows x86_64** · Ejecuta `.\serverui.exe --demo` en PowerShell.
- **Desde el código** · Con Rust 1.98.1 y una terminal interactiva,
  `cargo run --locked -- --demo` funciona en Ubuntu y Windows.

La demo usa datos simulados. Para terminales limitados, añade
`--ascii --no-color` o usa `NO_COLOR` para desactivar los colores.

## Controles

Usa `Tab` y `Shift+Tab` para cambiar de sección, `j/k` o las flechas para
moverte, `?` para ayuda y `q` o `Ctrl+C` para salir.

| Tecla | Sección | Operaciones |
| --- | --- | --- |
| `1` | Dashboard | CPU, memoria, swap, tiempo activo y capacidad por montaje. |
| `2` | Procesos | `/` filtra, `s/S` cambia el orden y Enter abre el detalle. |
| `3` | Servicios | Enter abre el journal. `a/d/R/e` propone iniciar, detener, reiniciar o habilitar una unidad cargada. |
| `4` | Ajustes | Consulta las opciones de la sesión. |
| `5` | Paquetes | Recomendados de los índices locales. `/` busca, Enter con búsqueda vacía vuelve a recomendados y `i/u/d` propone instalar, actualizar o eliminar. |
| `6` | Contenedores | `e` alterna Docker y Podman. Enter muestra logs y `a/d/R` propone iniciar, detener o reiniciar. |
| `7` | Red y discos | `n` muestra interfaces e IP, `b` dispositivos de bloque y `m` montajes. Inventario de solo lectura. |

`r` actualiza o reintenta. Revisa la unidad, el plan APT o el ID del
contenedor y pulsa `y` para confirmar o `n/Esc` para cancelar. La confirmación
requiere un terminal de al menos **40×12**.
Durante una operación APT, ServerUI espera el resultado antes de salir.

## Permisos en Linux

ServerUI funciona con los permisos de tu sesión. Los servicios usan systemd
y el journal del sistema. Docker usa `/var/run/docker.sock`; Podman usa su
almacenamiento local. APT consulta índices locales y requiere autorización
mediante `pkexec` para aplicar cambios. En SSH, la autorización de polkit
puede requerir un agente en otra terminal, como `pkttyagent --process PID`.

Si una acción queda sin resultado por un timeout o una desconexión, consulta
el estado antes de repetirla. Para el alcance de pruebas y recuperación,
consulta la [lista de aceptación](docs/ACCEPTANCE_STAGE_5.md).

## Desarrollo

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
```

[Changelog](CHANGELOG.md) · [CI](.github/workflows/ci.yml) ·
[Plan de estabilización](docs/STAGE_5_PLAN.md)
