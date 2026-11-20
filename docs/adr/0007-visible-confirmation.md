# ADR 0007 Confirmación visible en terminal

Stage 5, v1.0.0-dev.

Al quedar por debajo de 40×12, ServerUI muestra el aviso de tamaño en lugar
de las vistas. El runner cancela las confirmaciones pendientes con un resize
por debajo del mínimo. Cuando llega una tecla de confirmación de servicios,
APT o contenedores, consulta además el tamaño real del terminal. Una lectura
fallida cancela la confirmación. Así, el envío de una acción no depende de
que el evento de resize ya se haya procesado.

El render y el runner comparten el mínimo. Se consulta el tamaño al
confirmar, por lo que el refresco periódico conserva la misma ruta de
ejecución. La cancelación afecta al diálogo pendiente. Un trabajo que ya se
envió conserva los límites y reglas de cada proveedor.
