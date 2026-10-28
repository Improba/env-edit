//! Linux SSH transport. Scripts are fixed; user supplied paths are shell-quoted.
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn valid_alias(alias: &str) -> bool {
    !alias.is_empty()
        && !alias.starts_with('-')
        && alias
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

pub fn valid_user(user: &str) -> bool {
    !user.is_empty()
        && !user.starts_with('-')
        && user
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

#[derive(Clone, Copy)]
pub struct Connection<'a> {
    pub alias: &'a str,
    pub user: Option<&'a str>,
    pub port: Option<u16>,
}

pub fn valid_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.contains('\0')
        && !path.contains('\n')
        && !path.contains('\r')
        && !path.contains('\\')
        && !path.split('/').any(|part| part == "..")
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn run(connection: Connection<'_>, script: &str, input: Option<&[u8]>) -> Result<Vec<u8>, String> {
    if !valid_alias(connection.alias)
        || connection.user.is_some_and(|user| !valid_user(user))
        || connection.port == Some(0)
    {
        return Err("Hôte, utilisateur ou port SSH invalide".into());
    }
    let mut command = Command::new("ssh");
    command.args([
        "-T",
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=yes",
        "-o",
        "ConnectTimeout=8",
    ]);
    if let Some(user) = connection.user {
        command.args(["-l", user]);
    }
    if let Some(port) = connection.port {
        command.args(["-p", &port.to_string()]);
    }
    let mut child = command
        .args(["--", connection.alias, &format!("sh -c {}", quote(script))])
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "Impossible de démarrer SSH".to_string())?;
    if let Some(data) = input {
        child
            .stdin
            .take()
            .ok_or("Entrée SSH indisponible")?
            .write_all(data)
            .map_err(|_| "Transfert SSH interrompu".to_string())?;
    }
    let output = child
        .wait_with_output()
        .map_err(|_| "Connexion SSH interrompue".to_string())?;
    if !output.status.success() {
        return Err(classify_failure(output.status.code(), &output.stderr));
    }
    Ok(output.stdout)
}

fn classify_failure(code: Option<i32>, stderr: &[u8]) -> String {
    match code {
        Some(42) => return "Conflit : le fichier distant a changé".into(),
        Some(43) => return "Dossier distant introuvable ou inaccessible".into(),
        Some(44) => return "Fichier distant introuvable, illisible ou lien symbolique".into(),
        Some(45) => {
            return "Sauvegarde refusée : le compte SSH n'est pas propriétaire du fichier".into()
        }
        Some(46) => {
            return "Sauvegarde refusée : le groupe du fichier ne pourrait pas être conservé".into()
        }
        _ => {}
    }
    // Classify only known SSH diagnostics; never return stderr verbatim.
    let diagnostic = String::from_utf8_lossy(stderr);
    if diagnostic.contains("Host key verification failed")
        || diagnostic.contains("No ") && diagnostic.contains("host key is known")
    {
        "Clé d'hôte non vérifiée : établissez d'abord la connexion SSH depuis un terminal".into()
    } else if diagnostic.contains("Permission denied") {
        "Authentification SSH refusée : vérifiez l'utilisateur et la clé dans SSHPilot ou ~/.ssh/config".into()
    } else if diagnostic.contains("Could not resolve hostname") {
        "Hôte SSH introuvable : vérifiez l'adresse ou l'alias".into()
    } else if diagnostic.contains("Connection timed out")
        || diagnostic.contains("Connection refused")
    {
        "Serveur SSH inaccessible : vérifiez l'adresse, le port et le réseau".into()
    } else {
        "Opération SSH refusée : vérifiez la connexion, le dossier et les permissions du serveur"
            .into()
    }
}

