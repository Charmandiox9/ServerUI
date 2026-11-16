# Instalar ServerUI v0.2.0 en Ubuntu Server

La release v0.2.0 incluye Stage 1 para Ubuntu 22.04 x86_64, con baseline glibc
2.35. La versión anterior v0.1.0 contiene solamente demo. No se requiere
actualizar Ubuntu, instalar Rust ni ejecutar ServerUI como root para usarlo.

Antes de transferir, comprobar en el servidor:

```sh
cat /etc/os-release
uname -m
getconf GNU_LIBC_VERSION
```

Descargar el par de archivos de la release en el servidor:

```sh
cd ~
release_url='https://github.com/Charmandiox9/ServerUI/releases/download/v0.2.0'
archive='serverui-v0.2.0-x86_64-unknown-linux-gnu.tar.gz'
curl -fLO "$release_url/$archive"
curl -fLO "$release_url/$archive.sha256"
```

Como alternativa, desde PowerShell en la carpeta del proyecto, copiar el paquete
local ya verificado, reemplazando USUARIO e IP:

```powershell
scp .\target\stage1-packages\serverui-v0.2.0-x86_64-unknown-linux-gnu.tar.gz USUARIO@IP:~/
scp .\target\stage1-packages\serverui-v0.2.0-x86_64-unknown-linux-gnu.tar.gz.sha256 USUARIO@IP:~/
```

En la sesión Ubuntu:

```sh
cd ~
sha256sum -c serverui-v0.2.0-x86_64-unknown-linux-gnu.tar.gz.sha256 &&
mkdir -p ~/serverui-test-v0.2.0 &&
tar -xzf serverui-v0.2.0-x86_64-unknown-linux-gnu.tar.gz -C ~/serverui-test-v0.2.0 &&
mkdir -p ~/.local/bin &&
install -m 755 ~/serverui-test-v0.2.0/serverui ~/.local/bin/serverui
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
En el filtro q es texto; Ctrl+C siempre permite salir. Confirmar que el shell
recupera cursor y eco normales después de salir.

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
