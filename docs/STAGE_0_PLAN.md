# Plan de ServerUI Stage 0 v0.1.0

Estado: aprobado por el usuario el 16 de noviembre de 2026; implementación de Stage 0 autorizada. El árbol y los criterios de este documento describen la propuesta aprobada; los resultados efectivos se registran en `ACCEPTANCE_STAGE_0.md`.

## Fuente y análisis de alcance

Fuente: Project Charter ServerUI v1.0, 16 de noviembre de 2026, entregado por el usuario como documento oficial. Original: `C:/Users/Charm/Downloads/Project_Charter_ServerUI_Codex.docx`. Se revisó su contenido textual, incluidas las tablas. El archivo conserva el estado editorial «Propuesta para aprobación» y una fecha de aprobación pendiente; la instrucción directa del usuario establece su uso como fuente oficial. La primera entrega se limitó a planificación y AGENTS.md; después el usuario aprobó comenzar Stage 0. La sección de gobierno del charter no autoriza por sí sola ejecutar acciones fuera del pedido.

El directorio de trabajo estaba vacío y no era un repositorio Git al revisarlo. Inicializar Git y configurar la protección remota de main son actividades distintas; ninguna se ha realizado en esta fase.

El alcance general del producto no equivale al alcance de v0.1.0. Las secciones 6 y 12 delimitan Stage 0 a fundación, navegación, diseño TUI, modo demo y CI. La separación entre presentación, aplicación y adaptadores, los privilegios mínimos y la restauración de terminal son restricciones transversales desde el comienzo.

| Referencia del charter | Requisito para Stage 0 | Evidencia prevista |
| --- | --- | --- |
| 5 y S0-01 | Rust, Ratatui/Crossterm y límites entre capas | Workspace y dependencias revisables |
| S0-02 y 8 | Loop de eventos, teclado, resize y salida limpia | Pruebas de eventos, render y terminal |
| S0-03 | Dashboard, Procesos, Servicios y Ajustes provisionales | Navegación y buffers de pantalla |
| S0-04 y S0-05 | Contrato SystemProvider, MockProvider y métricas simuladas | `cargo run -- --demo`; pruebas del contrato |
| S0-06 y 11 | Pruebas automatizadas pertinentes | Tests deterministas de estado y render |
| S0-07 y 10 | CI y documentación | Workflow Linux/Windows, README y CHANGELOG |
| 2 y 8 | Bajo consumo, actualizaciones sin bloquear UI | Medición release y pruebas de respuesta |
| 7 y 9 | Demo segura y compatibilidad SSH | Verificación de aislamiento funcional y smoke SSH |

Quedan fuera las funcionalidades de Stage 1 y posteriores: métricas reales, listado o control de procesos reales, systemd, logs reales, APT, contenedores, administración de red/almacenamiento/usuarios, elevación de privilegios e historial persistente. Búsqueda global de acciones y detección real de capacidades se aplazan; no son necesarias para las cuatro pantallas provisionales. Ajustes muestra información y funciones pendientes, sin editor persistente.

Las decisiones que siguen son propuestas técnicas para revisión, no requisitos textuales adicionales del charter.

## Workspace propuesto

Workspace virtual, edición Rust 2024, resolver 3 explícito y `default-members` apuntando al binario para conservar `cargo run -- --demo`. Fijar toolchain estable concreto y MSRV después de comprobar compatibilidad de todas las dependencias. No se presume una versión de Rust ya instalada.

```text
ServerUI/
  AGENTS.md
  Cargo.toml
  Cargo.lock
  rust-toolchain.toml
  crates/
    serverui-core/
      Cargo.toml
      src/
        lib.rs
        domain/           # Datos demo, errores y contrato SystemProvider
        application/      # Estado, acciones, efectos y transiciones
    serverui-providers/
      Cargo.toml
      src/
        lib.rs
        mock.rs           # Único proveedor implementado en Stage 0
    serverui-tui/
      Cargo.toml
      src/
        lib.rs
        input.rs          # Teclas a acciones
        render.rs         # Estado a widgets
        terminal.rs       # Inicialización y restauración
    serverui/
      Cargo.toml
      src/
        main.rs           # CLI, composición y reporte final de errores
        runner.rs         # Eventos y despacho de efectos al proveedor
  docs/
    STAGE_0_PLAN.md
    ACCEPTANCE_STAGE_0.md
    adr/                  # Solo decisiones que requieran registro
  .github/workflows/ci.yml
  README.md
  CHANGELOG.md
```

