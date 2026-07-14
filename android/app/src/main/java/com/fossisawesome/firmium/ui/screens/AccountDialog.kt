package com.fossisawesome.firmium.ui.screens
import com.fossisawesome.firmium.ui.theme.LocalAppFontFamily

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.filled.VisibilityOff
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import com.fossisawesome.firmium.data.api.AuthManager
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.AuthState
import java.net.URI

// Account popup: connect form when logged out, server info + disconnect when logged in.
// Mirrors AccountModal.svelte (desktop) — opened from the account icon in the page header / nav rail.
@Composable
fun AccountDialog(
    state: AuthState,
    isAuthenticated: Boolean,
    serverUrl: String?,
    onLogin: (server: String, username: String, password: String, savePassword: Boolean) -> Unit,
    onSwitchServer: (url: String, username: String) -> Unit,
    onRemoveServer: (url: String, username: String) -> Unit,
    onDisconnect: () -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = LocalFirmiumColors.current

    Dialog(onDismissRequest = onDismiss) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(4.dp))
                .background(colors.surface)
                .border(1.dp, colors.border, RoundedCornerShape(4.dp)),
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(32.dp)
                    .verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(0.dp),
            ) {
                if (isAuthenticated) {
                    val hostname = remember(serverUrl) {
                        try { URI(serverUrl ?: "").host ?: serverUrl ?: "" } catch (_: Exception) { serverUrl ?: "" }
                    }
                    Text(
                        text = "Connected",
                        fontSize = 14.sp,
                        fontWeight = FontWeight.Bold,
                        fontFamily = LocalAppFontFamily.current,
                        color = colors.text,
                        modifier = Modifier.padding(bottom = 4.dp),
                    )
                    Text(
                        text = hostname,
                        fontSize = 12.sp,
                        fontFamily = LocalAppFontFamily.current,
                        color = colors.muted,
                        modifier = Modifier.padding(bottom = 20.dp),
                    )
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(2.dp))
                            .border(1.dp, colors.error, RoundedCornerShape(2.dp))
                            .clickable(
                                interactionSource = remember { MutableInteractionSource() },
                                indication = null,
                            ) { onDisconnect() }
                            .padding(vertical = 12.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            text = "Disconnect",
                            fontSize = 12.sp,
                            fontWeight = FontWeight.Bold,
                            fontFamily = LocalAppFontFamily.current,
                            color = colors.error,
                            letterSpacing = 0.5.sp,
                        )
                    }
                } else {
                    if (state.savedServers.isNotEmpty()) {
                        SavedServersList(
                            servers = state.savedServers,
                            isLoading = state.isLoading,
                            onConnect = onSwitchServer,
                            onRemove = onRemoveServer,
                        )
                    }
                    AccountConnectForm(state = state, onLogin = onLogin)
                }
            }

            FirmiumIconButton(
                onClick = onDismiss,
                modifier = Modifier.align(Alignment.TopEnd).padding(8.dp).size(32.dp),
            ) {
                FirmiumIcon(Icons.Default.Close, contentDescription = "Close", tint = colors.muted, modifier = Modifier.size(16.dp))
            }
        }
    }
}

@Composable
private fun AccountConnectForm(
    state: AuthState,
    onLogin: (server: String, username: String, password: String, savePassword: Boolean) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var server by remember { mutableStateOf("") }
    var username by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var showPassword by remember { mutableStateOf(false) }
    var savePassword by remember { mutableStateOf(true) }

    // Pre-fill fields once the auth state loads with persisted values.
    LaunchedEffect(state.savedServer, state.savedUsername) {
        if (server.isEmpty() && state.savedServer.isNotEmpty()) server = state.savedServer
        if (username.isEmpty() && state.savedUsername.isNotEmpty()) username = state.savedUsername
        savePassword = state.savePassword
    }

    val userFocus = remember { FocusRequester() }
    val passFocus = remember { FocusRequester() }

    val canSubmit = server.isNotBlank() && username.isNotBlank() && password.isNotBlank() && !state.isLoading

    val cleartextWarning = remember(server) {
        try {
            val uri = URI(server.trim())
            val host = uri.host ?: ""
            val isLocal = host == "localhost" || host == "127.0.0.1" || host == "::1" ||
                host.matches(Regex("^10\\..*|^192\\.168\\..*|^172\\.(1[6-9]|2\\d|3[01])\\..*|\\.local$"))
            if (uri.scheme == "http" && !isLocal) {
                "Connecting over plain HTTP to a non-local server sends your credentials unencrypted. Use HTTPS if possible."
            } else null
        } catch (_: Exception) { null }
    }

    SetupField(
        value = server,
        onValueChange = { server = it },
        placeholder = "https://navidrome.example.com",
        imeAction = ImeAction.Next,
        onImeAction = { userFocus.requestFocus() },
    )
    if (cleartextWarning != null) {
        Text(
            text = cleartextWarning,
            color = colors.muted,
            fontSize = 11.sp,
            fontFamily = LocalAppFontFamily.current,
            modifier = Modifier.padding(bottom = 16.dp),
        )
    }

    SetupField(
        value = username,
        onValueChange = { username = it },
        imeAction = ImeAction.Next,
        onImeAction = { passFocus.requestFocus() },
        focusRequester = userFocus,
    )

    // Password field — intentionally taller than the other inputs.
    Column(modifier = Modifier.padding(bottom = 16.dp)) {
        FirmiumTextField(
            value = password,
            onValueChange = { password = it },
            visualTransformation = if (showPassword) VisualTransformation.None else PasswordVisualTransformation(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { if (canSubmit) onLogin(server.trim(), username.trim(), password, savePassword) }),
            trailingIcon = {
                FirmiumIconButton(
                    onClick = { showPassword = !showPassword },
                    modifier = Modifier.size(44.dp),
                ) {
                    FirmiumIcon(
                        if (showPassword) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                        contentDescription = if (showPassword) "Hide password" else "Show password",
                        tint = colors.muted,
                        modifier = Modifier.size(18.dp),
                    )
                }
            },
            modifier = Modifier.fillMaxWidth().heightIn(min = 64.dp).focusRequester(passFocus),
        )
    }

    // Save password toggle
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(bottom = 20.dp)
            .clickable(
                interactionSource = remember { MutableInteractionSource() },
                indication = null,
            ) { savePassword = !savePassword },
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(
            "Save password",
            fontSize = 12.sp,
            fontFamily = LocalAppFontFamily.current,
            color = colors.muted,
        )
        FirmiumToggle(checked = savePassword, onCheckedChange = { savePassword = it })
    }

    if (state.error != null) {
        Text(
            text = state.error,
            color = colors.error,
            fontSize = 12.sp,
            fontFamily = LocalAppFontFamily.current,
            modifier = Modifier.padding(bottom = 12.dp),
        )
    }

    // Connect button
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(2.dp))
            .background(if (canSubmit) colors.accent else colors.accent.copy(alpha = 0.4f))
            .then(
                if (canSubmit) Modifier.clickable(
                    interactionSource = remember { MutableInteractionSource() },
                    indication = null,
                ) { onLogin(server.trim(), username.trim(), password, savePassword) }
                else Modifier
            )
            .padding(vertical = 12.dp),
        contentAlignment = Alignment.Center,
    ) {
        if (state.isLoading) {
            FirmiumSpinner(color = Color.Black, modifier = Modifier.size(18.dp), strokeWidth = 2.dp)
        } else {
            Text(
                text = "Connect",
                fontSize = 12.sp,
                fontWeight = FontWeight.Bold,
                fontFamily = LocalAppFontFamily.current,
                color = Color.Black,
                letterSpacing = 0.5.sp,
            )
        }
    }
}

