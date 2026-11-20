# Instalar ServerUI v1.0.0 en Ubuntu Server

ServerUI v1.0.0 se ejecuta en Ubuntu 22.04 x86_64 con glibc 2.35. Usa el
binario de la release desde una sesión de usuario normal.

Antes de transferir, comprobar en el servidor:

```sh
cat /etc/os-release
uname -m
getconf GNU_LIBC_VERSION
```

Descargar el par de archivos de la release en el servidor:

```sh
cd ~
release_url='https://github.com/Charmandiox9/ServerUI/releases/download/v1.0.0'
archive='serverui-v1.0.0-x86_64-unknown-linux-gnu.tar.gz'
curl -fLO "$release_url/$archive"
curl -fLO "$release_url/$archive.sha256"
```

Como alternativa, desde PowerShell en la carpeta del proyecto, copiar el paquete
local ya verificado, reemplazando USUARIO e IP:

```powershell
scp .\target\release-assets\serverui-v1.0.0-x86_64-unknown-linux-gnu.tar.gz USUARIO@IP:~/
scp .\target\release-assets\serverui-v1.0.0-x86_64-unknown-linux-gnu.tar.gz.sha256 USUARIO@IP:~/
```

En la sesión Ubuntu:

```sh
cd ~
sha256sum -c serverui-v1.0.0-x86_64-unknown-linux-gnu.tar.gz.sha256 &&
mkdir -p ~/serverui-test-v1.0.0 &&
tar -xzf serverui-v1.0.0-x86_64-unknown-linux-gnu.tar.gz -C ~/serverui-test-v1.0.0 &&
mkdir -p ~/.local/bin &&
install -m 755 ~/serverui-test-v1.0.0/serverui ~/.local/bin/serverui
```

Continuar solo si el checksum dice OK. Probar primero versión/demo y después
lectura real; no usar sudo:

```sh
~/.local/bin/serverui --version
~/.local/bin/serverui --demo --ascii --no-color
~/.local/bin/serverui --ascii --no-color
```

Verificar Dashboard, 2 para Procesos, / para filtrar, Enter para terminar el
filtro y abrir detalle, s para ordenar, Escape para cerrar y q para salir.
En el filtro q es texto; Ctrl+C sale cuando termina cualquier operación APT en curso. Confirmar que el shell
recupera cursor y eco normales después de salir.

Servicios (3) permite consultas systemd y journal. a/d/R/e proponen iniciar,
detener, reiniciar o habilitar la unidad seleccionada; solo y ejecuta y n/Escape
cancela. Usar los permisos existentes del usuario, sin sudo ni elevar toda la
TUI. Revisar unidad e impacto antes de confirmar: puede afectar dependencias y
SSH. Un timeout no garantiza cancelar un job aceptado por systemd; comprobar
estado antes de reintentar. Las pruebas desechables están en docs/SERVICE_ACCEPTANCE_SSH.md.

Paquetes (5) muestra los recomendados disponibles y permite buscar con `/`. Instalar, actualizar o eliminar requiere revisar la simulación, confirmar y obtener autorización mediante polkit. Consulta [Paquetes APT](../README.md#permisos-en-linux) para el flujo por SSH.

Contenedores (6) usa Docker o Podman locales con los permisos existentes.
Red y discos (7) muestra el inventario de solo lectura. Consulta el
[recorrido de Stage 4](../README.md#controles).

## Diagnóstico de formato y compatibilidad

Un mensaje que termina el nombre en `$'\r'` al verificar el checksum indica que
el manifiesto contiene CRLF. El empaquetador ahora escribe LF explícito. Volver
a copiar el par nuevo completo; el paquete anterior también podía requerir una
glibc demasiado reciente, por lo que corregir solo el texto no basta.

Un error GLIBC_x.y not found indica un binario construido con un baseline
posterior. No sustituir libc ni actualizar el servidor para sortearlo: generar
el binario con `scripts/Dockerfile.ubuntu22-build` o el runner Ubuntu 22.04 de CI.
`scripts/check_linux_abi.py` verifica que ninguna dependencia GLIBC supere 2.35.

Para desarrollo, la imagen de compilación instala herramientas solo dentro de
Docker y ejecuta builds como UID 1000 con el proyecto de solo lectura. La prueba
SSH usa un contenedor aislado como fixture; no cambia el SSH del servidor real.
