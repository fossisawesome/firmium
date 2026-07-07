package com.fossisawesome.firmium.ui.tv

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.fossisawesome.firmium.ui.components.FirmiumTextField
import com.fossisawesome.firmium.ui.components.Text
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors

// Minimal single-form TV login — no multi-server switcher or QR flow (Phase 1 scope).
@Composable
fun TvLoginScreen(
    error: String?,
    onLogin: (server: String, username: String, password: String) -> Unit,
) {
    val colors = LocalFirmiumColors.current
    var server by remember { mutableStateOf("") }
    var username by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }

    Column(
        modifier = Modifier.fillMaxSize().padding(48.dp),
        horizontalAlignment = Alignment.Start,
    ) {
        Text(text = "Sign in to Firmium", color = colors.text, fontSize = 24.sp, modifier = Modifier.padding(bottom = 24.dp))

        FirmiumTextField(
            value = server,
            onValueChange = { server = it },
            label = "Server URL",
            placeholder = "https://music.example.com",
            modifier = Modifier.width(420.dp).padding(bottom = 16.dp),
        )
        FirmiumTextField(
            value = username,
            onValueChange = { username = it },
            label = "Username",
            modifier = Modifier.width(420.dp).padding(bottom = 16.dp),
        )
        FirmiumTextField(
            value = password,
            onValueChange = { password = it },
            label = "Password",
            visualTransformation = PasswordVisualTransformation(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
            modifier = Modifier.width(420.dp).padding(bottom = 24.dp),
        )

        if (error != null) {
            Text(text = error, color = colors.error, fontSize = 13.sp, modifier = Modifier.padding(bottom = 16.dp))
        }

        TvActionButton(onClick = { onLogin(server, username, password) }, colors = colors) {
            Text(text = "Log In", color = colors.text, fontSize = 14.sp)
        }
    }
}