// One labeled input field matching .field + .field label + .field input
@Composable
private fun SetupField(
    label: String = "",
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String = "",
    imeAction: ImeAction = ImeAction.Next,
    onImeAction: (() -> Unit)? = null,
    focusRequester: FocusRequester? = null,
) {
    val colors = LocalFirmiumColors.current
    Column(modifier = Modifier.padding(bottom = 20.dp)) {
        if (label.isNotEmpty()) {
            Text(
                label.uppercase(),
                fontSize = 11.sp, fontFamily = LocalAppFontFamily.current,
                color = colors.muted, letterSpacing = 0.5.sp,
                modifier = Modifier.padding(bottom = 6.dp),
            )
        }
        FirmiumTextField(
            value = value,
            onValueChange = onValueChange,
            placeholder = placeholder,
            keyboardOptions = KeyboardOptions(imeAction = imeAction),
            keyboardActions = KeyboardActions(onNext = { onImeAction?.invoke() }, onDone = { onImeAction?.invoke() }),
            modifier = if (focusRequester != null) Modifier.fillMaxWidth().focusRequester(focusRequester)
                       else Modifier.fillMaxWidth(),
        )
    }
}

@Composable
private fun SavedServersList(
    servers: List<AuthManager.SavedServer>,
    isLoading: Boolean,
    onConnect: (url: String, username: String) -> Unit,
    onRemove: (url: String, username: String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    Column(modifier = Modifier.padding(bottom = 16.dp)) {
        Text(
            "Saved Servers".uppercase(),
            fontSize = 11.sp, fontFamily = LocalAppFontFamily.current,
            color = colors.muted, letterSpacing = 0.5.sp,
            modifier = Modifier.padding(bottom = 8.dp),
        )
        servers.forEach { server ->
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(bottom = 6.dp)
                    .clip(RoundedCornerShape(2.dp))
                    .background(colors.bg)
                    .border(1.dp, colors.border, RoundedCornerShape(2.dp))
                    .padding(horizontal = 10.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        server.url,
                        fontSize = 12.sp, fontFamily = LocalAppFontFamily.current,
                        color = colors.text, maxLines = 1,
                    )
                    Text(
                        server.username,
                        fontSize = 11.sp, fontFamily = LocalAppFontFamily.current,
                        color = colors.muted,
                    )
                }
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(2.dp))
                        .background(colors.accent)
                        .clickable(enabled = !isLoading) { onConnect(server.url, server.username) }
                        .padding(horizontal = 10.dp, vertical = 4.dp),
                ) {
                    Text(
                        "Connect",
                        fontSize = 11.sp, fontWeight = FontWeight.Bold,
                        fontFamily = LocalAppFontFamily.current, color = Color.Black,
                    )
                }
                Spacer(Modifier.width(6.dp))
                FirmiumIconButton(
                    onClick = { onRemove(server.url, server.username) },
                    modifier = Modifier.size(28.dp),
                ) {
                    FirmiumIcon(Icons.Default.Close, contentDescription = "Remove", tint = colors.muted, modifier = Modifier.size(14.dp))
                }
            }
        }
        Text(
            "or add a new server",
            fontSize = 11.sp, fontFamily = LocalAppFontFamily.current,
            color = colors.muted, letterSpacing = 0.5.sp,
            modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
            textAlign = androidx.compose.ui.text.style.TextAlign.Center,
        )
    }
}
