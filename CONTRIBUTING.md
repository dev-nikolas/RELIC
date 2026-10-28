# Guia de Contribucion para RELIC

Agradecemos el interes en contribuir a RELIC. Este proyecto mantiene estandares de ingenieria estrictos para preservar la claridad, el rendimiento en Rust puro y la reproducibilidad matematica.

---

## 1. Principios de Diseno del Proyecto

1. **Regla Estricta de Lineas:** Ningun archivo fuente (`.rs`, `.py`) debe exceder las **180 lineas de codigo**. La modularidad y el principio de responsabilidad unica son obligatorios.
2. **Cero Dependencias en Inferencia:** El crate `relic_core` no debe incorporar librerias externas pesadas de deep learning. Toda operacion debe implementarse en Rust seguro y estandar.
3. **Paridad y Verificacion Formal:** Cualquier operador nuevo o cambio en la arquitectura debe contar con pruebas unitarias asociadas que validen su estabilidad numerica.

---

## 2. Flujo de Trabajo para Pull Requests

1. **Crear una rama descriptiva:**
   ```bash
   git checkout -b feature/nombre-de-la-mejora
   ```

2. **Validar la suite de pruebas unitarias:**
   Antes de abrir un PR, asegurate de que todas las pruebas pasen en local:
   ```bash
   cargo test --workspace
   ```

3. **Verificar formato y lints:**
   ```bash
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   ```

4. **Auditar longitud de archivos:**
   Verifica que ningun archivo nuevo o modificado supere las 180 lineas:
   ```powershell
   Get-ChildItem -Recurse -Include *.rs,*.py | Select-Object Name, @{Name="Lines";Expression={(Get-Content $_.FullName | Measure-Object -Line).Lines}}
   ```

---

## 3. Estilo de Codigo

* **Rust:**
  * Usar tipos explicitos en firmas publicas.
  * Manejo de errores idiomatico con `Result` en lugar de `panic!` o `unwrap()` no controlados en la ruta caliente.
  * Documentar cada estructura y funcion publica con docstrings (`///`).
* **Python:**
  * Type hints obligatorios en todas las funciones.
  * Manejo defensivo de importaciones relativas y absolutas con bloques `try/except ImportError`.

---

## 4. Licencia de Contribuciones y Atribución Intelectual

Todas las contribuciones enviadas seran licenciadas bajo los terminos de la **Licencia AGPL-3.0 con Clausulas Adicionales de Atribucion Directa y Notificacion Previa al Autor Original (dev-nikolas)**.
Al colaborar en este repositorio, se reconoce de manera vinculante que:
- El merito conceptual, formulacion teorica y arquitectura base de RELIC corresponden a dev-nikolas.
- Ningun colaborador ni tercero podra redistribuir o compartir derivados sin notificar previamente al autor y asegurar la atribucion explicita de la idea y del codigo.
