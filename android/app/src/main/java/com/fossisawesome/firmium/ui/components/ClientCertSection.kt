package com.fossisawesome.firmium.ui.components

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import android.net.Uri
import android.widget.Toast
import androidx.compose.ui.platform.LocalContext
import com.fossisawesome.firmium.data.net.TlsConfig
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

// Client certificate (mTLS) setup, for servers behind a proxy that demands one. Shown in the
// login dialog (needed before the first connect) and in Settings > Account. The identity is a
// PKCS#12 file imported into the Android Keystore; the CA is an optional PEM. See TlsConfig.
@Composable
fun ClientCertSection(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val colors = LocalFirmiumColors.current

    var hasIdentity by remember { mutableStateOf(TlsConfig.hasIdentity()) }
    var hasCa by remember { mutableStateOf(TlsConfig.hasCa()) }
    var pendingP12 by remember { mutableStateOf<ByteArray?>(null) }

    suspend fun readBytes(uri: Uri): ByteArray? = withContext(Dispatchers.IO) {
        runCatching { context.contentResolver.openInputStream(uri)?.use { it.readBytes() } }.getOrNull()
    }

    val identityPicker = rememberLauncherForActivityResult(ActivityResultContracts.GetContent()) { uri ->
        if (uri != null) scope.launch { pendingP12 = readBytes(uri) ?: run { Toast.makeText(context, "Could not read file", Toast.LENGTH_SHORT).show(); null } }
    }
    val caPicker = rememberLauncherForActivityResult(ActivityResultContracts.GetContent()) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            val bytes = readBytes(uri)
            val result = withContext(Dispatchers.IO) { runCatching { TlsConfig.importCa(bytes ?: error("Could not read file")) } }
            hasCa = TlsConfig.hasCa()
            Toast.makeText(context, result.fold({ "CA certificate installed" }, { it.message ?: "Import failed" }), Toast.LENGTH_SHORT).show()
        }
    }

    Column(modifier = modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text("Client certificate (mTLS)", fontSize = 13.sp, fontWeight = FontWeight.Bold,
            fontFamily = LocalAppFontFamily.current, color = colors.text)
        Text("For servers that require one. Add it before connecting.", fontSize = 11.sp,
            fontFamily = LocalAppFontFamily.current, color = colors.muted)
        CertRow("Identity (.p12 / .pfx)", hasIdentity,
            onAdd = { identityPicker.launch("*/*") },
            onRemove = { TlsConfig.clearIdentity(); hasIdentity = false })
        CertRow("CA certificate (PEM)", hasCa,
            onAdd = { caPicker.launch("*/*") },
            onRemove = { TlsConfig.clearCa(); hasCa = false })
    }

    pendingP12?.let { bytes ->
        PasswordDialog(
            onDismiss = { pendingP12 = null },
            onConfirm = { password ->
                val result = withContext(Dispatchers.IO) { runCatching { TlsConfig.importIdentity(bytes, password.toCharArray()) } }
                result.exceptionOrNull()?.let { it.message ?: "Import failed" } ?: run {
                    hasIdentity = TlsConfig.hasIdentity()
                    pendingP12 = null
                    Toast.makeText(context, "Certificate installed", Toast.LENGTH_SHORT).show()
                    null
                }
            },
        )
    }
}

@Composable
private fun CertRow(label: String, installed: Boolean, onAdd: () -> Unit, onRemove: () -> Unit) {
    val colors = LocalFirmiumColors.current
    Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Column(modifier = Modifier.weight(1f)) {
            Text(label, fontSize = 13.sp, fontFamily = LocalAppFontFamily.current, color = colors.text)
            Text(if (installed) "Installed" else "Not set", fontSize = 11.sp,
                fontFamily = LocalAppFontFamily.current, color = colors.muted)
        }
        Text(
            if (installed) "Remove" else "Choose file…", fontSize = 13.sp, fontFamily = LocalAppFontFamily.current,
            color = if (installed) colors.error else colors.accent,
            modifier = Modifier.clip(RoundedCornerShape(4.dp)).clickable { if (installed) onRemove() else onAdd() }.padding(horizontal = 12.dp, vertical = 8.dp),
        )
    }
}

// onConfirm returns null on success (dialog closes itself via the caller) or an error to show.
@Composable
private fun PasswordDialog(onDismiss: () -> Unit, onConfirm: suspend (String) -> String?) {
    val colors = LocalFirmiumColors.current
    val scope = rememberCoroutineScope()
    var password by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }

    Dialog(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier.fillMaxWidth()
                .clip(RoundedCornerShape(8.dp))
                .background(colors.surface)
                .border(1.dp, colors.border, RoundedCornerShape(8.dp))
                .padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text("Certificate password", fontSize = 16.sp, fontWeight = FontWeight.Bold,
                fontFamily = LocalAppFontFamily.current, color = colors.text)
            Text("Used once to unlock the file. Firmium stores the key in the Android Keystore, not the password.",
                fontSize = 12.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted)
            FirmiumTextField(
                value = password,
                onValueChange = { password = it; error = null },
                placeholder = "Password (blank if none)",
                visualTransformation = PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                modifier = Modifier.fillMaxWidth(),
            )
            error?.let { Text(it, fontSize = 12.sp, fontFamily = LocalAppFontFamily.current, color = colors.error) }
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp, Alignment.End)) {
                Text("Cancel", fontSize = 13.sp, fontFamily = LocalAppFontFamily.current, color = colors.muted,
                    modifier = Modifier.clickable { onDismiss() }.padding(8.dp))
                Text("Import", fontSize = 13.sp, fontFamily = LocalAppFontFamily.current,
                    color = if (busy) colors.muted else colors.accent,
                    modifier = Modifier.clickable(enabled = !busy) {
                        busy = true
                        scope.launch { error = onConfirm(password); busy = false }
                    }.padding(8.dp))
            }
        }
    }
}