El árbol representa la estructura aprobada. Los tests unitarios pueden estar junto a sus módulos; los tests de integración y fixtures pertenecen al crate cuyo contrato verifican. No crear subcrates adicionales sin un límite funcional que los justifique.

Dependencias internas permitidas: `serverui-providers → serverui-core`, `serverui-tui → serverui-core` y `serverui → core + providers + tui`. El núcleo no depende de los demás. Dominio y aplicación permanecen separados en módulos del núcleo para evitar otro crate prematuro.

Flujo: teclado/resize/tick → acción de aplicación → transición de estado y efecto tipado → runner → proveedor → resultado tipado → nueva transición → render. El renderizado nunca llama al proveedor. La composición selecciona explícitamente MockProvider; no hay fallback silencioso a datos reales.

## Contratos y ejecución

SystemProvider expone únicamente una consulta de snapshot de demostración y su resultado estructurado. El modelo representa un pequeño conjunto de métricas simuladas, identificadas como tales, suficiente para probar presentación y actualización. MockProvider usa fixtures y una secuencia determinista; permite escenarios normal, vacío y error sin depender del reloj del host.

No definir por adelantado métodos para instalar paquetes, reiniciar servicios o terminar procesos. En Stage 1 se revisará el contrato de lectura y se añadirá LinuxProvider detrás del mismo límite de adaptadores. Mantener tipos del núcleo independientes de sysinfo, D-Bus, Crossterm y estructuras específicas de Linux evita acoplar futuros proveedores a una biblioteca o al terminal.

El núcleo expresa estados de carga, disponible, vacío y error, con conservación explícita del último dato válido cuando corresponda. El runner entrega tiempo/eventos de manera inyectable. Las pantallas Procesos y Servicios anuncian su implementación pendiente; no muestran listas reales ni botones operativos engañosos.

Propuesta de CLI: `--demo`, `--help`, `--version`, `--ascii`, `--no-color` y `--refresh-ms` con validación y límites documentados. Refresco demo predeterminado de 1000 ms; rango propuesto de 100 a 60000 ms. Ejecutar sin `--demo` termina con mensaje de proveedor real pendiente y código no cero, antes de activar raw mode. No se crean archivos de configuración.

Usar `poll` de eventos con timeout hasta el siguiente tick y lectura solo cuando haya evento. Redibujar en cambios de estado/tamaño y actualización demo; no dibujar continuamente. Evitar starvation del tick durante ráfagas de teclado. Las consultas mock son inmediatas y acotadas: Stage 0 no necesita Tokio ni tareas de fondo. Antes de añadir I/O real lento, el runner deberá incorporar ejecución fuera del hilo de UI, cancelación, límites de cola y protección frente a respuestas obsoletas; no se implementa esa infraestructura especulativamente ahora.

Envolver el ciclo de vida de terminal para limpiar inicializaciones parciales, salida normal, errores y pánico con unwinding; restaurar antes de imprimir errores a stderr. Ctrl+C se trata como entrada de salida en raw mode. SIGKILL, panic=abort y pérdida abrupta de conexión no garantizan cleanup y se documentan. Entrada/salida sin TTY se rechazan con un error legible; `--help` y `--version` funcionan sin TTY.

## Interacción y compatibilidad

Propuesta: Tab/Shift+Tab o flechas izquierda/derecha cambian sección; teclas 1–4 seleccionan directamente; `?` abre ayuda, Escape la cierra, `q` y Ctrl+C salen. Mostrar sección activa, atajos y etiqueta DEMO permanentemente. Diseñar con caracteres simples y colores ANSI básicos; ofrecer ASCII y ausencia de color. El uso por SSH no incorpora cliente SSH ni conexión remota propia: ServerUI se ejecuta dentro de la sesión del usuario.

