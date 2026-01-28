# 🔀 gix - Git Profile Manager

<div align="center">

![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)

**Gestiona múltiples identidades de Git con facilidad**

Cambia entre perfiles de trabajo, personal y open source con diferentes claves SSH y configuraciones de usuario de manera fluida.

[Instalación](#-instalación) •
[Uso Rápido](#-uso-rápido) •
[Comandos](#-comandos) •
[Configuración](#-configuración) •
[FAQ](#-faq)

</div>

---

## ✨ Características

- 🔐 **Múltiples Perfiles**: Gestiona identidades separadas para trabajo, personal, open source
- 🔑 **Autenticación Flexible**: Soporta claves SSH y tokens HTTPS
- 🎯 **Detección Automática**: Detecta automáticamente el perfil configurado por repositorio
- 🔄 **Intercepción de Comandos**: Intercepta comandos git (push, pull, fetch) para aplicar el perfil correcto
- 📊 **Diagnósticos**: Comando `doctor` para verificar tu configuración
- 🔄 **Auto-actualización**: Verifica y actualiza a nuevas versiones fácilmente

## 📦 Instalación

### Opción 1: Script de Instalación (Recomendado)

```bash
curl -fsSL https://raw.githubusercontent.com/elmanci2/gix/refs/heads/master/install.sh | bash
```

### Opción 2: Con Cargo

Si tienes Rust instalado:

```bash
cargo install --git https://github.com/elmanci2/gix
```

### Opción 3: Desde Código Fuente

```bash
git clone https://github.com/elmanci2/gix.git
cd gix
cargo build --release
# Copiar binario a tu PATH
cp target/release/gix ~/.local/bin/
```

### Verificar Instalación

```bash
gix version
```

## 🚀 Uso Rápido

### 1. Crear tu primer perfil

```bash
gix profile add
```

Sigue las instrucciones interactivas:
- Nombre del perfil (ej: "Trabajo", "Personal")
- Nombre de usuario Git
- Email de Git
- Método de autenticación (SSH o Token)
- Seleccionar/crear clave SSH

### 2. Configurar un repositorio

```bash
cd tu-repositorio
gix use
```

Selecciona el perfil a usar. gix configurará el repositorio automáticamente.

### 3. Usar git normalmente

```bash
gix push
gix pull
gix fetch
```

gix interceptará estos comandos y aplicará el perfil correcto automáticamente.

## 📖 Comandos

### Gestión de Perfiles

| Comando | Descripción |
|---------|-------------|
| `gix profile add` | Agregar nuevo perfil |
| `gix profile list` | Listar todos los perfiles |
| `gix profile edit` | Editar un perfil existente |
| `gix profile delete` | Eliminar un perfil |

### Uso de Perfiles

| Comando | Descripción |
|---------|-------------|
| `gix use` | Seleccionar perfil para el repositorio actual |
| `gix use <nombre>` | Usar un perfil específico |
| `gix set` | Establecer perfil global por defecto |
| `gix status` | Ver el perfil activo en el repositorio |

### Comandos Git

gix funciona como un wrapper transparente de git:

```bash
gix push           # Push con el perfil correcto
gix pull           # Pull con el perfil correcto
gix fetch          # Fetch con el perfil correcto
gix commit -m "..."  # Commit con el usuario correcto
```

### Configuración y Sistema

| Comando | Descripción |
|---------|-------------|
| `gix commands` | Configurar qué comandos git interceptar |
| `gix version` | Mostrar versión instalada |
| `gix update` | Verificar e instalar actualizaciones |
| `gix doctor` | Ejecutar diagnósticos del sistema |

## ⚙️ Configuración

### Ubicación de Archivos

| Archivo | Ubicación | Descripción |
|---------|-----------|-------------|
| Config global | `~/.gix/config.json` | Perfiles y configuración general |
| Config local | `.gix/config.json` | Perfil seleccionado por repositorio |
| Log de uso | `~/.gix/usage.log` | Historial de comandos ejecutados |

### Ejemplo de config.json

```json
{
  "profiles": [
    {
      "profile_name": "Personal",
      "name": "Tu Nombre",
      "email": "tu@email.personal.com",
      "auth": {
        "SSH": {
          "key_path": "/Users/tu/.ssh/id_ed25519_personal"
        }
      }
    },
    {
      "profile_name": "Trabajo",
      "name": "Tu Nombre Trabajo",
      "email": "tu@empresa.com",
      "auth": {
        "SSH": {
          "key_path": "/Users/tu/.ssh/id_ed25519_trabajo"
        }
      }
    }
  ],
  "intercepted_commands": ["pull", "push", "fetch", "clone"]
}
```

### Comandos Interceptados

Por defecto, gix intercepta: `pull`, `push`, `fetch`, `clone`

Para cambiar esto:
```bash
gix commands
```

## 🔐 Seguridad

### Claves SSH

- gix verifica que las claves SSH tengan permisos seguros (600 o 400)
- Advertencias si los permisos son demasiado abiertos
- Soporte para claves con passphrase

### Tokens

- Los tokens se almacenan en `~/.gix/config.json`
- El archivo tiene permisos 600 (solo lectura/escritura por el propietario)
- Los tokens nunca se muestran en logs o salida

### Mejores Prácticas

1. **Usa claves SSH diferentes** para cada contexto (trabajo, personal)
2. **Protege tus claves** con passphrase
3. **Revisa permisos** regularmente con `gix doctor`

## 💡 Ejemplos de Uso

### Caso 1: Desarrollador con cuenta personal y de trabajo

```bash
# Agregar perfil personal
gix profile add
# Nombre: Personal
# Email: yo@gmail.com
# SSH: ~/.ssh/id_ed25519

# Agregar perfil de trabajo
gix profile add
# Nombre: Trabajo
# Email: yo@empresa.com
# SSH: ~/.ssh/id_ed25519_trabajo

# En un repo personal
cd ~/proyectos/mi-proyecto
gix use Personal

# En un repo de trabajo
cd ~/trabajo/proyecto-empresa
gix use Trabajo
```

### Caso 2: Contribuidor de Open Source

```bash
# Perfil para OS con email público
gix profile add
# Nombre: OpenSource
# Email: yo+oss@gmail.com
# SSH: ~/.ssh/id_ed25519_oss

cd ~/oss/proyecto-cool
gix use OpenSource
gix push  # Usa la identidad correcta
```

## ❓ FAQ

### ¿gix modifica mi configuración global de git?

No. gix solo modifica la configuración local del repositorio (`git config --local`) y usa variables de entorno para comandos individuales.

### ¿Puedo seguir usando git normalmente?

Sí. gix es un wrapper opcional. Puedes usar `git` directamente cuando quieras. La configuración local que aplica gix persistirá.

### ¿Qué pasa si no tengo un perfil configurado?

gix te preguntará qué perfil usar y opcionalmente lo guardará para el repositorio.

### ¿Cómo desinstalo gix?

```bash
# Con el script
curl -fsSL https://raw.githubusercontent.com/elmanci2/gix/refs/heads/master/install.sh | bash -s -- --uninstall

# Manualmente
rm ~/.local/bin/gix
rm -rf ~/.gix  # Opcional: eliminar configuración
```

### ¿gix funciona con GitHub, GitLab, Bitbucket?

Sí. gix es agnóstico del proveedor. Funciona con cualquier servidor git que soporte SSH o HTTPS.

## 🛠️ Desarrollo

### Requisitos

- Rust 1.70+
- Cargo

### Compilar

```bash
git clone https://github.com/elmanci2/gix.git
cd gix
cargo build --release
```

### Tests

```bash
cargo test
```

### Contribuir

1. Fork el repositorio
2. Crea una rama (`git checkout -b feature/nueva-caracteristica`)
3. Commit tus cambios (`git commit -am 'Agrega nueva característica'`)
4. Push a la rama (`git push origin feature/nueva-caracteristica`)
5. Abre un Pull Request

## 📄 Licencia

Este proyecto está bajo la Licencia MIT. Ver [LICENSE](LICENSE) para más detalles.

---

<div align="center">

**¿Encontraste un bug? ¿Tienes una idea?**

[Abre un Issue](https://github.com/elmanci2/gix/issues) | [Contribuye](https://github.com/elmanci2/gix/pulls)

Hecho con ❤️ por [elmanci2](https://github.com/elmanci2)

</div>