pub fn list(connection: Connection<'_>, directory: &str) -> Result<Vec<String>, String> {
    if !valid_path(directory) {
        return Err("Dossier distant invalide".into());
    }
    let script = format!("d={}; test -d \"$d\" || exit 43; find \"$d\" -maxdepth 1 -type f -name '.env*' -printf '%f\\0'", quote(directory));
    let output = run(connection, &script, None)?;
    let mut names = output
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(|name| {
            String::from_utf8(name.to_vec()).map_err(|_| "Nom distant non UTF-8".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    names.retain(|name| name == ".env" || name.starts_with(".env."));
    names.sort();
    Ok(names)
}

pub struct RemoteFile {
    pub data: Vec<u8>,
    pub fingerprint: String,
    pub mode: String,
}

pub fn read(connection: Connection<'_>, path: &str) -> Result<RemoteFile, String> {
    if !valid_path(path)
        || !Path::new(path).file_name().is_some_and(|n| {
            n.to_str()
                .is_some_and(|n| n == ".env" || n.starts_with(".env."))
        })
    {
        return Err("Fichier distant invalide".into());
    }
    let script = format!("p={}; test -f \"$p\" && test ! -L \"$p\" || exit 44; stat -c '%a' -- \"$p\"; sha256sum -- \"$p\" | cut -d ' ' -f 1; cat -- \"$p\"", quote(path));
    let output = run(connection, &script, None)?;
    let mut parts = output.splitn(3, |byte| *byte == b'\n');
    let mode = String::from_utf8(parts.next().unwrap_or_default().to_vec())
        .map_err(|_| "Métadonnées SSH invalides".to_string())?;
    let fingerprint = String::from_utf8(parts.next().unwrap_or_default().to_vec())
        .map_err(|_| "Métadonnées SSH invalides".to_string())?;
    let data = parts.next().ok_or("Réponse SSH incomplète")?.to_vec();
    if !mode.bytes().all(|b| (b'0'..=b'7').contains(&b))
        || mode.is_empty()
        || fingerprint.len() != 64
        || !fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Métadonnées SSH invalides".into());
    }
    Ok(RemoteFile {
        data,
        fingerprint,
        mode,
    })
}

pub fn save(
    connection: Connection<'_>,
    path: &str,
    expected: &str,
    mode: &str,
    data: &[u8],
) -> Result<(), String> {
    if !valid_path(path)
        || expected.len() != 64
        || !expected.bytes().all(|b| b.is_ascii_hexdigit())
        || mode.is_empty()
        || !mode.bytes().all(|b| (b'0'..=b'7').contains(&b))
    {
        return Err("Paramètres de sauvegarde SSH invalides".into());
    }
    let script = save_script(path, expected, mode);
    // The remote script only reads payload on stdin; it never echoes secret values.
    run(connection, &script, Some(data)).map(|_| ())
}

fn save_script(path: &str, expected: &str, mode: &str) -> String {
    format!(
        r#"p={path}; expected={expected}; mode={mode};
test -f "$p" && test ! -L "$p" || exit 44
test "$(stat -c '%u' -- "$p")" = "$(id -u)" || exit 45
current=$(sha256sum -- "$p" | cut -d ' ' -f 1)
test "$current" = "$expected" || exit 42
d=$(dirname -- "$p"); n=$(basename -- "$p")
tmp=$(mktemp "$d/.$n.envedit.XXXXXXXX") || exit 1
trap 'rm -f -- "$tmp"' EXIT HUP INT TERM
chmod "$mode" "$tmp" || exit 1
test "$(stat -c '%g' -- "$tmp")" = "$(stat -c '%g' -- "$p")" || exit 46
cat > "$tmp" || exit 1
test "$(sha256sum -- "$p" | cut -d ' ' -f 1)" = "$expected" || exit 42
backup="$d/.$n.envedit-backup-$(date +%Y%m%d%H%M%S)-$$"
umask 077
cp -- "$p" "$backup" || exit 1
mv -f -- "$tmp" "$p" || exit 1
trap - EXIT HUP INT TERM
"#,
        path = quote(path),
        expected = quote(expected),
        mode = quote(mode)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "linux")]
    use sha2::{Digest, Sha256};
    #[cfg(target_os = "linux")]
    use std::fs;
    #[cfg(target_os = "linux")]
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn rejects_unsafe_targets() {
        assert!(valid_alias("prod-bastion"));
        assert!(valid_user("deploy"));
        assert!(!valid_user("-oProxyCommand=evil"));
        assert!(!valid_user("deploy;touch /tmp/x"));
        assert!(!valid_alias("-oProxyCommand=evil"));
        assert!(!valid_alias("host;touch /tmp/x"));
        assert!(valid_path("/var/app/.env"));
        assert!(!valid_path("/var/../root/.env"));
        assert!(!valid_path("relative/.env"));
    }

    #[test]
    fn ssh_errors_are_classified_without_returning_remote_output() {
        assert_eq!(
            classify_failure(Some(43), b"private-value"),
            "Dossier distant introuvable ou inaccessible"
        );
        assert_eq!(classify_failure(Some(255), b"Permission denied (publickey). private-value"),
            "Authentification SSH refusée : vérifiez l'utilisateur et la clé dans SSHPilot ou ~/.ssh/config");
        assert_eq!(
            classify_failure(Some(255), b"Host key verification failed. private-value"),
            "Clé d'hôte non vérifiée : établissez d'abord la connexion SSH depuis un terminal"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn remote_save_script_preserves_original_on_conflict_and_keeps_backup() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("envedit-ssh-script-{nonce}"));
        fs::create_dir(&directory).unwrap();
        let path = directory.join(".env");
        fs::write(&path, b"VALUE=fixture\n").unwrap();
        let expected = format!("{:x}", Sha256::digest(b"VALUE=fixture\n"));
        let script = save_script(path.to_str().unwrap(), &expected, "600");
        let mut child = Command::new("sh")
            .args(["-c", &script])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"VALUE=updated\n")
            .unwrap();
        assert!(child.wait().unwrap().success());
        assert_eq!(fs::read(&path).unwrap(), b"VALUE=updated\n");
        let backups = fs::read_dir(&directory)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path != &directory.join(".env"))
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(&backups[0]).unwrap(), b"VALUE=fixture\n");
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&backups[0]).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let status = Command::new("sh")
            .args(["-c", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(42));
        assert_eq!(fs::read(&path).unwrap(), b"VALUE=updated\n");
        fs::remove_dir_all(directory).unwrap();
    }
}