Diseño completo para 80×24; modo compacto para 40×12; por debajo, mensaje de tamaño insuficiente que conserve salida por teclado. Probar áreas de 0×0 y 1×1 sin pánicos. Texto largo se recorta de forma segura, sin exceder el área ni depender de glifos de ancho ambiguo. Ayuda y errores deben caber en el modo compacto.

## Dependencias propuestas

| Dependencia | Uso | Decisión de Stage 0 |
| --- | --- | --- |
| `ratatui` | Widgets, layout y TestBackend | Backend Crossterm; features mínimas |
| `crossterm` | Eventos, teclado, resize y terminal | Versión compatible con el backend elegido |
| `clap` | CLI, ayuda y validación de argumentos | Derive si mejora claridad; solo en binario |
| `thiserror` | Errores tipados de núcleo/adaptadores | Sin mensajes terminales en tipos de dominio |
| Biblioteca estándar | Tiempo, colecciones y resultados | Preferirla cuando cubra la necesidad |
| `insta` (solo desarrollo, opcional) | Snapshots legibles | Añadir solo si assertions de buffers no bastan |

Seleccionar y fijar versiones compatibles al implementar, verificando MSRV, features y grafo con Cargo. Centralizar requisitos en `[workspace.dependencies]` y guardar la resolución exacta en Cargo.lock. No afirmar versiones exactas sin resolverlas. Ratatui ha separado backends en versiones recientes: evitar una segunda versión incompatible de Crossterm; usar su reexport si la versión elegida lo ofrece o alinear la dependencia directa.

No se necesitan sysinfo, clientes D-Bus, serde/TOML, base de datos, Docker SDK, Tokio, async-trait, loggers a disco ni librerías SSH. Cada adición posterior debe justificar su coste y stage.

Referencias técnicas oficiales consultadas para estas propuestas:

- Workspace virtual y resolver: https://doc.rust-lang.org/stable/cargo/reference/workspaces.html
- Backends y compatibilidad de versiones: https://ratatui.rs/concepts/backends/
- Inicialización y restauración: https://docs.rs/ratatui/latest/ratatui/init/index.html
- Poll y lectura de eventos: https://docs.rs/crossterm/latest/src/crossterm/event.rs.html

## Implementación incremental tras revisión

Cada incremento entrega código, pruebas pertinentes y documentación revisable. Los nombres siguen el backlog del charter.

| Paso | Trabajo | Criterio de término |
| --- | --- | --- |
| S0-01 | Crear cuatro crates, límites, toolchain, lock y metadatos v0.1.0 | Compila workspace vacío en Linux y Windows; cargo run selecciona el binario |
| S0-04 | Modelos mínimos, contrato y MockProvider | Snapshots deterministas; escenarios vacío/error; núcleo independiente del SO |
| S0-02 | CLI, guard de terminal y loop de eventos | Demo inicia/sale; error de inicio y no-TTY limpios; help/version sin TTY |
| S0-03 | Cuatro pantallas, navegación, ayuda, resize y presentación accesible | Flujo completo por teclado y buffers correctos en tamaños acordados |
| S0-05 | Actualizaciones demo y manejo de errores del proveedor | Tick y error no bloquean navegación; reintento demo conserva coherencia |
| S0-06 | Consolidar tests de navegación, render y regresión de cleanup | Suite determinista; smoke PTY/SSH documentado con resultados |
| S0-07 | CI, README, CHANGELOG y checklist de aceptación | Checks en verde; instrucciones reproducibles y limitaciones registradas |

Las pruebas se incorporan en cada paso; S0-06 consolida cobertura de extremo a extremo. S0-07 puede arrancar con CI mínimo en S0-01 y completarse al final. No se inicia Stage 1 al terminar este backlog sin autorización explícita.

## Pruebas y aceptación

