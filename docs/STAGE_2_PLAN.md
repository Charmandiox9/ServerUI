# Stage 2 / objetivo v0.3.0

Autorizado por el usuario el 16 de noviembre de 2026, junto con mejoras visuales
sin degradar rendimiento. Fuente: Project Charter v1.0, secciones 3, 6–11,
leído del DOCX original. Stage 1 cerrado: PR #2 integrado en 7383750, CI Linux y
Windows aprobada y release v0.2.0 publicada (comprobados por API de GitHub).

## Incremento de implementación

- Núcleo: contratos de servicios, operaciones tipadas, confirmación vinculada a
  unidad/operación, estados comprobables y registro volátil acotado.
- Proveedores: mock determinista; adaptador Linux systemctl/journalctl sin shell,
  sudo, agente de autenticación ni privilegios adicionales. Solo servicios del
  sistema cargados; los logs son una captura manual de las últimas 80 líneas.
- Composición: worker de servicios separado del de métricas, inicialización
  diferida, una operación en vuelo, colas acotadas, salida sin join bloqueante.
- TUI: lista/filtro, logs desplazables, iniciar/detener/reiniciar/habilitar,
  confirmación explícita y errores legibles. Dashboard/procesos siguen de lectura.
- Visual: jerarquía, barras de capacidad, tabla y selección legibles, colores ANSI
  opcionales, ASCII y 40x12. Sin animaciones ni nuevos ticks de renderizado.

Archivos previstos: core/services, application, providers/services, nuevo worker,
runner/main, input/render, pruebas, CI, README, CHANGELOG y ADR 0003.
No añadir dependencias, APT, acciones sobre procesos, configuración persistente,
administración de red, discos o usuarios. No publicar sin nueva autorización.

## Verificación y cierre

fmt, Clippy, tests workspace, release Windows/Linux; demo en ambos sistemas,
PTY/SSH y benchmark antes/después. Pruebas puras: cancelación/confirmación,
identidad de unidad, no repetir acciones, saturación, error, timeout, filtrado,
logs acotados y neutralización de controles. Fixtures Linux de comandos sin
tocar servicios reales. VM Ubuntu con snapshot: unidad desechable, autorización
denegada, cancelación, ejecución, fallo, logs y recuperación. No usar servicios
SSH/red para pruebas. No cerrar Stage 2 hasta completar esa evidencia.

## Estado del incremento

Implementado localmente como v0.3.0-dev en `feat/stage-2-services`, sin nuevas
dependencias. fmt/Clippy/tests/release aprobados en Windows y Ubuntu 22.04;
49/53 pruebas respectivamente. PTY, recorrido demo de servicios, cuatro sesiones
SSH aisladas y benchmarks cortos aprobados. Ver cifras y límites en
`ACCEPTANCE_STAGE_2.md`. La confirmación física de Stage 1 no se reutiliza como
aceptación de Stage 2. El smoke físico exploratorio de demo/lectura pasó; en la
laptop el benchmark real incumplió el umbral de CPU y localizó el coste dominante
en el recolector. Ver evidencia en aceptación. Quedan pendientes el rendimiento
físico de CPU, la VM con snapshot, el smoke final de acciones y autorización de
publicación/CI remota.

Revisión de cierre del 16 de noviembre de 2026: el usuario autorizó commits, PR y
release con changelog si Stage 2 está completo. La condición aún no se cumple:
faltan aceptación de acciones en VM y rendimiento físico de CPU. Se conserva
esa autorización condicional; CI remota e integración se ejecutarán tras el cierre,
sin presentar v0.3.0-dev como una release terminada.

## Optimización y alternativa de aceptación autorizadas

El usuario autorizó optimizar el recolector y sustituir el requisito de VM por
una prueba controlada mediante SSH en USUARIO@IP_AUTORIZADA. Documentar la
excepción y los límites de recuperación; la autorización no implica permiso
genérico para mutar servicios existentes ni cambiar privilegios.

Archivos: proveedor Linux/nuevo parser de proc, probe de coste, integraciones
Linux, regresiones de CPU y documentación/ADR 0004. Restricciones: sin nuevas
dependencias ni unsafe, conservar refresco, identidad y semántica de CPU/RSS,
ninguna operación administrativa durante perfilado. Terminado: comparación en
laptop antes/después, fmt/Clippy/tests/release, PTY/SSH y evidencia efectiva.
Preparar aparte fixtures de servicios desechables y revisión de operaciones
antes de ejecutar cambios en el equipo físico.

## Estado posterior a optimización

Adoptado ADR 0004. CPU real de la laptop pasó de 3,9801 % a 0,8999 % de un núcleo
en muestras de 60 s, con el mismo refresco de 1000 ms. Smoke físico, PTY, Linux
57 tests, Windows 49 tests, fmt/Clippy/release y empaquetador aprobados. El fixture
de usuario y su guía están preparados; seis pruebas de sus salvaguardas pasaron
en Windows y Ubuntu. El modo plan ya se comprobó en la laptop, sin escribir unidades.

El usuario autorizó expresamente las acciones y cleanup. Los ocho casos reales
del gestor de usuario pasaron y la limpieza se confirmó independientemente;
siete tests de seguridad del fixture pasaron en Windows/Ubuntu. La alternativa
SSH y sus límites están registrados en aceptación. La condición funcional para
publicar está satisfecha; continuar commits, PR, CI, integración y release con
changelog según la autorización vigente. Stage 3 no está autorizado.
