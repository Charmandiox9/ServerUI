# ADR 0003 Servicios y comandos acotados

Fecha: 16 de noviembre de 2026. Stage 2 autorizado; aceptación pendiente.

Se elige systemctl/journalctl, disponibles en Ubuntu 22.04, en lugar de añadir
un runtime async y un cliente D-Bus. El núcleo define unidades validadas y cuatro
operaciones: start, stop, restart, enable. La presentación solo emite acciones.
Argumentos separados, ejecutables absolutos /usr/bin, stdin null, sin shell,
sudo ni preguntas de credenciales (--no-ask-password). Los permisos existentes
deciden si se autoriza; denegación se informa sin elevar la TUI.

Un worker diferido de servicios mantiene una petición en vuelo, independiente
del recolector de métricas. Consultas solo al entrar/refrescar explícitamente;
logs bajo demanda, sin follow. Salidas limitadas en bytes y tiempo; lectores
acotados, sin esperar joins desde la UI. Se mata el cliente al vencer el plazo,
pero esto NO revierte ni cancela un job ya enviado a systemd. Nunca reintentar
automáticamente una mutación. La salida indica resultado incierto tras timeout.

La confirmación captura unidad y operación; navegación y otras acciones quedan
capturadas hasta y (una pulsación, no repetición) o Escape/n. Se advierte impacto
potencial sobre dependencias, disponibilidad y SSH. Registro solo en memoria,
16 resultados, sin logs de entorno ni diagnósticos crudos de comandos fallidos.
No se cambia la política polkit ni se ejecutan acciones reales en desarrollo.

Listado limitado a servicios cargados (list-units --all), no a todos los archivos
instalados. Unidad ASCII restringida, máximo 256 bytes; se rechazan patrones,
rutas y nombres escapados. Logs limitados a 80 líneas/256 KiB y texto visible
neutralizado; permisos pueden ocultar el journal. Éxito del comando no garantiza
salud futura del servicio. Enable no inicia el servicio ni es rollback automático.

Referencias oficiales consultadas: systemctl(1) y journalctl(1),
https://www.freedesktop.org/software/systemd/man/latest/systemctl.html y
https://www.freedesktop.org/software/systemd/man/latest/journalctl.html.