| Área | Criterio verificable | Medio |
| --- | --- | --- |
| Arranque | `cargo run -- --demo` inicia en Linux y Windows con TTY | Smoke documentado |
| CLI | Argumentos inválidos y sin demo dan error; help/version no necesitan TTY | Tests de integración del binario |
| Navegación | Todas las secciones accesibles; ida/vuelta, ayuda y salida coherentes | Tests de acciones y transiciones |
| Mock | Mismo escenario/eventos producen mismos datos; vacío/error/reintento son explícitos | Tests de contrato |
| Render | 80×24, 40×12, 20×5, 1×1 y 0×0 no causan pánico ni acceso fuera de límites | TestBackend y assertions/snapshots |
| Accesibilidad | Estado y atajos legibles sin color; ASCII usable | Tests de buffers y revisión de terminal |
| Eventos | Tick inyectado, resize repetido y ráfaga de teclas conservan respuesta y progreso | Tests con fuente de eventos falsa |
| Cleanup | Salida, error parcial y pánico con unwind intentan restaurar todas las propiedades adquiridas | Backend/guard instrumentado y smoke PTY |
| Terminal real | Shell utilizable, eco y cursor restaurados tras salida y fallos inducidos | PTY Linux y terminal Windows; fallo en subproceso de prueba |
| SSH | Navegación, resize y salida desde sesión SSH a Ubuntu Server sin privilegios | Prueba manual reproducible, registrar cliente y TERM |
| Seguridad | Demo usa exclusivamente mock, sin lectura de métricas ni procesos externos ni red/escrituras de aplicación | Revisión de dependencias/código y observación de ejecución Linux |
| Calidad | fmt, clippy sin warnings y tests aprobados; build release Linux | CI y comprobación local disponible |

Comandos previstos de CI, una vez exista Cargo.lock:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
```

Matriz Linux y Windows para compilación/tests, con fmt/clippy al menos en Linux. CI verifica el toolchain fijado; añadir comprobación de MSRV si se declara diferente. Fijar acciones de GitHub por SHA revisado, permisos de lectura mínimos y sin secretos para tests. No ejecutar builds privilegiados ni montar sockets del host. Las pruebas interactivas no se sustituyen por snapshots.

Para Stage 0 bastan mocks y validación de compilación/ejecución demo en Linux. Un contenedor Ubuntu sin privilegios puede verificar compatibilidad de build, pero no acredita systemd ni terminal/SSH. Una VM Ubuntu accesible por SSH sirve de entorno representativo para el smoke manual; no requiere acciones administrativas del producto.

Objetivos de rendimiento propuestos, sujetos a medir en un entorno documentado: RSS en reposo preferentemente inferior a 30 MB según el charter, primer frame dentro de 1 s y respuesta de navegación dentro de 100 ms en terminal local. El charter no fija los dos últimos umbrales. Medir binario release ya compilado, no cargo run ni memoria del compilador. Registrar SO, hardware, terminal, tamaño, refresco y método; separar latencia local de latencia de red SSH. Para memoria, informar RSS en bytes/MB y MiB sin confundir unidades, tras 60 s y durante 10 min de uso. Objetivo de CPU propuesto: menos del 1 % de un núcleo en reposo con refresco de 1 s; verificar además ausencia de espera activa y de crecimiento sostenido. Estos objetivos no son garantías universales; cualquier desviación requiere medición, explicación y decisión de revisión antes del cierre.

## Riesgos y decisiones para revisión

- Mantener cuatro crates evita acoplar terminal y Linux, a costa de algo de configuración inicial. Dominio/aplicación comparten crate mientras el tamaño no justifique separarlos.
- El proveedor mock prueba la composición, no la exactitud de métricas Linux. Esa validación pertenece a Stage 1.
- Las diferencias de teclado y render entre emuladores requieren pruebas reales; snapshots no demuestran restauración del terminal ni experiencia SSH.
- Las versiones exactas de dependencias, MSRV y presupuesto de rendimiento necesitan validación al implementar. La fase documental inicial no descargó crates ni generó un lock; la implementación aprobada sí los requiere.
- La licencia open source no está especificada; decidirla antes de distribución. No elegir una licencia ni publicar una release por inferencia.

La revisión aprobada cubre estructura de cuatro crates, CLI propuesta, comportamiento sin --demo, teclas/tamaños y objetivos medibles. Los resultados de implementación y pruebas se registran por separado en `ACCEPTANCE_STAGE_0.md`.

Para cerrar v0.1.0: demo verificable, CI satisfactoria, checklist con evidencia, README/CHANGELOG actualizados y revisión del alcance. Crear la etiqueta y publicar artefactos únicamente cuando se autorice el release; completar Stage 0 no autoriza etapas posteriores.
