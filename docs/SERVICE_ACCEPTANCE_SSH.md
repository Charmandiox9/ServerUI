# Prueba de acciones por SSH sin VM

El usuario sustituyó el requisito de VM con snapshot por pruebas controladas
en `USUARIO@IP_AUTORIZADA`. La excepción se registra: no existe rollback
integral del equipo. La recuperación se limita a las unidades y archivos de
fixture; los registros del journal permanecen.

## Alternativa de menor privilegio preparada

El equipo tiene systemd del usuario disponible en `/run/user/1000/bus`. Se
comprobó, solo mediante consultas, que `DBUS_SYSTEM_BUS_ADDRESS` permite dirigir
las llamadas systemctl de ServerUI a ese gestor. Esta prueba usa systemd real,
pero sus servicios pertenecen al usuario: no demuestra autorización polkit del
gestor del sistema ni habilitación al boot del host. Los logs reales del sistema
ya se probaron en lectura; `journalctl --system` no se redirige al gestor de usuario.

No se ejecuta ServerUI como root ni se conceden permisos polkit generales. En
Ubuntu 22.04, habilitar servicios requiere `manage-unit-files`, distinto de
`manage-units`; no se asume que ese permiso pueda limitarse a una unidad.

El script [service_acceptance_fixture.py](../scripts/service_acceptance_fixture.py)
prepara exclusivamente, sin sobrescribir rutas existentes:

- `~/.config/systemd/user/serverui-acceptance.service`: sleep inofensivo.
- `serverui-acceptance-fail.service`: termina deliberadamente con error.
- `serverui-acceptance-slow.service`: inicio con espera de 8 s para probar el
  timeout de 5 s del cliente, seguido de sleep inofensivo.
- `serverui-acceptance.target` y su directorio `.wants`, sin enlazar el target
  a ningún target de arranque/login. El enable de la prueba no añade autostart real.
- Manifiesto `/tmp/serverui-acceptance-1000.json` con hashes para cleanup.

El modo predeterminado `plan` no escribe nada. Revisar y autorizar estas
operaciones antes de ejecutar `prepare` en el equipo físico.

## Recorrido automatizado de aceptación

En una sesión SSH del usuario normal, una vez autorizada la preparación:

```sh
cd /tmp/serverui-smoke.QGEm9ENT
python3 service_acceptance_fixture.py plan
python3 service_acceptance_fixture.py prepare
python3 service_real_smoke.py /tmp/serverui-opt.1jT64OLH/serverui
```

El harness requiere `python3-dbus`, ya disponible en el equipo autorizado; no lo
instala. Mantiene referencias `RefUnit` durante la prueba para que systemd no
descargue las unidades inactivas antes de listarlas. Esas referencias se liberan
al cerrar su conexión D-Bus. Aplica la variable indicada abajo solo al proceso
ServerUI, valida el gestor de usuario y realiza cleanup en el bloque final.

La variable `DBUS_SYSTEM_BUS_ADDRESS=unix:path=/run/user/1000/bus` afecta únicamente
a ese proceso y sus hijos; no se exporta ni cambia
el gestor del sistema. En Servicios, `/serverui-acceptance` limita visualmente
la lista. Esc/Enter termina el filtro. Realizar esta secuencia solo sobre fixtures:

| Caso | Acción y comprobación independiente |
| --- | --- |
| Cancelar | Proponer iniciar y pulsar n. El servicio sigue inactive y el historial vacío. |
| Iniciar | a, revisar unidad, y. `systemctl --user is-active serverui-acceptance.service` devuelve active. |
| Reiniciar | Registrar MainPID, R/y y comparar un PID distinto con estado active. |
| Detener | d/y. Estado inactive. |
| Denegación de escritura | Con el servicio detenido y sin enlaces, ejecutar `lock-enable`; e/y debe fallar con error controlado. `is-enabled` sigue disabled. Luego `unlock-enable`. Esto prueba EACCES del filesystem, no polkit del gestor del sistema. |
| Habilitar | e/y tras desbloquear. `systemctl --user is-enabled serverui-acceptance.service` devuelve enabled; is-active sigue inactive. |
| Fallo del servicio | Seleccionar la unidad -fail, a/y. Mostrar error legible; `systemctl --user show -p Result serverui-acceptance-fail.service` informa exit-code. |
| Timeout | Seleccionar -slow, a/y. La UI sigue navegable, informa resultado incierto y no reintenta. Después de 8 s comprobar active con consulta independiente. Salir de la TUI no garantiza cancelar el job. |
| Historial/salida | h muestra operaciones y resultados; q sale restaurando terminal. |

Consultas independientes útiles (no usar sudo):

```sh
systemctl --user show -p ActiveState -p MainPID -p UnitFileState serverui-acceptance.service
python3 service_acceptance_fixture.py lock-enable
python3 service_acceptance_fixture.py unlock-enable
```

Guardar versión/hash del binario, fecha, UID, estado antes/después, cancelación,
errores y resultado de restauración. No copiar journal o diagnósticos sensibles.

## Recuperación

Salir de ServerUI y ejecutar, como el mismo usuario:

```sh
python3 service_acceptance_fixture.py cleanup
```

Verifica hashes y propietarios, detiene/deshabilita solo las tres unidades
creadas, elimina sus archivos/enlaces y recarga el gestor del usuario. Se niega
a borrar archivos modificados o desconocidos. No elimina journal, configuración
SSH, servicios ajenos ni una instalación existente de ServerUI. Ante discrepancia,
revisar antes de continuar; no ejecutar limpieza recursiva de ~/.config.

Para considerar cerrado Stage 2, registrar expresamente qué se aceptó de esta
alternativa y qué no se probó del gestor del sistema. Hasta entonces, la VM
histórica queda sustituida como plan, sin presentar estas pruebas como ya realizadas.

## Resultado observado

El 16 de noviembre de 2026 el usuario autorizó explícitamente preparar, ejecutar
y retirar estas unidades en su equipo por SSH. El recorrido automatizado pasó
los ocho casos de la tabla, incluida UI responsiva durante el inicio lento,
resultado incierto a los 5 s y job completado posteriormente. Cleanup completado;
una consulta SSH independiente confirmó ausencia de los cuatro archivos, directorio
de enlaces y manifiesto. No se modificaron polkit ni servicios del sistema.

El primer intento detectó GC de unidades inactivas y un reset-failed innecesario.
Se corrigió el harness mediante RefUnit y la limpieza para resetear solo unidades
fallidas de la allowlist; el segundo recorrido pasó. No se cambió ServerUI por
estos ajustes de fixture.

Referencias oficiales: [systemd 249 / seguridad D-Bus](https://www.freedesktop.org/software/systemd/man/249/org.freedesktop.systemd1.html),
[systemd.service](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html).
