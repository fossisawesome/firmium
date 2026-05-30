package com.fossisawesome.firmium.ui.screens

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
import com.fossisawesome.firmium.ui.components.*
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.viewmodel.AuthState

@Composable
fun LoginScreen(
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

    Box(
        modifier = Modifier.fillMaxSize().background(colors.bg).windowInsetsPadding(WindowInsets.systemBars)
            .padding(24.dp),
        contentAlignment = Alignment.Center,
    ) {
        Column(
            modifier = Modifier
                .widthIn(max = 400.dp)
                .fillMaxWidth()
                .clip(RoundedCornerShape(4.dp))
                .background(colors.surface)
                .border(1.dp, colors.border, RoundedCornerShape(4.dp))
                .padding(32.dp)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(0.dp),
        ) {
            // "Firmium" title
            Text(
                text = "Firmium",
                fontSize = 20.sp,
                fontWeight = FontWeight.Normal,
                fontFamily = FontFamily.Monospace,
                color = colors.accent,
                letterSpacing = (-0.5).sp,
                modifier = Modifier.padding(bottom = 28.dp),
            )

            SetupField(
                label = "Server URL",
                value = server,
                onValueChange = { server = it },
                placeholder = "https://navidrome.example.com",
                imeAction = ImeAction.Next,
                onImeAction = { userFocus.requestFocus() },
            )

            SetupField(
                label = "Username",
                value = username,
                onValueChange = { username = it },
                imeAction = ImeAction.Next,
                onImeAction = { passFocus.requestFocus() },
                focusRequester = userFocus,
            )

            // Password field — intentionally taller than the other inputs.
            Column(modifier = Modifier.padding(bottom = 16.dp)) {
                Text(
                    "Password".uppercase(),
                    fontSize = 11.sp, fontFamily = FontFamily.Monospace,
                    color = colors.muted, letterSpacing = 0.5.sp,
                    modifier = Modifier.padding(bottom = 6.dp),
                )
                FirmiumTextField(
                    value = password,
                    onValueChange = { password = it },
                    visualTransformation = if (showPassword) VisualTransformation.None else PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { if (canSubmit) onLogin(server.trim(), username.trim(), password, savePassword) }),
                    trailingIcon = {
                        FirmiumIconButton(
                            onClick = { showPassword = !showPassword },
                            modifier = Modifier.size(36.dp),
                        ) {
                            FirmiumIcon(
                                if (showPassword) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                                contentDescription = null,
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
                    fontFamily = FontFamily.Monospace,
                    color = colors.muted,
                )
                FirmiumToggle(checked = savePassword, onCheckedChange = { savePassword = it })
            }

            if (state.error != null) {
                Text(
                    text = state.error,
                    color = colors.error,
                    fontSize = 12.sp,
                    fontFamily = FontFamily.Monospace,
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
                        fontFamily = FontFamily.Monospace,
                        color = Color.Black,
                        letterSpacing = 0.5.sp,
                    )
                }
            }
        }
    }
}

// One labeled input field matching .field + .field label + .field input
@Composable
private fun SetupField(
    label: String,
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String = "",
    imeAction: ImeAction = ImeAction.Next,
    onImeAction: (() -> Unit)? = null,
    focusRequester: FocusRequester? = null,
) {
    val colors = LocalFirmiumColors.current
    Column(modifier = Modifier.padding(bottom = 20.dp)) {
        Text(
            label.uppercase(),
            fontSize = 11.sp, fontFamily = FontFamily.Monospace,
            color = colors.muted, letterSpacing = 0.5.sp,
            modifier = Modifier.padding(bottom = 6.dp),
        )
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
